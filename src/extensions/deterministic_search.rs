// Temporary hack: Allow dead deferred code warning in release build
#![cfg_attr(not(debug_assertions), allow(dead_code))]
#![cfg_attr(not(debug_assertions), allow(unused_imports))]

use std::{
    fmt::Debug,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use rayon::{
    iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use rootcause::Report;
use scc::{Guard, HashMap};
use tokio::sync::watch::channel;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    apps::{App, AppName, app_string::AppString, app_substr::AppSubstr},
    extensions::{DeferredReceiver, DeferredSender, DeferredToken, SearchEngine},
    fs::{
        apps::{AppList, apps},
        config::Configuration,
        db::{AppPersistence, FilesystemPersistence},
    },
};

#[derive(Debug, Clone)]
pub struct DeterministicSearchEngine {
    db: Arc<Mutex<FilesystemPersistence>>,
    config: Configuration,
    apps: Arc<Mutex<AppList>>,
    learned_substring_index: Arc<HashMap<AppString, App>>,
    substring_index: Arc<HashMap<AppString, Vec<AppName>>>,

    /// Keeps track of the latest search query.
    /// The higher that number is, the more recent
    /// the query is.
    /// Used by receivers to detect when a new
    /// search was started on a different thread,
    /// in which case old results (with smaller tokens)
    /// should be discarded
    deferred_token: Arc<AtomicUsize>,
    deferred_watcher: DeferredSender,

    /// Every query the user has entered when searching
    /// for an app. For instance, if the user launches Fetch, and opens
    /// Firefox by having search "Fire", then the vector will contain the
    /// following: `["F", "Fi", "Fir", "Fire"]`
    query_history: scc::Stack<AppString>,
}

impl SearchEngine for DeterministicSearchEngine {
    fn blocking_search(&self, query: AppString) -> Vec<App> {
        self.query_history.push(query.clone());

        let mut filtered_apps: Vec<App> = self.apps.lock().expect("no poison").to_vec();

        filtered_apps.par_sort_by_cached_key(|app| app.name.clone());

        filtered_apps = filtered_apps
            .into_par_iter()
            .filter(|app| self.is_query_substring_of_app_name(&query, &app.name))
            .collect();

        filtered_apps.par_sort_by_cached_key(|app| {
            if query == app.name {
                (0, 0)
            } else {
                let (dist_name, dist_substring) =
                    beginning_distance(&query.substring(0, query.len()), &app.name);

                (
                    dist_name.overflowing_neg().0,
                    dist_substring.overflowing_neg().0,
                )
            }
        });

        filtered_apps.par_sort_by_key(|app| {
            i32::from(
                self.learned_substring_index
                    .get_sync(&query)
                    .is_none_or(|s| s.get().name != app.name),
            )
        });

        filtered_apps
    }

    fn deferred_search(&self, query: AppString) -> (DeferredToken, DeferredReceiver) {
        let tx = self.deferred_watcher.clone();
        let rx = tx.subscribe();
        let token = self.deferred_token.fetch_add(1, Ordering::Acquire);
        let res = self.blocking_search(query);
        tx.send_replace((token, res));
        (token, rx)
    }

    fn after_search(&self, opened_app: Option<App>) {
        let query_history = self.query_history.pop_all();

        if let Some(app) = opened_app {
            {
                let guard = Guard::new();
                query_history.iter(&guard).for_each(|query| {
                    let _ = self
                        .learned_substring_index
                        .upsert_sync(query.clone(), app.clone());
                });
            }

            self.db
                .lock()
                .expect("no lock poisoning")
                .save_data(
                    "learned_substring_index",
                    self.learned_substring_index.clone(),
                )
                .expect("json map is expected to function");
        }

        self.deferred_token.store(0, Ordering::Release);
        // Check for modified apps, update if needed.
        let applist = self.apps.clone();

        let new_apps = apps(&self.config);
        let mut current_apps = applist.lock().expect("no lock poisoning");
        if new_apps.ne(&current_apps) {
            let _ = std::mem::replace(&mut *current_apps, new_apps);
        }
        drop(current_apps);
        self.index_apps();
    }
}

impl DeterministicSearchEngine {
    pub fn build(config: &Configuration) -> Result<Self, Report> {
        let db = FilesystemPersistence::open()?;
        let apps: AppList = apps(config);
        let substring_index = Arc::new(scc::HashMap::new());

        let learned_substring_index =
            Arc::new(db.get_data("learned_substring_index").unwrap_or_default());

        let (tx, _rx) = channel((0, vec![]));
        let engine = Self {
            db: Arc::new(Mutex::new(db)),
            config: config.clone(),
            apps: Arc::new(Mutex::new(apps)),
            learned_substring_index,
            substring_index,
            deferred_token: Arc::new(AtomicUsize::new(0)),
            deferred_watcher: tx,
            query_history: scc::Stack::new(),
        };

        engine.index_apps();

        Ok(engine)
    }

    #[inline]
    fn index_apps(&self) {
        self.apps
            .lock()
            .expect("no lock poisoning")
            .par_iter()
            .for_each(|app| {
                for n in 0..=app.name.grapheme_len() {
                    let substrings = substrings(&app.name, n);
                    for substr in substrings {
                        self.substring_index
                            .entry_sync(substr.into())
                            .or_default()
                            .push(app.name.clone());
                    }
                }
            });
    }

    #[inline]
    fn is_query_substring_of_app_name(&self, query: &AppString, app_name: &AppName) -> bool {
        let Some(res) = self.substring_index.get_sync(query) else {
            return false;
        };

        res.contains(app_name)
    }
}

#[inline]
#[must_use]
pub fn substrings(string: &str, n: usize) -> Vec<String> {
    let graphemes = UnicodeSegmentation::graphemes(string, true).collect::<Vec<&str>>();
    if n > graphemes.len() {
        return Vec::new();
    }

    let mut vec = vec![];
    for i in 0..=(string.len() - n) {
        // TODO: Slow, can probably use pointers + graphemes here to get valid UTF-8 memory range
        let substr_vec = graphemes.get(i..i + n).expect("within range").to_vec();

        if !substr_vec.is_empty() {
            let substr = substr_vec.join("");
            vec.push(substr);
        }
    }

    vec
}

/// Substring distance from a space and/or beginning of app name
/// Users are expected to search starting from the beginning of app name
/// (For instance: "Ad" or "Ph" for "Adobe Photoshop")
#[inline]
fn beginning_distance(substr: &AppSubstr, name: &AppString) -> (usize, usize) {
    for (i, word) in name.split_ascii_whitespace().enumerate() {
        let word_appstr = AppString::from(word);
        for j in 0..word_appstr.len().saturating_sub(substr.len()) {
            let name_substr = word_appstr.substring(j, substr.len());
            if substr == &name_substr {
                return (i, j);
            }
        }
    }

    (0, name.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substrings() {
        assert_eq!(substrings("abc", 0), Vec::<String>::new());
        assert_eq!(substrings("abc", 1), vec!["a", "b", "c"]);
        assert_eq!(substrings("abc", 2), vec!["ab", "bc"]);
        assert_eq!(substrings("abc", 3), vec!["abc"]);
        assert_eq!(substrings("abc", 4), Vec::<String>::new());

        assert_eq!(
            substrings("Firefox", 3),
            vec!["Fir", "ire", "ref", "efo", "fox"]
        );
    }

    #[test]
    fn test_substring_beginning_distance() {
        let test_app_name: AppString = "Adobe Photoshop".into();
        assert_eq!(beginning_distance(&"Ado".into(), &test_app_name), (0, 0));
        assert_eq!(beginning_distance(&"ado".into(), &test_app_name), (0, 0));
        assert_eq!(beginning_distance(&"Pho".into(), &test_app_name), (1, 0));
        assert_eq!(beginning_distance(&"pho".into(), &test_app_name), (1, 0));
        assert_eq!(beginning_distance(&"dob".into(), &test_app_name), (0, 1));
        assert_eq!(beginning_distance(&"hot".into(), &test_app_name), (1, 1));
        assert_eq!(beginning_distance(&"oto".into(), &test_app_name), (1, 2));
    }
}
