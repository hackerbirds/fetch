//! Not really a "database", naive use of filesystem is good enough
//! for our use case

use std::{fs::File, os::unix::fs::FileExt};

use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;

pub trait AppPersistence {
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

        let data_file_path = {
            let mut path = fetch_app_dir.clone();
            path.push("data.json");

            path
        };

        let data_file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(data_file_path)
            .expect("TODO (file opens)");

        Self { data_file }
    }
}

impl AppPersistence for FilesystemPersistence {
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
                .write_all_at(serde_json::to_vec(&generic_json).unwrap().as_ref(), 0)
                .unwrap();
        }
    }
}
