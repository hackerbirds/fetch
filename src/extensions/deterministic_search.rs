// Temporary hack: Allow dead deferred code warning in release build
#![cfg_attr(not(debug_assertions), allow(dead_code))]
#![cfg_attr(not(debug_assertions), allow(unused_imports))]

use std::{
    fmt::Debug,
    sync::atomic::{AtomicUsize, Ordering},
};

use rayon::{
    iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use scc::HashMap;
use tokio::sync::watch::channel;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    apps::{App, AppName, app_string::AppString, app_substr::AppSubstr},
    extensions::{DeferredReceiver, DeferredSender, DeferredToken, SearchEngine},
    fs::{
        apps::{AppList, apps},
        db::{AppPersistence, FilesystemPersistence},
    },
};

#[derive(Debug)]
pub struct DeterministicSearchEngine {
    db: FilesystemPersistence,
    apps: AppList,
    learned_substring_index: HashMap<AppString, App>,
    substring_index: HashMap<AppString, Vec<AppName>>,

    /// Keeps track of the latest search query.
    /// The higher that number is, the more recent
    /// the query is.
    /// Used by receivers to detect when a new
    /// search was started on a different thread,
    /// in which case old results (with smaller tokens)
    /// should be discarded
    deferred_token: AtomicUsize,
    deferred_watcher: DeferredSender,
}

impl SearchEngine for DeterministicSearchEngine {
    fn blocking_search(&self, query: AppString) -> Vec<App> {
        let mut filtered_apps: Vec<App> = self.apps.to_vec();

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

    // Debug build code. Sends results back one by one for testing purposes :@)
    #[cfg(debug_assertions)]
    fn deferred_search(&self, query: AppString) -> (DeferredToken, DeferredReceiver) {
        let tx = self.deferred_watcher.clone();
        let rx = tx.subscribe();
        let token = self.deferred_token.fetch_add(1, Ordering::Acquire);
        tx.send_replace((token, vec![]));

        let res = self.blocking_search(query);

        for entry in res {
            tx.send_modify(|(w_token, vec)| {
                if token == *w_token {
                    vec.push(entry);
                }
            });
        }
        (token, rx)
    }

    fn selected(&mut self, query_history: Vec<AppName>, opened_app: &App) {
        query_history.into_par_iter().for_each(|query| {
            let _ = self
                .learned_substring_index
                .upsert_sync(query, opened_app.clone());
        });

        self.db.save_data(
            "learned_substring_index",
            self.learned_substring_index.clone(),
        );

        self.update();
    }

    fn update(&mut self) {
        self.deferred_token.store(0, Ordering::Release);
        // Check for modified apps, update if needed.
        let current_apps = &mut self.apps;
        let new_apps = apps();
        if new_apps.ne(current_apps) {
            let _ = std::mem::replace(current_apps, new_apps);

            self.index_apps();
        }
    }
}

impl DeterministicSearchEngine {
    #[must_use]
    pub fn build() -> Self {
        let db = FilesystemPersistence::open();
        let apps: AppList = apps();
        let substring_index: scc::HashMap<AppString, Vec<AppName>> = scc::HashMap::new();

        let learned_substring_index = db.get_data("learned_substring_index").unwrap_or_default();

        let (tx, _rx) = channel((0, vec![]));
        let mut engine = Self {
            db,
            apps,
            learned_substring_index,
            substring_index,
            deferred_token: AtomicUsize::new(0),
            deferred_watcher: tx,
        };

        engine.index_apps();

        engine
    }

    #[inline]
    fn index_apps(&mut self) {
        self.apps.par_iter().for_each(|app| {
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
        #[expect(clippy::missing_panics_doc, reason = "infallible")]
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
