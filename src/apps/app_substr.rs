use std::ops::Deref;

use arcstr::Substr;
use unicase::UniCase;

/// NOTE: Case insensitive, efficient representation of an immutable substring
///
/// Obtained with [`AppString::substring`]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppSubstr(pub(super) UniCase<Substr>);

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
