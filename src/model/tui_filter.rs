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

//! Filter bar expressions for the interactive log viewer.

#![deny(clippy::unwrap_used)]

use crate::LogEntry;
use crate::LogEntryKind;
use crate::LogLevel;
use crate::State;
use crate::controller::util::split_csv_values;
use crate::package_name_contains;
use crate::passes_log_level;
use crate::passes_tag_filter;

/// One constraint parsed from the TUI filter input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TuiFilter {
    /// Match log lines whose PID equals this value.
    Pid(String),
    /// Match lines whose UID equals this value or maps in [`State::uids_map`].
    Uid(String),
    /// Match log tag (supports glob rules via [`passes_tag_filter`]).
    Tag(String),
    /// Match resolved package display name ([`package_name_contains`]).
    Package(String),
    /// Require at least this [`LogLevel`] (combined with other level filters via minimum).
    Level(LogLevel),
    /// Case-insensitive substring in message or banner text.
    Keyword(String),
}

/// Ordered list of active TUI filters parsed from user input.
#[derive(Clone, Debug, Default)]
pub struct TuiFilterSet {
    /// Parsed filter clauses in input order.
    filters: Vec<TuiFilter>,
}

impl TuiFilterSet {
    /// Creates an empty filter set (no constraints).
    pub fn new() -> Self {
        Self {
            filters: Vec::default(),
        }
    }

    /// Borrowed view of parsed filters.
    pub fn filters(&self) -> &[TuiFilter] {
        &self.filters
    }

    /// Returns true when no filters are configured.
    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }

    /// Parses a filter bar string into typed [`TuiFilter`] values.
    ///
    /// Supports `pid:`, `uid:`, `tag:`, `package:`, and `level:` prefixes (CSV where noted),
    /// plus bare tokens as keywords. Quoted segments preserve embedded spaces.
    pub fn parse(input: &str) -> Self {
        let mut filters = Vec::default();

        for token in tokenize_filter_input(input) {
            if let Some((prefix, value)) = token.split_once(':') {
                let value = value.trim();
                if value.is_empty() {
                    continue;
                }

                match prefix.trim().to_ascii_lowercase().as_str() {
                    "pid" => filters.push(TuiFilter::Pid(value.to_string())),
                    "uid" => filters.push(TuiFilter::Uid(value.to_string())),
                    "tag" => {
                        filters.extend(split_csv_values(value).into_iter().map(TuiFilter::Tag))
                    }
                    "package" => {
                        filters.extend(split_csv_values(value).into_iter().map(TuiFilter::Package))
                    }
                    "level" => filters.extend(
                        split_csv_values(value)
                            .into_iter()
                            .filter_map(|part| LogLevel::parse_name(&part).map(TuiFilter::Level)),
                    ),
                    _ => {}
                }
            } else if !token.is_empty() {
                filters.push(TuiFilter::Keyword(token));
            }
        }

        Self { filters }
    }

    /// Returns true when `entry` satisfies all active filters and minimum log level.
    pub fn matches(&self, entry: &LogEntry, state: &State) -> bool {
        if !passes_log_level(entry.level, self.min_log_level(state)) {
            return false;
        }

        if matches!(
            entry.kind,
            LogEntryKind::ProcessStart | LogEntryKind::ProcessDeath
        ) {
            return self.matches_lifecycle_banner(entry, state);
        }

        if self.filters.is_empty() {
            return true;
        }

        let mut tags = Vec::default();
        let mut packages = Vec::default();
        let mut pids = Vec::default();
        let mut uids = Vec::default();
        let mut keywords = Vec::default();

        for filter in &self.filters {
            match filter {
                TuiFilter::Tag(tag) => tags.push(tag.clone()),
                TuiFilter::Package(package) => packages.push(package.clone()),
                TuiFilter::Pid(pid) => pids.push(pid.clone()),
                TuiFilter::Uid(uid) => uids.push(uid.clone()),
                TuiFilter::Level(_) => {}
                TuiFilter::Keyword(keyword) => keywords.push(keyword.clone()),
            }
        }

        if !tags.is_empty() && !passes_tag_filter(&entry.tag, &tags) {
            return false;
        }

        if !packages.is_empty()
            && !packages
                .iter()
                .any(|package| package_name_contains(entry, state, package))
        {
            return false;
        }

        if !pids.is_empty() && !pids.contains(&entry.pid) {
            return false;
        }

        if !uids.is_empty()
            && !uids
                .iter()
                .any(|uid| entry.uid == *uid || state.uids_map.contains_key(uid))
        {
            return false;
        }

        for keyword in &keywords {
            if !entry
                .message
                .to_ascii_lowercase()
                .contains(&keyword.to_ascii_lowercase())
            {
                return false;
            }
        }

        true
    }

    /// Process start/death banners ignore tag filters (plain-mode parity) but
    /// still respect package/pid/uid constraints and keyword search on banner text.
    fn matches_lifecycle_banner(&self, entry: &LogEntry, state: &State) -> bool {
        if self.filters.is_empty() {
            return true;
        }

        let mut packages = Vec::default();
        let mut pids = Vec::default();
        let mut uids = Vec::default();
        let mut keywords = Vec::default();

        for filter in &self.filters {
            match filter {
                TuiFilter::Package(package) => packages.push(package.clone()),
                TuiFilter::Pid(pid) => pids.push(pid.clone()),
                TuiFilter::Uid(uid) => uids.push(uid.clone()),
                TuiFilter::Keyword(keyword) => keywords.push(keyword.clone()),
                TuiFilter::Tag(_) | TuiFilter::Level(_) => {}
            }
        }

        if !packages.is_empty()
            && !packages
                .iter()
                .any(|package| package_name_contains(entry, state, package))
        {
            return false;
        }

        if !pids.is_empty() && !pids.contains(&entry.pid) {
            return false;
        }

        if !uids.is_empty()
            && !uids
                .iter()
                .any(|uid| entry.uid == *uid || state.uids_map.contains_key(uid))
        {
            return false;
        }

        for keyword in &keywords {
            if !entry
                .banner_text
                .to_ascii_lowercase()
                .contains(&keyword.to_ascii_lowercase())
            {
                return false;
            }
        }

        true
    }

    /// Effective minimum level: explicit `level:` filters (most verbose wins) or [`State::log_level`].
    fn min_log_level(&self, state: &State) -> LogLevel {
        let level_filters = self
            .filters
            .iter()
            .filter_map(|filter| match filter {
                TuiFilter::Level(level) => Some(*level),
                _ => None,
            })
            .collect::<Vec<_>>();

        if level_filters.is_empty() {
            state.log_level
        } else {
            level_filters.into_iter().min().unwrap_or(state.log_level)
        }
    }
}

/// Splits `input` on whitespace outside of double-quoted regions.
fn tokenize_filter_input(input: &str) -> Vec<String> {
    let mut tokens = Vec::default();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in input.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                current.push(ch);
            }
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(normalize_token(&current));
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        tokens.push(normalize_token(&current));
    }

    tokens
}

/// Trims a token and strips surrounding `"` quotes when present.
fn normalize_token(token: &str) -> String {
    let trimmed = token.trim();

    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2usize {
        trimmed[1..trimmed.len() - 1usize].to_string()
    } else {
        trimmed.to_string()
    }
}
