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

//! Core domain types for pidcatrs.
//!
//! This module groups data structures and pure helpers used across the CLI, log processor,
//! and TUI: ADB device metadata, parsed log lines, filtering rules, ANSI tokenization, and
//! shared runtime [`State`].

/// ADB-attached device identity and connection state.
pub mod adb_device;
/// Parsed `adb devices` connection states.
pub mod adb_state;
/// Splitting log text into plain segments and CSI escape sequences.
pub mod ansi;
/// Positions of ANSI codes within visible (non-escape) text.
pub mod ansi_segment;
/// Command-line argument definitions (re-exported at crate root).
pub mod cli_args;
/// Log-line filtering helpers (package, tag, level, ownership).
pub mod filter;
/// Parsed Android log record and lifecycle banner kinds.
pub mod log_entry;
/// Regex-based log line layout configuration.
pub mod log_format;
/// Android log priority letters and CLI parsing.
pub mod log_level;
/// Child process or stdin stream supplying raw log bytes.
pub mod log_source;
/// [`Option`] implementation of [`ValueOrPanic`](value_unwrap::ValueOrPanic).
pub mod option_unwrap;
/// [`Result`] implementation of [`ValueOrPanic`](value_unwrap::ValueOrPanic).
pub mod result_unwrap;
/// Mutable maps and display state while tailing logcat.
pub mod state;
/// Timestamp parsing, validation, and column formatting.
pub mod timestamp;
/// Interactive TUI filter expression parsing and matching.
pub mod tui_filter;
/// Custom unwrap helpers that panic with styled messages.
pub mod value_unwrap;
