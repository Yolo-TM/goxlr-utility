//! Storage for the Fader Paging configuration.
//!
//! Fader Paging lets the Sampler Bank buttons (A / B / C) double as 'pages' for the faders, each
//! page can assign a different channel, colours and scribble to each of the four faders.
//!
//! The configuration lives in `fader-pages.json`, next to `settings.json`, and is keyed by the
//! serial number of the device. It is deliberately kept out of the profile and the main settings
//! so it survives profile changes and can be edited / shared on its own.

use crate::config_store::DeviceConfigStore;
use goxlr_ipc::FaderPagingConfig;

pub const FADER_PAGES_FILE_NAME: &str = "fader-pages.json";

pub type FaderPagesHandle = DeviceConfigStore<FaderPagingConfig>;
