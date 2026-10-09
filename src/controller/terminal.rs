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

//! Process-wide bookkeeping for undoing terminal changes made by the TUI.
//!
//! The TUI switches the terminal into an alternate screen / raw mode, may enable mouse capture,
//! and may force colored output. Those changes must be reverted on every exit path, including
//! fatal errors raised from arbitrary code such as
//! `exit_with_error`. Each change is therefore
//! recorded in a global flag via a `register_*` function, and `restore_tui_terminal` reverts
//! exactly the changes that were registered.

use std::io::stdout;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use colored::control::unset_override;
use crossterm::ExecutableCommand;
use crossterm::event::DisableMouseCapture;

/// Set once the ratatui terminal (raw mode and alternate screen) has been initialized and still
/// needs to be restored.
static TUI_TERMINAL_INITIALIZED: AtomicBool = AtomicBool::new(false);
/// Set while mouse capture is enabled and needs to be disabled on restore.
static TUI_MOUSE_CAPTURE: AtomicBool = AtomicBool::new(false);
/// Set while the global `colored` color override is forced and needs to be unset on restore.
static TUI_COLOR_OVERRIDE: AtomicBool = AtomicBool::new(false);

/// Records that mouse capture was enabled, so [`restore_tui_terminal`] disables it again.
pub fn register_tui_mouse_capture() {
    TUI_MOUSE_CAPTURE.store(true, Ordering::Relaxed);
}

/// Records that the global `colored` color override was set, so [`restore_tui_terminal`] removes
/// it again.
pub fn register_tui_color_override() {
    TUI_COLOR_OVERRIDE.store(true, Ordering::Relaxed);
}

/// Records that the ratatui terminal (raw mode and alternate screen) was initialized, so
/// [`restore_tui_terminal`] restores the original terminal state.
pub fn register_tui_terminal() {
    TUI_TERMINAL_INITIALIZED.store(true, Ordering::Relaxed);
}

/// Undo TUI terminal changes (same cleanup as a normal quit).
///
/// Reverts, in order, only the changes previously registered through
/// `register_tui_terminal`, `register_tui_mouse_capture`, and
/// `register_tui_color_override`. Each flag is cleared as it is handled, so the function is
/// idempotent and safe to call multiple times or when the TUI was never started.
pub fn restore_tui_terminal() {
    if TUI_TERMINAL_INITIALIZED.swap(false, Ordering::Relaxed) {
        ratatui::restore();
    }

    if TUI_MOUSE_CAPTURE.swap(false, Ordering::Relaxed) {
        let _ = stdout().execute(DisableMouseCapture);
    }

    if TUI_COLOR_OVERRIDE.swap(false, Ordering::Relaxed) {
        unset_override();
    }
}
