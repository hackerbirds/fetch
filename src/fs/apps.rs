use std::{ffi::OsStr, fs::DirEntry, path::PathBuf, str::FromStr};

use crate::{apps::App, fs::config::Configuration};

pub type AppList = Box<[App]>;

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

#[inline]
#[must_use]
pub fn is_dir_entry_app(dir_entry: &DirEntry) -> bool {
    if cfg!(target_os = "macos") {
        dir_entry.path().extension().is_some_and(|d| d == "app")
    } else if cfg!(target_os = "windows") {
        // TODO: Untested
        dir_entry.path().extension().is_some_and(|d| d == "exe")
    } else {
        // Linux?
        todo!("Support for Linux systems (look through ELF metadata?)")
    }
}

#[inline]
pub fn apps(config: &Configuration) -> AppList {
    let default_app_paths = config
        .applications
        .iter()
        .filter_map(|app_path| PathBuf::from_str(app_path).ok());

    let app_paths: Vec<PathBuf> = config
        .application_dirs
        .iter()
        .filter_map(|app_dir| std::fs::read_dir(app_dir).ok())
        .flat_map(IntoIterator::into_iter)
        .filter_map(Result::ok)
        .filter_map(|entry| is_dir_entry_app(&entry).then_some(entry))
        .map(|app| app.path())
        .chain(default_app_paths)
        .collect();

    app_paths
        .clone()
        .into_iter()
        .filter_map(|p| p.file_stem().map(OsStr::to_os_string))
        .filter_map(|file_name| file_name.into_string().ok())
        .zip(app_paths.clone())
        .map(|(s, p)| App {
            name: s.into(),
            path: p,
        })
        .collect::<Vec<App>>()
        .into()
}
