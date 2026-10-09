use std::io::stdout;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use colored::control::unset_override;
use crossterm::ExecutableCommand;
use crossterm::event::DisableMouseCapture;

static TUI_TERMINAL_INITIALIZED: AtomicBool = AtomicBool::new(false);
static TUI_MOUSE_CAPTURE: AtomicBool = AtomicBool::new(false);
static TUI_COLOR_OVERRIDE: AtomicBool = AtomicBool::new(false);

pub fn register_tui_mouse_capture() {
    TUI_MOUSE_CAPTURE.store(true, Ordering::Relaxed);
}

pub fn register_tui_color_override() {
    TUI_COLOR_OVERRIDE.store(true, Ordering::Relaxed);
}

pub fn register_tui_terminal() {
    TUI_TERMINAL_INITIALIZED.store(true, Ordering::Relaxed);
}

/// Undo TUI terminal changes (same cleanup as a normal quit).
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
