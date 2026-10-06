#![deny(clippy::unwrap_used)]

use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;

use tui_file_explorer::Theme as ExplorerTheme;

use crate::active_theme;

pub fn background() -> Color {
    active_theme().ui.background.into()
}

pub fn text() -> Color {
    active_theme().ui.text.into()
}

pub fn subtext() -> Color {
    active_theme().ui.subtext.into()
}

pub fn accent() -> Color {
    active_theme().ui.accent.into()
}

pub fn secondary() -> Color {
    active_theme().ui.secondary.into()
}

pub fn success() -> Color {
    active_theme().ui.success.into()
}

pub fn warning() -> Color {
    active_theme().ui.warning.into()
}

pub fn error() -> Color {
    active_theme().ui.error.into()
}

pub fn matched() -> Color {
    active_theme().ui.r#match.into()
}

pub fn keys() -> Color {
    active_theme().ui.keys.into()
}

pub fn selection() -> Color {
    active_theme().ui.selection.into()
}

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

pub fn status_style() -> Style {
    app_background_style()
}

pub fn selection_line_style() -> Style {
    Style::default().bg(selection())
}

pub fn status_accent() -> Style {
    Style::default()
        .fg(background())
        .bg(accent())
        .add_modifier(Modifier::BOLD)
}

pub fn status_idle() -> Style {
    Style::default().fg(warning())
}

pub fn error_style() -> Style {
    Style::default().fg(error())
}

pub fn heading_style() -> Style {
    Style::default().fg(accent()).add_modifier(Modifier::BOLD)
}

pub fn dim_style() -> Style {
    Style::default().fg(subtext())
}

pub fn placeholder_style() -> Style {
    Style::default().fg(subtext()).add_modifier(Modifier::DIM)
}

pub fn app_background_style() -> Style {
    Style::default().bg(background()).fg(text())
}

pub fn help_selected_style() -> Style {
    Style::default()
        .fg(accent())
        .bg(selection())
        .add_modifier(Modifier::BOLD)
}

pub fn help_keys_style(executable: bool) -> Style {
    if executable {
        Style::default().fg(keys())
    } else {
        Style::default().fg(keys()).add_modifier(Modifier::DIM)
    }
}

pub fn help_desc_style(executable: bool) -> Style {
    if executable {
        Style::default().fg(text())
    } else {
        Style::default().fg(subtext())
    }
}

pub fn hint_style() -> Style {
    Style::default().fg(subtext())
}

pub fn hint_key_style() -> Style {
    Style::default().fg(accent()).add_modifier(Modifier::BOLD)
}

const EXPORT_HINT: (&str, &str) = ("^s", " export");

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
