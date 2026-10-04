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

pub fn resolve_entry_package(entry: &LogEntry, state: &State) -> String {
    if !entry.package.is_empty() {
        return entry.package.clone();
    }

    if state.uids_map.contains_key(&entry.owner) {
        return state
            .uids_map
            .get(&entry.owner)
            .cloned()
            .unwrap_or_default();
    }

    state
        .pids_map
        .get(&entry.owner)
        .cloned()
        .unwrap_or_default()
}

pub fn package_name_contains(entry: &LogEntry, state: &State, package: &str) -> bool {
    resolve_entry_package(entry, state).contains(package)
}
