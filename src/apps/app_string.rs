use std::{fmt::Display, ops::Deref};

use arcstr::ArcStr;
use gpui::SharedString;
use unicase::UniCase;
use unicode_segmentation::UnicodeSegmentation;

use crate::apps::app_substr::AppSubstr;

/// NOTE: Case insensitive, efficient representation of an immutable string
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AppString(UniCase<ArcStr>);

impl Display for AppString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AppString {
    #[inline]
    #[must_use]
    pub fn grapheme_len(&self) -> usize {
        #[expect(clippy::missing_panics_doc, reason = "upper bound size hint exists")]
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
        SharedString::new(value.0.as_str())
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
