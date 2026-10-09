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

//! Session state accumulated while parsing and displaying logcat.

use std::collections::HashMap;

use crate::LogEntry;
use crate::LogLevel;

/// Running context for PID/UID resolution, colors, filters, and multiline log assembly.
#[derive(Clone, Debug)]
pub struct State {
    /// Maps process id → package name for lines that only carry a PID owner.
    pub pids_map: HashMap<String, String>,
    /// Maps user id → package name when ownership is expressed as UID.
    pub uids_map: HashMap<String, String>,
    /// Most recently seen log tag (used when continuation lines omit the tag).
    pub last_tag: Option<String>,
    /// Primary application PID when filtering to a single process.
    pub app_pid: Option<String>,
    /// Minimum log level configured for output (plain mode and TUI default).
    pub log_level: LogLevel,
    /// Human-readable process names from CLI that should map to packages.
    pub named_processes: Vec<String>,
    /// Package prefixes that match any process under that package tree.
    pub catchall_packages: Vec<String>,
    /// Rotating palette assigned to newly seen log tags or tokens.
    pub token_colors: Vec<colored::Color>,
    /// Stable tag/token → color assignments for the current session.
    pub known_tokens: HashMap<String, colored::Color>,
    /// Partial log entry waiting for continuation lines (wrapped messages).
    pub long_pending: Option<LogEntry>,
}
