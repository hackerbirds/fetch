pub mod app_string;
pub mod app_substr;
pub mod fs;

use std::path::PathBuf;

use self::app_string::AppString;

pub type AppName = AppString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    pub(crate) name: AppName,
    pub(crate) path: PathBuf,
}

impl PartialOrd for App {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for App {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.path.cmp(&other.path)
    }
}
