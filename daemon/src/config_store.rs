//! A small per-device JSON config store, living in its own file next to `settings.json`.
//!
//! Used for configuration that shouldn't live inside a profile or the main settings (Fader Paging,
//! Sampler Actions). The file is keyed by device serial, written atomically, and a file that fails
//! to parse is backed up (`<name>.failed`) while the defaults are loaded.

use anyhow::{Context, Result};
use log::{debug, error, warn};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::fs;
use std::fs::{File, create_dir_all};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct DeviceConfigStore<T> {
    path: PathBuf,
    devices: Arc<RwLock<HashMap<String, T>>>,
}

impl<T> Clone for DeviceConfigStore<T> {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            devices: self.devices.clone(),
        }
    }
}

impl<T: Serialize + DeserializeOwned + Default + Clone> DeviceConfigStore<T> {
    /// Loads `file_name`, located next to the provided settings file.
    pub fn load(settings_path: &Path, file_name: &str) -> Self {
        let path = settings_path.with_file_name(file_name);
        let devices = match Self::read(&path) {
            Ok(devices) => devices,
            Err(e) => {
                error!("Unable to read {}: {:#}", path.to_string_lossy(), e);
                HashMap::new()
            }
        };

        Self {
            path,
            devices: Arc::new(RwLock::new(devices)),
        }
    }

    pub async fn get(&self, serial: &str) -> T {
        self.devices
            .read()
            .await
            .get(serial)
            .cloned()
            .unwrap_or_default()
    }

    /// Replaces the configuration for a device and writes the file.
    pub async fn set(&self, serial: &str, config: T) {
        let mut devices = self.devices.write().await;
        devices.insert(serial.to_string(), config);

        if let Err(e) = Self::write(&self.path, &devices) {
            error!("Couldn't save {}: {:#}", self.path.to_string_lossy(), e);
        }
    }

    fn read(path: &Path) -> Result<HashMap<String, T>> {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(HashMap::new()),
            Err(e) => {
                return Err(e).context("Could not open the file for reading");
            }
        };

        match serde_json::from_reader(file) {
            Ok(devices) => Ok(devices),
            Err(e) => {
                // Same approach as the main settings, keep a copy of the broken file and
                // carry on with the defaults rather than refusing to start.
                warn!("Error parsing {} ({e}), loading defaults.", path.display());
                let mut backup = PathBuf::from(path);
                backup.set_extension("failed");
                if let Err(e) = fs::copy(path, &backup) {
                    warn!("Unable to back up the broken file: {e}");
                }
                Ok(HashMap::new())
            }
        }
    }

    fn write(path: &Path, devices: &HashMap<String, T>) -> Result<()> {
        if let Some(parent) = path.parent() {
            create_dir_all(parent).context("Could not create the config directory")?;
        }

        let mut tmp = path.to_path_buf();
        tmp.set_extension("tmp");

        debug!("Saving {:?}", path);
        let file = File::create(&tmp)?;
        serde_json::to_writer_pretty(&file, devices)?;
        file.sync_all()?;
        drop(file);

        if path.exists() {
            fs::remove_file(path).unwrap_or_else(|e| warn!("Error Removing File: {e}"));
        }
        fs::rename(tmp, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    struct Sample {
        enabled: bool,
        names: Vec<String>,
    }

    fn temp_settings_path(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("goxlr-config-store-{name}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir.join("settings.json")
    }

    #[tokio::test]
    async fn round_trips_per_device() {
        let settings = temp_settings_path("roundtrip");
        let store = DeviceConfigStore::<Sample>::load(&settings, "sample.json");
        assert!(!store.get("S1").await.enabled);

        let config = Sample {
            enabled: true,
            names: vec!["a".into()],
        };
        store.set("S1", config.clone()).await;

        let reloaded = DeviceConfigStore::<Sample>::load(&settings, "sample.json");
        assert_eq!(reloaded.get("S1").await, config);
        assert_eq!(reloaded.get("other").await, Sample::default());
    }

    #[tokio::test]
    async fn corrupt_file_is_backed_up_and_defaults_loaded() {
        let settings = temp_settings_path("corrupt");
        fs::write(settings.with_file_name("sample.json"), "{ nope").unwrap();

        let store = DeviceConfigStore::<Sample>::load(&settings, "sample.json");
        assert_eq!(store.get("S1").await, Sample::default());
        assert!(settings.with_file_name("sample.failed").exists());
    }
}
