use std::sync::Mutex;

use rayon::{
    iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use scc::HashMap;
use unicode_segmentation::UnicodeSegmentation;

use crate::apps::{
    App, AppName,
    app_string::AppString,
    app_substr::AppSubstr,
    fs::{AppList, apps},
};

#[derive(Debug, Default)]
pub struct SearchEngine {
    apps: Mutex<AppList>,
    learned_substring_index: scc::HashMap<AppString, App>,
    substring_index: scc::HashMap<AppString, Vec<AppName>>,
}

impl SearchEngine {
    pub fn search(&self, query: &AppString) -> Vec<App> {
        #[allow(clippy::missing_panics_doc, reason = "Infallible mutex lock")]
        let mut filtered_apps: Vec<App> =
            self.apps.lock().expect("mutex lock can't poison").to_vec();

        filtered_apps.par_sort_by_cached_key(|app| app.name.clone());

        filtered_apps = filtered_apps
            .into_par_iter()
            .filter(|app| self.is_query_substring_of_app_name(query, &app.name))
            .collect();

        filtered_apps.par_sort_by_cached_key(|app| {
            if query == &app.name {
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
                    .get_sync(query)
                    .is_none_or(|s| s.get().name != app.name),
            )
        });

        filtered_apps
    }

    pub fn selected(&self, query_history: Vec<AppName>, opened_app: &App) {
        query_history.into_par_iter().for_each(|query| {
            let _ = self
                .learned_substring_index
                .upsert_sync(query, opened_app.clone());
        });

        self.update();
    }

    /// If needed, update the search engine.
    pub fn update(&self) {
        // Check for modified apps, update if needed.
        #[allow(clippy::missing_panics_doc, reason = "Infallible mutex lock")]
        let mut current_apps = self.apps.lock().expect("mutex lock can't poison");
        let new_apps = apps();
        if current_apps.ne(&new_apps) {
            let _ = std::mem::replace(&mut *current_apps, new_apps);

            // Drop lock
            drop(current_apps);
            self.index_apps();
        }
    }

    #[must_use]
    pub fn build() -> Self {
        let apps: AppList = apps();
        let substring_index: scc::HashMap<AppString, Vec<AppName>> = scc::HashMap::new();

        let engine = Self {
            apps: Mutex::new(apps),
            learned_substring_index: HashMap::new(),
            substring_index,
        };

        engine.index_apps();

        engine
    }

    #[inline]
    /// Note: Grabs the Mutex lock. Don't call this while holding the self.apps lock
    fn index_apps(&self) {
        #[allow(clippy::missing_panics_doc, reason = "Infallible mutex lock")]
        self.apps
            .lock()
            .expect("mutex lock can't poison")
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
