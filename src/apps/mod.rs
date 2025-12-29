pub mod app_string;
pub mod app_substr;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use self::app_string::AppString;

pub type AppName = AppString;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct App {
    pub(crate) name: AppName,
    pub(crate) path: PathBuf,
    pub(crate) icon_png_img: Vec<u8>,
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
