use std::{fmt::Display, ops::Deref, path::PathBuf};

use arcstr::{ArcStr, Substr};
use gpui::SharedString;
use serde::{Deserialize, Serialize};
use unicase::UniCase;
use unicode_segmentation::UnicodeSegmentation;

/// Case insensitive, efficient representation of an immutable UTF-8 encoded string
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AppString(#[serde(with = "unicase_serde::unicase")] UniCase<ArcStr>);

/// NOTE: Case insensitive, efficient representation of an immutable substring
///
/// Obtained with [`AppString::substring`]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppSubstr(pub(super) UniCase<Substr>);

pub type AppName = AppString;
pub type AppList = Box<[ExecutableApp]>;

/// An executable app the user can launch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct ExecutableApp {
    pub(crate) name: AppName,
    pub(crate) path: PathBuf,
    pub(crate) is_open: bool,
    pub(crate) icon_png_data: Option<Vec<u8>>,
}

impl Deref for AppSubstr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl From<String> for AppSubstr {
    fn from(value: String) -> Self {
        Self(UniCase::new(Substr::from(value)))
    }
}

impl From<&str> for AppSubstr {
    fn from(value: &str) -> Self {
        Self(UniCase::new(Substr::from(value)))
    }
}

impl Display for AppString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AppString {
    #[inline]
    #[must_use]
    pub fn grapheme_len(&self) -> usize {
        self.0
            .graphemes(true)
            .size_hint()
            .1
            .expect("upper bound size hint exists")
    }

    #[inline]
    #[must_use]
    pub fn substring(&self, i: usize, len: usize) -> AppSubstr {
        AppSubstr(UniCase::new(self.0.substr(i..i + len)))
    }
}

impl From<SharedString> for AppString {
    fn from(value: SharedString) -> Self {
        Self::from(value.as_str())
    }
}

impl From<AppString> for SharedString {
    fn from(value: AppString) -> Self {
        SharedString::new(value.0.into_inner())
    }
}

impl From<String> for AppString {
    fn from(value: String) -> Self {
        Self(UniCase::new(ArcStr::from(value)))
    }
}

impl From<&str> for AppString {
    fn from(value: &str) -> Self {
        Self(UniCase::new(ArcStr::from(value)))
    }
}

impl Deref for AppString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl PartialOrd for ExecutableApp {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ExecutableApp {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.path.cmp(&other.path)
    }
}
