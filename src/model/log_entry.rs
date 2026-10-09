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

//! Structured representation of one displayed log line or lifecycle banner.

use chrono::DateTime;
use chrono::Local;

use crate::LogLevel;

/// Distinguishes normal log lines from process lifecycle banners.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LogEntryKind {
    /// Standard logcat line with tag, level, and message.
    Normal,
    /// Banner emitted when a tracked process starts.
    ProcessStart,
    /// Banner emitted when a tracked process exits.
    ProcessDeath,
}

/// Fully parsed log record ready for filtering, coloring, and export.
#[derive(Clone, Debug)]
pub struct LogEntry {
    /// Whether this row is a normal line or a lifecycle banner.
    pub kind: LogEntryKind,
    /// Parsed or fallback timestamp for the row.
    pub timestamp: DateTime<Local>,
    /// Process id from the log line, if present.
    pub pid: String,
    /// User id from the log line, if present.
    pub uid: String,
    /// Raw owner field (PID or UID string) before package resolution.
    pub owner: String,
    /// Resolved package name when known from the line itself.
    pub package: String,
    /// Android log tag.
    pub tag: String,
    /// Priority letter mapped to [`LogLevel`].
    pub level: LogLevel,
    /// Message body (may include embedded ANSI).
    pub message: String,
    /// Text shown for process start/death banners (may differ from `message`).
    pub banner_text: String,
    /// Original input line before normalization.
    pub raw: String,
}
