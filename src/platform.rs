use std::path::PathBuf;

use rootcause::Report;
use scc::HashSet;

use crate::{
    fs::config::Configuration,
    url::{Url, UrlEntry},
};

#[cfg(target_os = "macos")]
pub mod mac;

#[cfg(target_os = "macos")]
pub type ImplPlatform = mac::MacPlatform;

/// A collection of utility functions that are platform-dependant.
pub trait Platform {
    /// List of the paths of apps included by default.
    fn default_app_paths() -> Vec<PathBuf>;

    /// List of the default directories to check for apps within.
    fn default_app_dirs() -> Vec<PathBuf>;

    /// List of binaries to display in search results.
    fn list_binary_paths(config: &Configuration, quick: bool) -> HashSet<PathBuf>;

    /// List of the path of the binaries that are currently running
    /// on the system.
    fn list_open_binaries() -> Vec<PathBuf>;

    /// Takes a URL and converts it to a [`UrlEntry`], for displaying.
    /// As an example, an application would have a [`UrlEntry`] containing
    /// the app name, app icon, etc.
    fn to_url_entry(url: &Url) -> Option<UrlEntry>;

    fn open_url(url: &Url) -> Result<(), Report>;
}
