use std::{borrow::Cow, fmt::Display, path::PathBuf};

use serde::{Deserialize, Serialize};

use scc::{Guard, HashIndex};

use crate::{
    app::ExecutableApp,
    fs::config::Configuration,
    platform::{ImplPlatform, Platform},
};

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Url {
    /// A URL to handle opening files (`file://`)
    File(PathBuf),
    /// A URL to handle opening web URLs (`https://`)
    Https(Cow<'static, str>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UrlEntry {
    Url { name: String, url: Url },
    App { app: ExecutableApp },
}

impl Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Url::File(path_buf) => {
                write!(f, "file://{}", path_buf.display())
            }
            Url::Https(domain) => {
                write!(f, "https://{domain}")
            }
        }
    }
}

impl From<PathBuf> for Url {
    fn from(value: PathBuf) -> Self {
        Self::File(value)
    }
}

/// An index map of all known apps, optimized for fast reads.
#[derive(Debug, Clone)]
pub struct UrlIndex(scc::HashIndex<Url, UrlEntry>);

impl UrlIndex {
    #[must_use]
    pub fn build(config: &Configuration) -> Self {
        let apps = ImplPlatform::list_binary_paths(config);
        let map = HashIndex::with_capacity(apps.len());

        apps.iter_sync(|p| {
            let url = Url::File(p.clone());
            if let Some(url_entry) = ImplPlatform::to_url_entry(&url) {
                let _ = map.insert_sync(url, url_entry);
            }

            true
        });

        Self(map)
    }

    pub fn update(&self, config: &Configuration) {
        let apps = ImplPlatform::list_binary_paths(config);
        self.0.retain_sync(|k, _v| {
            if let Url::File(path) = k {
                apps.contains_sync(path)
            } else {
                false
            }
        });
        apps.iter_sync(|app| {
            let url = Url::File(app.clone());
            if let Some(url_entry) = ImplPlatform::to_url_entry(&url) {
                // If the key already exists (kept from the retain call)
                // then this doesn't update, so it stays efficient
                let _ = self.0.insert_sync(url, url_entry);
            }

            true
        });
    }

    pub fn get<'a>(&'a self, url: &'a Url, guard: &'a Guard) -> Option<&'a UrlEntry> {
        self.0.peek(url, guard)
    }

    pub fn iter<'a>(&'a self, guard: &'a Guard) -> impl Iterator<Item = (&'a Url, &'a UrlEntry)> {
        self.0.iter(guard)
    }
}
