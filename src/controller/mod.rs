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

//! Application controllers that drive pidcatrs at runtime.
//!
//! The controller layer sits between the passive `model` types (CLI arguments,
//! log entries, filters, session `State`) and the outside world. It is
//! responsible for talking to `adb`, reading and parsing logcat input, rendering entries to the
//! console or to files, and running either the plain streaming mode or the interactive TUI.
//!
//! # Submodules
//!
//! - `adb` — building and running `adb` commands (server, devices, processes, logcat).
//! - `log_processor` — parsing raw logcat lines into entries and rendering them, including
//!   column layout, wrapping, and TUI helpers.
//! - `plain` — the non-interactive streaming mode (`run_plain`) and its run flag.
//! - `setup` — CLI argument normalization, package resolution, and initial `State`
//!   construction shared by plain mode and the TUI.
//! - `terminal` — bookkeeping for restoring the terminal after the TUI exits or fails.
//! - `tui` — the interactive ratatui-based user interface.
//! - `util` — small shared helpers (colored output, paging, CSV splitting, line trimming).
//! - `writer` — the `Writer` output sink used by the renderers.

/// Running and parsing `adb` commands: server start-up, device discovery, process/UID lookup,
/// and spawning or clearing `logcat`.
pub mod adb;
/// Logcat line parsing, filtering hooks, and rendering (banners, headers, wrapping, ANSI
/// handling, and TUI layout helpers).
pub mod log_processor;
/// Plain (non-TUI) streaming mode that reads logcat from `adb` or stdin and prints it.
pub mod plain;
/// Start-up helpers shared by plain mode and the TUI: argument normalization, package
/// resolution, initial state, and ADB bootstrap.
pub mod setup;
/// Process-wide bookkeeping used to restore the terminal after the TUI.
pub mod terminal;
/// The interactive terminal user interface built on ratatui.
pub mod tui;
/// Small stateless utilities: colored messages, error exit, column layout, paging, CSV splitting,
/// and log-line trimming.
pub mod util;
/// Output sinks (console, file, in-memory buffer) used when rendering log entries.
pub mod writer;
