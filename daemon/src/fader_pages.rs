//! Storage for the Fader Paging configuration.
//!
//! Fader Paging lets the Sampler Bank buttons (A / B / C) double as 'pages' for the faders, each
//! page can assign a different channel, colours and scribble to each of the four faders.
//!
//! The configuration lives in `fader-pages.json`, next to `settings.json`, and is keyed by the
//! serial number of the device. It is deliberately kept out of the profile and the main settings
//! so it survives profile changes and can be edited / shared on its own.

use anyhow::{Context, Result};
use goxlr_ipc::FaderPagingConfig;
use log::{debug, error, warn};
use std::collections::HashMap;
use std::fs;
use std::fs::{File, create_dir_all};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

pub const FADER_PAGES_FILE_NAME: &str = "fader-pages.json";

#[derive(Debug, Clone)]
pub struct FaderPagesHandle {
    path: PathBuf,
    devices: Arc<RwLock<HashMap<String, FaderPagingConfig>>>,
}

impl FaderPagesHandle {
    /// Loads the file located next to the provided settings file.
    pub fn load(settings_path: &Path) -> FaderPagesHandle {
        let path = settings_path.with_file_name(FADER_PAGES_FILE_NAME);
        let devices = match Self::read(&path) {
            Ok(devices) => devices,
            Err(e) => {
                error!("Unable to read {}: {:#}", path.to_string_lossy(), e);
                HashMap::new()
            }
        };

        FaderPagesHandle {
            path,
            devices: Arc::new(RwLock::new(devices)),
        }
    }

    pub async fn get(&self, serial: &str) -> FaderPagingConfig {
        self.devices
            .read()
            .await
            .get(serial)
            .cloned()
            .unwrap_or_default()
    }

    /// Replaces the configuration for a device and writes the file.
    pub async fn set(&self, serial: &str, config: FaderPagingConfig) {
        let mut devices = self.devices.write().await;
        devices.insert(serial.to_string(), config);

        if let Err(e) = Self::write(&self.path, &devices) {
            error!(
                "Couldn't save fader pages to {}: {:#}",
                self.path.to_string_lossy(),
                e
            );
        }
    }

    fn read(path: &Path) -> Result<HashMap<String, FaderPagingConfig>> {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(HashMap::new()),
            Err(e) => {
                return Err(e).context("Could not open the Fader Pages file for reading");
            }
        };

        match serde_json::from_reader(file) {
            Ok(devices) => Ok(devices),
            Err(e) => {
                // Same approach as the main settings, keep a copy of the broken file and
                // carry on with the defaults rather than refusing to start.
                warn!("Error parsing Fader Pages ({e}), loading defaults.");
                let mut backup = PathBuf::from(path);
                backup.set_extension("failed");
                if let Err(e) = fs::copy(path, &backup) {
                    warn!("Unable to back up the broken file: {e}");
                }
                Ok(HashMap::new())
            }
        }
    }

    fn write(path: &Path, devices: &HashMap<String, FaderPagingConfig>) -> Result<()> {
        if let Some(parent) = path.parent() {
            create_dir_all(parent).context("Could not create the config directory")?;
        }

        let mut tmp = path.to_path_buf();
        tmp.set_extension("tmp");

        debug!("Saving Fader Pages to {:?}", path);
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
