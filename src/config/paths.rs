#![deny(clippy::unwrap_used)]

use std::path::PathBuf;

use etcetera::BaseStrategy;
use etcetera::choose_base_strategy;

pub const CONFIG_FILE_NAME: &str = "config.toml";
pub const THEMES_DIR_NAME: &str = "themes";

/// `$XDG_CONFIG_HOME/<pkg>` or `~/.config/<pkg>` on Linux and macOS, `%APPDATA%\<pkg>` on Windows.
pub fn config_dir() -> Option<PathBuf> {
    choose_base_strategy()
        .ok()
        .map(|strategy| strategy.config_dir().join(env!("CARGO_PKG_NAME")))
}

pub fn default_config_file() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join(CONFIG_FILE_NAME))
}

pub fn themes_dir() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join(THEMES_DIR_NAME))
}
