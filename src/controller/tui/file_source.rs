// Copyright (C) 2026 AbdAlMoniem AlHifnawy
//
// This file is part of pidcatrs.
//
// pidcatrs is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// pidcatrs is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with pidcatrs.  If not, see <https://www.gnu.org/licenses/>.
//
// Author: AbdAlMoniem AlHifnawy

//! Filesystem path helpers for the open-file and export-file dialogs.
//!
//! The dialogs combine a typed path input with a file explorer. These
//! helpers pick sensible starting directories, expand `~`, split typed input
//! into a directory and a name fragment, and validate that a chosen path is
//! a readable log file.

#![deny(clippy::unwrap_used)]

use std::fs::File;
use std::path::Path;
use std::path::PathBuf;

/// Returns the directory the "open file" dialog starts in.
///
/// # Returns
///
/// The user's home directory (from `$HOME`), falling back to the current
/// working directory, and finally to `"."`.
pub fn default_browse_directory() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home);
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Returns the directory the "export" dialog starts in.
///
/// # Returns
///
/// The working directory the binary was launched from, falling back to
/// [`default_browse_directory`] if it cannot be determined.
pub fn default_export_directory() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| default_browse_directory())
}

/// Expands a leading `~` into the user's home directory.
///
/// Only `~` on its own and the `~/` prefix are expanded; `~user` forms are
/// left untouched.
///
/// # Arguments
///
/// * `path` - The path as typed by the user.
///
/// # Returns
///
/// The expanded path, or `path` unchanged if there is nothing to expand or
/// `$HOME` is not set.
pub fn expand_path(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return format!("{}/{stripped}", home.to_string_lossy());
    }

    if path == "~"
        && let Some(home) = std::env::var_os("HOME")
    {
        return home.to_string_lossy().to_string();
    }

    path.to_string()
}

/// Tests whether `ch` separates path components (`/` or the platform separator).
///
/// # Arguments
///
/// * `ch` - The character to test.
///
/// # Returns
///
/// `true` if `ch` is a path separator.
fn is_path_separator(ch: char) -> bool {
    ch == '/' || ch == std::path::MAIN_SEPARATOR
}

/// Renders a directory path with a trailing separator.
///
/// The trailing separator lets the user keep typing inside the directory.
///
/// # Arguments
///
/// * `path` - The directory path.
///
/// # Returns
///
/// The path as a string, ending in a path separator.
pub fn directory_input(path: &Path) -> String {
    let mut input = path.to_string_lossy().to_string();
    if !input.ends_with(is_path_separator) {
        input.push(std::path::MAIN_SEPARATOR);
    }
    input
}

/// Splits typed path input into its directory part and trailing name fragment.
///
/// `~` is expanded first. For example `/home/ab` becomes
/// (`/home/`, `ab`). Input without a separator has no directory part.
///
/// # Arguments
///
/// * `input` - The path text typed by the user.
///
/// # Returns
///
/// A tuple of the directory (including its trailing separator, or `None` if
/// the input contains no separator) and the remaining file-name fragment.
pub fn split_path_input(input: &str) -> (Option<PathBuf>, String) {
    let expanded = expand_path(input);
    match expanded.rfind(is_path_separator) {
        Some(index) => (
            Some(PathBuf::from(&expanded[..=index])),
            expanded[index + 1..].to_string(),
        ),
        None => (None, expanded),
    }
}

/// Validates that `path` refers to an existing, readable regular file.
///
/// Surrounding whitespace is trimmed and `~` is expanded before checking.
///
/// # Arguments
///
/// * `path` - The path as typed or selected by the user.
///
/// # Returns
///
/// `Ok` with the expanded path on success, or `Err` with a user-facing
/// message if the path is empty, does not exist, is not a regular file or
/// cannot be opened for reading.
pub fn validate_log_file(path: &str) -> Result<String, String> {
    let expanded = expand_path(path.trim());

    if expanded.is_empty() {
        return Err("path is empty".to_string());
    }

    let file_path = Path::new(&expanded);

    if !file_path.exists() {
        return Err(format!("file not found: {expanded}"));
    }

    if !file_path.is_file() {
        return Err(format!("not a file: {expanded}"));
    }

    File::open(file_path).map_err(|err| format!("cannot read file: {err}"))?;

    Ok(expanded)
}
