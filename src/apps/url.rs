use std::{borrow::Cow, fmt::Display, path::PathBuf, process::Command};

use rootcause::Report;
use serde::{Deserialize, Serialize};

use crate::apps::ExecutableApp;

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

impl Url {
    pub fn open(&self) -> Result<(), Report> {
        Command::new("open")
            .arg("-u")
            .arg(self.to_string())
            .spawn()?;

        Ok(())
    }
}

impl From<PathBuf> for Url {
    fn from(value: PathBuf) -> Self {
        Self::File(value)
    }
}
