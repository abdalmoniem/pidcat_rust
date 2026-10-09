// Copyright (C) AbdAlMoniem AlHifnawy <hifnawy_moniem@hotmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

#![deny(clippy::unwrap_used)]

//! Default locations for the user configuration file and custom theme directory.

use std::path::PathBuf;

use etcetera::BaseStrategy;
use etcetera::choose_base_strategy;

/// File name of the main TOML configuration in [`config_dir`].
pub const CONFIG_FILE_NAME: &str = "config.toml";
/// Subdirectory name under [`config_dir`] where custom theme files are stored.
pub const THEMES_DIR_NAME: &str = "themes";

/// Returns `$XDG_CONFIG_HOME/<pkg>` or `~/.config/<pkg>` on Linux and macOS, `%APPDATA%\<pkg>` on Windows.
pub fn config_dir() -> Option<PathBuf> {
    choose_base_strategy()
        .ok()
        .map(|strategy| strategy.config_dir().join(env!("CARGO_PKG_NAME")))
}

/// Path to the default `config.toml` inside [`config_dir`], if that directory is known.
pub fn default_config_file() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join(CONFIG_FILE_NAME))
}

/// Path to the `themes` subdirectory inside [`config_dir`], if that directory is known.
pub fn themes_dir() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join(THEMES_DIR_NAME))
}
