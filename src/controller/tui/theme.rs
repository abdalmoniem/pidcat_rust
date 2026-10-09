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

//! Theme-aware colors and styles for the TUI.
//!
//! Every function in this module reads the currently active theme (see
//! [`active_theme`]) on each call, so a theme change is reflected the next
//! time the UI is drawn. The module exposes:
//!
//! - raw semantic colors ([`background`], [`text`], [`accent`], ...);
//! - ready-made [`Style`]s built from them ([`status_style`],
//!   [`heading_style`], ...);
//! - the file-explorer theme ([`explorer_theme`]);
//! - the contextual shortcut hints shown under the log table
//!   ([`main_shortcut_hints`]).

#![deny(clippy::unwrap_used)]

use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;

use tui_file_explorer::Theme as ExplorerTheme;

use crate::active_theme;

/// Returns the application background color of the active theme.
pub fn background() -> Color {
    active_theme().ui.background.into()
}

/// Returns the primary foreground (text) color of the active theme.
pub fn text() -> Color {
    active_theme().ui.text.into()
}

/// Returns the dimmed foreground color used for secondary text.
pub fn subtext() -> Color {
    active_theme().ui.subtext.into()
}

/// Returns the accent color used for borders, headings and highlights.
pub fn accent() -> Color {
    active_theme().ui.accent.into()
}

/// Returns the secondary accent color (e.g. for the device serial).
pub fn secondary() -> Color {
    active_theme().ui.secondary.into()
}

/// Returns the color signalling success (e.g. status feedback messages).
pub fn success() -> Color {
    active_theme().ui.success.into()
}

/// Returns the color signalling a warning or idle state.
pub fn warning() -> Color {
    active_theme().ui.warning.into()
}

/// Returns the color signalling an error.
pub fn error() -> Color {
    active_theme().ui.error.into()
}

/// Returns the color used to highlight search/filter matches.
pub fn matched() -> Color {
    active_theme().ui.r#match.into()
}

/// Returns the color used to display key bindings.
pub fn keys() -> Color {
    active_theme().ui.keys.into()
}

/// Returns the background color of selected rows.
pub fn selection() -> Color {
    active_theme().ui.selection.into()
}

/// Builds the color theme for the embedded file explorer widget.
///
/// Maps the active application theme onto the explorer's own palette so the
/// open/save dialogs match the rest of the UI.
///
/// # Returns
///
/// An [`ExplorerTheme`] derived from the current theme colors.
pub fn explorer_theme() -> ExplorerTheme {
    ExplorerTheme {
        brand: accent(),
        accent: accent(),
        dir: accent(),
        sel_bg: selection(),
        success: success(),
        match_file: matched(),
        dim: subtext(),
        fg: text(),
        bg: background(),
    }
}

/// Style of the status bar line (the plain application background style).
pub fn status_style() -> Style {
    app_background_style()
}

/// Style that highlights the selected log entry (background only).
pub fn selection_line_style() -> Style {
    Style::default().bg(selection())
}

/// Style of the status indicator while live logging is running.
///
/// Rendered as bold background-on-accent text.
pub fn status_accent() -> Style {
    Style::default()
        .fg(background())
        .bg(accent())
        .add_modifier(Modifier::BOLD)
}

/// Style of the status indicator while paused, idle or reading a static source.
pub fn status_idle() -> Style {
    Style::default().fg(warning())
}

/// Style for error text.
pub fn error_style() -> Style {
    Style::default().fg(error())
}

/// Style for section headings (bold accent).
pub fn heading_style() -> Style {
    Style::default().fg(accent()).add_modifier(Modifier::BOLD)
}

/// Style for de-emphasised text such as separators and empty-state messages.
pub fn dim_style() -> Style {
    Style::default().fg(subtext())
}

/// Style for placeholder text in empty input fields.
pub fn placeholder_style() -> Style {
    Style::default().fg(subtext()).add_modifier(Modifier::DIM)
}

/// Base style of every drawn area: theme background with theme text color.
pub fn app_background_style() -> Style {
    Style::default().bg(background()).fg(text())
}

/// Style of the currently highlighted row in lists and menus.
///
/// Bold accent text on the selection background.
pub fn help_selected_style() -> Style {
    Style::default()
        .fg(accent())
        .bg(selection())
        .add_modifier(Modifier::BOLD)
}

/// Style of the key-binding column in help-like lists.
///
/// # Arguments
///
/// * `executable` - Whether the row can be executed; non-executable rows are
///   dimmed.
pub fn help_keys_style(executable: bool) -> Style {
    if executable {
        Style::default().fg(keys())
    } else {
        Style::default().fg(keys()).add_modifier(Modifier::DIM)
    }
}

/// Style of the description column in help-like lists.
///
/// # Arguments
///
/// * `executable` - Whether the row can be executed; non-executable rows use
///   the subtext color.
pub fn help_desc_style(executable: bool) -> Style {
    if executable {
        Style::default().fg(text())
    } else {
        Style::default().fg(subtext())
    }
}

/// Style for plain hint text (e.g. "press ... to refresh").
pub fn hint_style() -> Style {
    Style::default().fg(subtext())
}

/// Style for the key part of a shortcut hint (bold accent).
pub fn hint_key_style() -> Style {
    Style::default().fg(accent()).add_modifier(Modifier::BOLD)
}

/// The "export" shortcut hint, appended when entries can be exported.
const EXPORT_HINT: (&str, &str) = ("^s", " export");

/// Returns the shortcut hints shown in the log table's bottom border.
///
/// The set depends on the interaction mode: select mode offers copy and
/// selection movement, normal mode offers scrolling and the device/file
/// pickers. When `can_export` is set the export hint is inserted just before
/// the trailing `pause` and `quit` hints.
///
/// # Arguments
///
/// * `select_mode` - Whether the log view is in select mode.
/// * `can_export` - Whether there are entries that could be exported.
///
/// # Returns
///
/// `(key, description)` pairs in display order. Descriptions start with a
/// space so they can be appended directly after the key.
pub fn main_shortcut_hints(
    select_mode: bool,
    can_export: bool,
) -> Vec<(&'static str, &'static str)> {
    let mut hints = if select_mode {
        vec![
            ("↑↓", " select"),
            ("y", " copy"),
            ("v", " normal"),
            ("/", " filter"),
            ("?", " commands"),
            ("p", " pause"),
            ("q", " quit"),
        ]
    } else {
        vec![
            ("↑↓", " scroll"),
            ("v", " select"),
            ("/", " filter"),
            ("?", " commands"),
            ("d", " device"),
            ("o", " file"),
            ("l", " clear"),
            ("p", " pause"),
            ("q", " quit"),
        ]
    };

    if can_export {
        let before_pause = hints.len() - 2;
        hints.insert(before_pause, EXPORT_HINT);
    }

    hints
}
