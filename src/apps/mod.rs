pub mod app_string;
pub mod app_substr;
pub mod index;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use self::app_string::AppString;

pub type AppName = AppString;
pub type AppList = Box<[ExecutableApp]>;

#[cfg(target_os = "macos")]
pub(crate) const APPLICATION_DIRS: [&str; 6] = [
    "/Applications",
    "/Applications/Utilities",
    "/System/Applications",
    "/System/Applications/Utilities",
    "/System/Library/CoreServices/Applications",
    "~/Applications",
];

#[cfg(target_os = "macos")]
pub(crate) const APPLICATIONS: [&str; 1] = ["/System/Library/CoreServices/Finder.app"];

/// An executable app the user can launch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct ExecutableApp {
    pub(crate) name: AppName,
    pub(crate) path: PathBuf,
    pub(crate) is_open: bool,
    pub(crate) icon_png_data: Option<Vec<u8>>,
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
