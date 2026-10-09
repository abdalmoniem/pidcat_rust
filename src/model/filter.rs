// Copyright (c) AbdAlMoniem AlHifnawy <hifnawy_moniem@hotmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

//! Pure predicates for package ownership, log level, tags, and display names.

#![deny(clippy::unwrap_used)]

use std::collections::HashMap;

use crate::LogEntry;
use crate::LogLevel;
use crate::State;
use crate::controller::log_processor::is_matching_tag;

/// Returns true when `all` is set or `owner` appears as a key in `package_map`.
pub fn passes_package_ownership(
    owner: &str,
    package_map: &HashMap<String, String>,
    all: bool,
) -> bool {
    all || package_map.contains_key(owner)
}

/// Returns true when `level` meets or exceeds `min_level` ([`LogLevel`] ordering).
pub fn passes_log_level(level: LogLevel, min_level: LogLevel) -> bool {
    level >= min_level
}

/// Returns true when `tag` matches any entry in `tag_filters` (glob rules via `is_matching_tag`).
pub fn passes_tag_filter(tag: &str, tag_filters: &[String]) -> bool {
    is_matching_tag(tag, tag_filters)
}

/// Returns true when `tag` matches any pattern in `ignore_tags`.
pub fn is_ignored_tag(tag: &str, ignore_tags: &[String]) -> bool {
    is_matching_tag(tag, ignore_tags)
}

/// Resolves the package column for an owner id using [`State::uids_map`] or [`State::pids_map`].
///
/// Empty `owner` yields an empty string; unknown owners become `UNKNOWN(owner)`.
pub fn owner_display_package(owner: &str, state: &State) -> String {
    if owner.is_empty() {
        return String::default();
    }

    let map = if state.uids_map.contains_key(owner) {
        &state.uids_map
    } else {
        &state.pids_map
    };

    map.get(owner)
        .cloned()
        .unwrap_or_else(|| format!("UNKNOWN({owner})"))
}

/// Package name for display: explicit `entry.package` or [`owner_display_package`].
pub fn entry_display_package(entry: &LogEntry, state: &State) -> String {
    if !entry.package.is_empty() {
        return entry.package.clone();
    }

    owner_display_package(&entry.owner, state)
}

/// Alias for [`entry_display_package`] used when applying package filters.
pub fn resolve_entry_package(entry: &LogEntry, state: &State) -> String {
    entry_display_package(entry, state)
}

/// Returns true when the resolved package display string matches `package` filter semantics.
///
/// Supports `unknown`, `unknown(owner)`, and substring match (ASCII case-insensitive).
pub fn package_name_contains(entry: &LogEntry, state: &State, package: &str) -> bool {
    let display = entry_display_package(entry, state);
    let filter = package.trim();

    if filter.eq_ignore_ascii_case("unknown") {
        return display.starts_with("UNKNOWN(");
    }

    if let Some(owner) = unknown_owner_filter(filter) {
        return display.eq_ignore_ascii_case(&format!("UNKNOWN({owner})"));
    }

    display
        .to_ascii_lowercase()
        .contains(&filter.to_ascii_lowercase())
}

/// Parses `unknown(...)` filter syntax and returns the inner owner token.
fn unknown_owner_filter(filter: &str) -> Option<String> {
    let lower = filter.to_ascii_lowercase();
    let rest = lower.strip_prefix("unknown(")?;
    let owner = rest.strip_suffix(')')?;
    if owner.is_empty() {
        return None;
    }
    Some(owner.to_string())
}
