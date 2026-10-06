#![deny(clippy::unwrap_used)]

use std::collections::HashMap;

use crate::LogEntry;
use crate::LogLevel;
use crate::State;
use crate::controller::log_processor::is_matching_tag;

pub fn passes_package_ownership(
    owner: &str,
    package_map: &HashMap<String, String>,
    all: bool,
) -> bool {
    all || package_map.contains_key(owner)
}

pub fn passes_log_level(level: LogLevel, min_level: LogLevel) -> bool {
    level >= min_level
}

pub fn passes_tag_filter(tag: &str, tag_filters: &[String]) -> bool {
    is_matching_tag(tag, tag_filters)
}

pub fn is_ignored_tag(tag: &str, ignore_tags: &[String]) -> bool {
    is_matching_tag(tag, ignore_tags)
}

/// Package name as shown in the log columns (`com.example` or `UNKNOWN(owner)`).
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

pub fn entry_display_package(entry: &LogEntry, state: &State) -> String {
    if !entry.package.is_empty() {
        return entry.package.clone();
    }

    owner_display_package(&entry.owner, state)
}

pub fn resolve_entry_package(entry: &LogEntry, state: &State) -> String {
    entry_display_package(entry, state)
}

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

fn unknown_owner_filter(filter: &str) -> Option<String> {
    let lower = filter.to_ascii_lowercase();
    let rest = lower.strip_prefix("unknown(")?;
    let owner = rest.strip_suffix(')')?;
    if owner.is_empty() {
        return None;
    }
    Some(owner.to_string())
}
