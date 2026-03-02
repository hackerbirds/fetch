use std::{borrow::Cow, fmt::Display, path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};

use scc::{Guard, HashIndex};

use crate::{
    apps::ExecutableApp,
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
pub struct UrlIndex(Arc<Configuration>, scc::HashIndex<Url, UrlEntry>);

impl UrlIndex {
    #[must_use]
    pub fn build(config: Arc<Configuration>) -> Self {
        let apps = ImplPlatform::list_binary_paths(&config);
        let map = HashIndex::with_capacity(apps.len());

        apps.iter_sync(|p| {
            let url = Url::File(p.clone());
            if let Some(url_entry) = ImplPlatform::to_url_entry(&url) {
                let _ = map.insert_sync(url, url_entry);
            }

            true
        });

        Self(config, map)
    }

    pub fn update(&self) {
        let apps = ImplPlatform::list_binary_paths(&self.0);
        self.1.retain_sync(|k, _v| {
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
                let _ = self.1.insert_sync(url, url_entry);
            }

            true
        });
    }

    pub fn get<'a>(&'a self, url: &'a Url, guard: &'a Guard) -> Option<&'a UrlEntry> {
        self.1.peek(url, guard)
    }

    pub fn iter<'a>(&'a self, guard: &'a Guard) -> impl Iterator<Item = (&'a Url, &'a UrlEntry)> {
        self.1.iter(guard)
    }
}
