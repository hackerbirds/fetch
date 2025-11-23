use std::{ffi::OsStr, fs::DirEntry, path::PathBuf};

use crate::apps::App;

pub type AppList = Box<[App]>;

#[cfg(target_os = "macos")]
const APPLICATION_DIRS: [&str; 5] = [
    "/Applications",
    "/Applications/Utilities",
    "/System/Applications",
    "/System/Applications/Utilities",
    "~/Applications",
];

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

pub fn apps() -> AppList {
    let app_paths: Vec<PathBuf> = APPLICATION_DIRS
        .into_iter()
        .filter_map(|app_dir| std::fs::read_dir(app_dir).ok())
        .flat_map(IntoIterator::into_iter)
        .filter_map(Result::ok)
        .filter_map(|entry| is_dir_entry_app(&entry).then_some(entry))
        .map(|app| app.path())
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
