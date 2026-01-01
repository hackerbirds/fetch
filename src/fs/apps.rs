use std::{
    fs::{DirEntry, File},
    io::BufReader,
    path::{Path, PathBuf},
    str::FromStr,
};

use icns::IconFamily;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rootcause::{prelude::Report, report};

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
        .into_par_iter()
        .filter_map(|p| read_app_file(p).ok())
        .collect::<Vec<App>>()
        .into()
}

pub fn read_app_file(path: PathBuf) -> Result<App, Report> {
    if cfg!(target_os = "macos") {
        // Because try blocks aren't stabilized, make this a function
        // so that error propagation stops at the function scope if icon
        // fails to load.
        fn try_get_icon_data(name: &String, path: &Path) -> Result<Vec<u8>, Report> {
            let info_plist_path = path.join("Contents/Info.plist");
            let mut info_plist_res = plist::Value::from_file(&info_plist_path);

            if info_plist_res.is_err() {
                // Low-effort attempt at loading iPad apps downloaded from Mac App Store.
                let info_plist_path = path.join(format!("Wrapper/{name}.app/Info.plist"));
                info_plist_res = plist::Value::from_file(info_plist_path);
            }

            let info_plist = info_plist_res?;

            // Extract an icon from Info.plist.

            // iPad apps downloaded from Mac App Store

            let icon_name = info_plist
                .as_dictionary()
                .expect("macOS plist is a dict")
                .get("CFBundleIconFile")
                .ok_or_else(|| report!("CFBundleIconFile not present in Info.plist"))?
                .as_string()
                .ok_or_else(|| {
                    report!(
                        "Could not convert CFBundleIconFile value into String (it wasn't a String?)"
                    )
                })?;

            #[allow(
                clippy::case_sensitive_file_extension_comparisons,
                reason = "APFS is case-insensitive"
            )]
            let icns_suffix = if icon_name.ends_with(".icns") {
                ""
            } else {
                ".icns"
            };

            let icon_path = path.join(format!("Contents/Resources/{icon_name}{icns_suffix}"));
            let icns_file = BufReader::new(File::open(icon_path)?);
            let icon_family = IconFamily::read(icns_file)?;

            let mut available_icons = icon_family.available_icons();
            available_icons.sort_by_cached_key(|k| k.pixel_width());
            // Ideally, ignore anything below 32x32 (too low quality)
            // `false` < `true`, so images bigger than 32x32 are sorted first
            available_icons.sort_by_cached_key(|k| k.pixel_width() <= 32);
            let smallest_available_icon_type = available_icons
                .first()
                .ok_or_else(|| report!("No available icons for app {name}"))?;

            let im = icon_family.get_icon_with_type(*smallest_available_icon_type)?;
            let mut png_data = Vec::new();
            let _ = im.write_png(&mut png_data);

            Ok(png_data)
        }

        if !path.is_dir() {
            // Not a directory (apps on macOS are directories)
            return Err(report!("This `.app` path isn't a directory"));
        }

        if path.as_path().extension().is_none_or(|d| d != "app") {
            // Not an .app
            return Err(report!("This path doesn't end with `.app`"));
        }

        let name = path
            .file_stem()
            .expect("This path must have a file stem (due to previous .app extension check)")
            .to_os_string()
            .into_string()
            .map_err(|x| {
                report!(x)
                    .attach("This file path isn't UTF-8 compatible (are you using a supported OS?)")
            })?;

        let icon_png_img = match try_get_icon_data(&name, &path) {
            Ok(icon_data) => icon_data,
            #[cfg_attr(
                not(debug_assertions),
                allow(unused_variables, reason = "Debug-only log")
            )]
            Err(report) => {
                println!(
                    "{}",
                    report.context(format!("Could not load icon for app \"{name}\""))
                );

                Vec::new()
            }
        };

        Ok(App {
            name: name.into(),
            path,
            icon_png_img,
        })
    } else {
        todo!("Support for non-macOS")
    }
}
