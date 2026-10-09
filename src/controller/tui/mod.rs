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

//! Interactive terminal user interface (TUI) for `pidcatrs`.
//!
//! This module hosts the full-screen, [`ratatui`]-based logcat viewer. It
//! wires together a handful of cooperating submodules:
//!
//! - `app` — the `TuiApp` state machine, key/mouse handling and
//!   the main event loop ([`run_tui`]).
//! - `ui` — pure rendering of the application state into a [`ratatui::Frame`].
//! - `log_ingest` — background tasks that read log lines from `adb logcat`,
//!   a file or a pipe and feed them to the application.
//! - `display_cache` — an incremental cache of rendered (ANSI-parsed) lines.
//! - `palette`, `help`, `device_picker`, `copy`, `export`, `file_source` —
//!   the dialogs/overlays and their supporting helpers.
//! - `border`, `theme` — shared drawing primitives and theme-aware styles.
//!
//! The only item exported to the rest of the crate is [`run_tui`].

#![deny(clippy::unwrap_used)]

/// Application state, input handling and the TUI event loop.
mod app;
/// Box-drawing border builders and panel/dialog renderers.
mod border;
/// "Copy log entry" menu: options, clipboard access and rendering.
mod copy;
/// Device selection dialog (`adb devices`) helpers and renderer.
mod device_picker;
/// Incremental cache of rendered log lines for the main log view.
mod display_cache;
/// "Export entries" format menu and file writers.
mod export;
/// Path helpers and validation for the open/save file dialogs.
mod file_source;
/// Command palette / help catalog and its row model.
mod help;
/// Background log ingestion (live `adb logcat`, file and pipe sources).
mod log_ingest;
/// Shared palette-style search input and list-selection state.
mod palette;
/// Theme-aware colors, styles and shortcut hint tables.
mod theme;
/// Frame rendering: filter bar, status bar, log table and overlays.
mod ui;

/// Starts the interactive TUI and blocks until the user quits.
///
/// Re-exported from `app` so the rest of the crate has a single entry point
/// into the terminal UI.
pub use app::run_tui;
