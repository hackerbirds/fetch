//! Not really a "database", naive use of filesystem is good enough
//! for our use case

use std::{fs::File, io::Write};

use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;

use crate::fs::config::Configuration;

pub trait AppPersistence {
    fn get_configuration(&self) -> Configuration;
    fn save_configuration(&mut self, config: &Configuration);
    #[allow(
        clippy::missing_errors_doc,
        clippy::result_unit_err,
        reason = "Will improve error handling in future"
    )]
    fn get_data<T: DeserializeOwned>(&self, json_key: &str) -> Result<T, ()>;
    fn save_data<T: Serialize>(&mut self, json_key: &str, obj: T);
}

/// Very naive way of storing data on the filesystem, with JSON files.
/// Two assumptions about our use case to justify this choice:
///
/// 1) We store so little data (*at most* a few megabytes), everything
///    can fit in memory just fine
///
/// 2) Storing can be "slow", since indexing happens after search,
///    where the user doesn't use the app, so doing things this way is not
///    affecting performance
#[derive(Debug)]
pub struct FilesystemPersistence {
    config_file: File,
    data_file: File,
}

impl Default for FilesystemPersistence {
    fn default() -> Self {
        Self::open()
    }
}

impl FilesystemPersistence {
    #[must_use]
    /// # Panics
    ///
    /// Can panic on a non-supported Platform that doesn't have a well-defined home directory
    pub fn open() -> Self {
        let mut fetch_app_dir = dirs::data_local_dir()
            .expect("supported for all of fetch's platforms (macos, windows, and linux)");
        fetch_app_dir.push("Fetch");
        // TODO: Error handle permissions
        let _ = std::fs::create_dir(&fetch_app_dir);

        let config_file_path = {
            let mut path = fetch_app_dir.clone();
            path.push("config.json");

            path
        };

        let data_file_path = {
            let mut path = fetch_app_dir.clone();
            path.push("data.json");

            path
        };

        let config_file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(config_file_path)
            .expect("TODO (file opens)");

        let data_file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(data_file_path)
            .expect("TODO (file opens)");

        Self {
            config_file,
            data_file,
        }
    }
}

impl AppPersistence for FilesystemPersistence {
    fn get_configuration(&self) -> Configuration {
        serde_json::from_reader(&self.config_file).unwrap_or_default()
    }

    fn save_configuration(&mut self, config: &Configuration) {
        self.config_file
            .write_all(serde_json::to_vec(config).unwrap().as_ref())
            .unwrap();
    }

    fn get_data<T: DeserializeOwned>(&self, json_key: &str) -> Result<T, ()> {
        let generic_json: serde_json::Value =
            serde_json::from_reader(&self.data_file).map_err(|_| ())?;

        serde_json::from_value::<T>(generic_json.get(json_key).unwrap_or_default().clone())
            .map_err(|_| ())
    }

    fn save_data<T: Serialize>(&mut self, json_key: &str, obj: T) {
        let mut generic_json: serde_json::Value =
            serde_json::from_reader(&self.data_file).unwrap_or(json!({}));

        if let Some(map) = generic_json.as_object_mut() {
            let json_value = serde_json::to_value(obj).unwrap();

            map.insert(json_key.to_string(), json_value);

            self.data_file
                .write_all(serde_json::to_vec(&generic_json).unwrap().as_ref())
                .unwrap();
        }
    }
}
