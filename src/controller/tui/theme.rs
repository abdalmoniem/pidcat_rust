#![deny(clippy::unwrap_used)]

use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Borders;

use tui_file_explorer::Theme as ExplorerTheme;

// Gruber Darker palette — https://github.com/rexim/gruber-darker-theme
pub const BG: Color = Color::Rgb(24, 24, 24); // #181818 bg0
pub const TEXT: Color = Color::Rgb(228, 228, 239); // #e4e4ef fg0
pub const SUBTEXT: Color = Color::Rgb(168, 153, 132); // #a89984 fg3
pub const YELLOW: Color = Color::Rgb(255, 221, 51); // #ffdd33 yellow0
pub const ORANGE: Color = Color::Rgb(204, 140, 60); // #cc8c3c brown0
pub const GREEN: Color = Color::Rgb(115, 201, 54); // #73c936 green0
pub const BLUE: Color = Color::Rgb(150, 166, 200); // #96a6c8 niagara0
pub const RED: Color = Color::Rgb(244, 56, 65); // #f43841 red0
pub const AQUA: Color = Color::Rgb(142, 192, 124); // #8ec07c aqua1
pub const CYAN: Color = Color::Rgb(140, 208, 211); // unselected palette entries
pub const SEL_BG: Color = Color::Rgb(64, 64, 64); // #404040 bg5

/// Primary accent — yellow highlights throughout the UI.
pub const ACCENT: Color = YELLOW;

/// Secondary accent — soft blue for labels and metadata.
pub const MAUVE: Color = BLUE;

pub fn explorer_theme() -> ExplorerTheme {
    ExplorerTheme {
        brand: YELLOW,
        accent: YELLOW,
        dir: YELLOW,
        sel_bg: SEL_BG,
        success: GREEN,
        match_file: AQUA,
        dim: SUBTEXT,
        fg: TEXT,
        bg: BG,
    }
}

pub fn panel_block(title: &str) -> Block<'_> {
    panel_block_with_title(Line::from(format!(" {title} ")))
}

pub fn panel_block_with_title(title: Line<'_>) -> Block<'_> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(YELLOW))
        .style(Style::default().bg(BG).fg(TEXT))
}

pub fn focused_panel_block_with_title(title: Line<'_>) -> Block<'_> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(YELLOW).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(BG).fg(TEXT))
}

pub fn overlay_block(title: &str) -> Block<'_> {
    Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(YELLOW))
        .style(Style::default().bg(BG).fg(TEXT))
}

pub fn status_style() -> Style {
    app_background_style()
}

pub fn selection_line_style() -> Style {
    Style::default().bg(SEL_BG)
}

pub fn status_accent() -> Style {
    Style::default()
        .fg(BG)
        .bg(YELLOW)
        .add_modifier(Modifier::BOLD)
}

pub fn status_idle() -> Style {
    Style::default().fg(ORANGE)
}

pub fn error_style() -> Style {
    Style::default().fg(RED)
}

pub fn heading_style() -> Style {
    Style::default().fg(YELLOW).add_modifier(Modifier::BOLD)
}

pub fn dim_style() -> Style {
    Style::default().fg(SUBTEXT)
}

pub fn placeholder_style() -> Style {
    Style::default().fg(SUBTEXT).add_modifier(Modifier::DIM)
}

pub fn app_background_style() -> Style {
    Style::default().bg(BG).fg(TEXT)
}

pub fn help_selected_style() -> Style {
    Style::default()
        .fg(YELLOW)
        .bg(SEL_BG)
        .add_modifier(Modifier::BOLD)
}

pub fn help_keys_style(executable: bool) -> Style {
    if executable {
        Style::default().fg(CYAN)
    } else {
        Style::default().fg(CYAN).add_modifier(Modifier::DIM)
    }
}

pub fn help_desc_style(executable: bool) -> Style {
    if executable {
        Style::default().fg(TEXT)
    } else {
        Style::default().fg(SUBTEXT)
    }
}

pub fn hint_style() -> Style {
    Style::default().fg(SUBTEXT)
}

pub fn hint_key_style() -> Style {
    Style::default().fg(YELLOW).add_modifier(Modifier::BOLD)
}

pub fn dialog_footer_line(entries: &[(&str, &str)]) -> Line<'static> {
    hint_line(entries, "  │  ")
}

pub fn main_shortcuts_line(select_mode: bool) -> Line<'static> {
    if select_mode {
        dialog_footer_line(&[
            ("↑↓", " select"),
            ("y", " copy"),
            ("v", " normal"),
            ("/", " filter"),
            ("?", " commands"),
            ("p", " pause"),
            ("q", " quit"),
        ])
    } else {
        dialog_footer_line(&[
            ("↑↓", " scroll"),
            ("v", " select"),
            ("/", " filter"),
            ("?", " commands"),
            ("d", " device"),
            ("o", " file"),
            ("l", " clear"),
            ("p", " pause"),
            ("q", " quit"),
        ])
    }
}

pub fn hint_line(entries: &[(&str, &str)], separator: &str) -> Line<'static> {
    let mut spans = Vec::default();

    for (index, (key, text)) in entries.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(separator.to_string(), hint_style()));
        }
        spans.push(Span::styled((*key).to_string(), hint_key_style()));
        spans.push(Span::styled((*text).to_string(), hint_style()));
    }

    Line::from(spans)
}
