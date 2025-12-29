use std::{
    fs::{DirEntry, File},
    io::BufReader,
    path::PathBuf,
    str::FromStr,
};

use icns::IconFamily;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

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

pub fn read_app_file(path: PathBuf) -> Result<App, ()> {
    if cfg!(target_os = "macos") {
        if !path.is_dir() {
            // Not a directory (apps on macOS are directories)
            return Err(());
        }

        if !path.as_path().extension().is_some_and(|d| d == "app") {
            // Not an .app
            return Err(());
        }

        let name = path
            .file_stem()
            .ok_or(())?
            .to_os_string()
            .into_string()
            .map_err(|_| ())?;

        //
        fn try_get_icon_data(name: &String, path: &PathBuf) -> Result<Vec<u8>, ()> {
            let info_plist_path = path.join("Contents/Info.plist");
            let mut info_plist_res = plist::Value::from_file(&info_plist_path);

            if let Err(_) = info_plist_res {
                // Low-effort attempt at loading iPad apps downloaded from Mac App Store.
                let info_plist_path = path.join(format!("Wrapper/{name}.app/Info.plist"));
                info_plist_res = plist::Value::from_file(info_plist_path)
            }

            let info_plist = info_plist_res.map_err(|_| ())?;

            // Extract an icon from Info.plist.

            // iPad apps downloaded from Mac App Store

            let icon_name = info_plist
                .as_dictionary()
                .expect("macOS plist is a dict")
                .get("CFBundleIconFile")
                .ok_or(())?
                .as_string()
                .ok_or(())?;

            #[allow(
                clippy::case_sensitive_file_extension_comparisons,
                reason = "APFS is case-insensitive"
            )]
            let icon_path = if !icon_name.ends_with(".icns") {
                path.join(format!("Contents/Resources/{icon_name}.icns"))
            } else {
                path.join(format!("Contents/Resources/{icon_name}"))
            };

            let icns_file = BufReader::new(File::open(icon_path).map_err(|_| ())?);
            let icon_family = IconFamily::read(icns_file).map_err(|_| ())?;

            let mut available_icons = icon_family.available_icons();
            available_icons.sort_by_cached_key(|k| k.pixel_width());
            // Ideally, ignore anything below 32x32 (too low quality)
            // `false` < `true`, so images bigger than 32x32 are sorted first
            available_icons.sort_by_cached_key(|k| k.pixel_width() <= 32);
            let smallest_available_icon_type = available_icons.first().ok_or(())?;

            let im = icon_family
                .get_icon_with_type(*smallest_available_icon_type)
                .map_err(|_| ())?;
            let mut png_data = Vec::new();
            let _ = im.write_png(&mut png_data);

            Ok(png_data)
        }

        let icon_png_img = try_get_icon_data(&name, &path).unwrap_or_default();

        Ok(App {
            name: name.into(),
            path,
            icon_png_img,
        })
    } else {
        todo!("Support for non-macOS")
    }
}
