use std::{
    fs::File,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use gpui::{Global, Keystroke};
use rootcause::{Report, prelude::ResultExt, report};
use serde::{Deserialize, Serialize};

use crate::fs::apps::{APPLICATION_DIRS, APPLICATIONS};

const DEFAULT_HOTKEY: &str = "alt-space";
const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Configuration {
    pub open_search_hotkey: HotkeyString,
    pub launch_on_boot: bool,
    pub prioritize_open_apps: bool,
    pub applications: Vec<String>,
    pub application_dirs: Vec<String>,
}

/// Format is "[Modifiers]-Key"
/// Key is a key code in a string format defined in [`global_hotkey::hotkey::Code`]
///
/// Examples:
///   - alt-space
///   - ctrl-win-KeyC
pub type HotkeyString = String;

impl Default for Configuration {
    fn default() -> Self {
        Self {
            open_search_hotkey: DEFAULT_HOTKEY.to_string(),
            launch_on_boot: true,
            prioritize_open_apps: true,
            applications: default_applications(),
            application_dirs: default_application_dirs(),
        }
    }
}

impl Global for Configuration {}

impl Configuration {
    pub fn read_from_fs() -> Result<Configuration, Report> {
        let config_path = config_file_path()?;
        let config_file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&config_path)?;

        if let Ok(res) = serde_json::from_reader(&config_file) {
            Ok(res)
        } else {
            // Write defaults to fs if config file is corrupted or doesn'texist
            let config = Configuration::default();
            config.write_to_fs(&config_path)?;
            Ok(config)
        }
    }

    fn write_to_fs(&self, path: &Path) -> Result<(), Report> {
        let serialized = serde_json::to_vec_pretty(self)?;

        let mut config_file = File::options()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        config_file.write_all(serialized.as_ref())?;

        Ok(())
    }

    pub fn hotkey_config(&self) -> Result<HotKey, Report> {
        let parsed_global_hotkey =
            Keystroke::parse(&self.open_search_hotkey).attach("Expected a valid keystroke")?;

        let modifiers = {
            let mut m = Modifiers::empty();
            let gpui_m = parsed_global_hotkey.modifiers;

            if gpui_m.alt {
                m = m.union(Modifiers::ALT);
            }
            if gpui_m.control {
                m = m.union(Modifiers::CONTROL);
            }
            if gpui_m.function {
                m = m.union(Modifiers::FN);
            }
            if gpui_m.platform {
                m = m.union(Modifiers::META);
            }
            if gpui_m.shift {
                m = m.union(Modifiers::SHIFT);
            }

            m
        };

        let key_name = parsed_global_hotkey.key.clone();
        let code = if key_name.is_empty() {
            Code::Space
        } else {
            let key_name_uppercased: String = {
                let mut c = key_name.chars();
                match c.next() {
                    None => unreachable!("assert checks that key_name isn't empty"),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            };
            Code::from_str(key_name_uppercased.as_str()).attach("Need a valid hotkey key")?
        };

        debug_assert!(!modifiers.is_empty());

        Ok(HotKey::new(Some(modifiers), code))
    }
}

pub fn config_file_path() -> Result<PathBuf, Report> {
    let mut fetch_app_dir = dirs::data_local_dir()
        .ok_or_else(|| report!("No data local directory found (are you on a supported OS?)"))?;

    fetch_app_dir.push("Fetch");

    if let Err(io_err) = std::fs::create_dir(&fetch_app_dir) {
        match io_err.kind() {
            ErrorKind::AlreadyExists => { /* no-op */ }
            other => {
                return Err(report!(other)
                    .attach("Failed to create data directory")
                    .into());
            }
        }
    }

    fetch_app_dir.push(CONFIG_FILE_NAME);

    Ok(fetch_app_dir)
}

#[inline]
fn default_applications() -> Vec<String> {
    APPLICATIONS.iter().map(|app| (*app).to_string()).collect()
}

#[inline]
fn default_application_dirs() -> Vec<String> {
    APPLICATION_DIRS
        .iter()
        .map(|app_dir| (*app_dir).to_string())
        .collect()
}
