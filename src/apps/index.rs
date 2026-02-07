use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use icns::IconFamily;
use rayon::{
    iter::{IntoParallelRefIterator, ParallelIterator},
    str::ParallelString,
};
use rootcause::{Report, report};
use scc::{Guard, HashIndex, HashSet};

use crate::{apps::ExecutableApp, fs::config::Configuration};

/// An index map of all known apps, optimized for fast reads.
#[derive(Debug, Clone)]
pub struct AppIndex(Arc<Configuration>, scc::HashIndex<PathBuf, ExecutableApp>);

impl AppIndex {
    fn list_running_apps() -> Vec<PathBuf> {
        if cfg!(target_os = "macos") {
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
        } else {
            todo!("Support for non-macOS platforms")
        }
    }

    fn list_apps(config: &Configuration) -> HashSet<PathBuf> {
        if cfg!(target_os = "macos") {
            let mut cmd = Command::new("mdfind");
            cmd.arg("kMDItemKind == 'Application'");

            for path in &config.application_dirs {
                cmd.arg("-onlyin");
                cmd.arg(path);
            }

            let mdfind_bytes = cmd.output().unwrap().stdout;

            let apps = String::from_utf8(mdfind_bytes).unwrap();

            let set = HashSet::new();

            apps.par_split('\n').map(PathBuf::from).for_each(|p| {
                let _ = set.insert_sync(p);
            });

            config.applications.par_iter().for_each(|app_path| {
                let _ = set.insert_sync(app_path.to_owned().into());
            });

            set
        } else {
            todo!("Support for non-macOS platforms")
        }
    }

    fn read_app_file(
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
                    report!(x).attach(
                        "This file path isn't UTF-8 compatible (are you using a supported OS?)",
                    )
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

    #[must_use]
    pub fn build(config: Arc<Configuration>) -> Self {
        if cfg!(target_os = "macos") {
            let running_apps = Self::list_running_apps();
            let apps = Self::list_apps(&config);
            let map = HashIndex::with_capacity(apps.len());

            apps.iter_sync(|p| {
                if let Ok(ok) = Self::read_app_file(p.clone(), &running_apps) {
                    let _ = map.insert_sync(p.clone(), ok);
                }

                true
            });

            Self(config, map)
        } else {
            todo!("Support for non-macOS platforms")
        }
    }

    pub fn update(&self) {
        let running_apps = Self::list_running_apps();
        let apps = Self::list_apps(&self.0);
        self.1.retain_sync(|k, _v| apps.contains_sync(k));
        apps.iter_sync(|app| {
            if let Ok(ok) = Self::read_app_file(app.clone(), &running_apps) {
                // If the key already exists (kept from the retain call)
                // then this doesn't update, so it stays efficient
                let _ = self.1.insert_sync(app.clone(), ok);
            }

            true
        });
    }

    pub fn get<'a>(&'a self, path: &'a Path, guard: &'a Guard) -> Option<&'a ExecutableApp> {
        self.1.peek(path, guard)
    }

    pub fn iter<'a>(
        &'a self,
        guard: &'a Guard,
    ) -> impl Iterator<Item = (&'a PathBuf, &'a ExecutableApp)> {
        self.1.iter(guard)
    }
}
