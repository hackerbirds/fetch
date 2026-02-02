use std::{
    fs::{DirEntry, File},
    io::BufReader,
    path::{Path, PathBuf},
    process::Command,
};

use icns::IconFamily;
use rootcause::{prelude::Report, report};

use crate::{apps::ExecutableApp, fs::config::Configuration};

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
#[must_use]
pub fn apps(config: &Configuration) -> AppList {
    if cfg!(target_os = "macos") {
        let running_apps = {
            let lsappinfo_bytes = Command::new("lsappinfo")
                .arg("list")
                .output()
                .unwrap()
                .stdout;

            let lsappinfo_res = String::from_utf8(lsappinfo_bytes).unwrap();

            lsappinfo_res
                .split('\n')
                .filter_map(|p| {
                    const BUNDLE_PATH_PREFIX: &str = "    bundle path=";
                    if p.starts_with(BUNDLE_PATH_PREFIX) {
                        // TODO: Use trim_prefix + trim_suffix when stabilized
                        // https://github.com/rust-lang/rust/issues/142312

                        let mut bundle_path = p.to_owned();

                        // remove prefix + double quote of path
                        bundle_path = bundle_path.split_off(BUNDLE_PATH_PREFIX.len() + 1);
                        // remove double quote of path
                        let _ = bundle_path.split_off(bundle_path.len() - 1);
                        Some(bundle_path)
                    } else {
                        None
                    }
                })
                .map(PathBuf::from)
                .collect::<Vec<PathBuf>>()
        };

        let mut cmd = Command::new("mdfind");
        cmd.arg("kMDItemKind == 'Application'");

        for path in &config.application_dirs {
            cmd.arg("-onlyin");
            cmd.arg(path);
        }

        for app in &config.applications {
            cmd.arg("-onlyin");
            cmd.arg(app);
        }

        let mdfind_bytes = cmd.output().unwrap().stdout;

        let apps = String::from_utf8(mdfind_bytes).unwrap();

        apps.split('\n')
            .filter_map(|p| read_app_file(p.into(), &running_apps).ok())
            .collect::<Vec<ExecutableApp>>()
            .into()
    } else {
        todo!("Support for non-macOS platforms")
    }
}

pub fn read_app_file(
    path: PathBuf,
    running_app_paths: &[PathBuf],
) -> Result<ExecutableApp, Report> {
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

        let icon_png_data = try_get_icon_data(&name, &path).ok();

        Ok(ExecutableApp {
            name: name.into(),
            is_open: running_app_paths.contains(&path),
            path,
            icon_png_data,
        })
    } else {
        todo!("Support for non-macOS")
    }
}
