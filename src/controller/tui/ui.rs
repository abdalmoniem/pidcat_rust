#![deny(clippy::unwrap_used)]

use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Margin;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarOrientation;
use ratatui::widgets::ScrollbarState;
use tui_file_explorer::render_themed;

use super::app::FileDialogMode;
use super::app::Overlay;
use super::app::SourceMode;
use super::app::TuiApp;
use super::border::render_dialog;
use super::border::render_labeled_panel;
use super::border::render_log_table_panel;
use super::copy::render_copy_menu;
use super::device_picker::device_state_label;
use super::device_picker::render_device_picker;
use super::export::export_format_dialog_height;
use super::export::render_export_format_menu;
use super::help::HELP_CATALOG;
use super::help::HelpRow;
use super::help::build_help_rows;
use super::palette::input_field_line;
use super::palette::render_search_field;
use super::theme;

use crate::controller::util::format_usize_separated;

pub fn render(frame: &mut Frame, app: &mut TuiApp) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(frame.area());

    render_filter_bar(frame, app, chunks[0usize]);
    refresh_log_view(app, chunks[2usize]);
    render_status_bar(frame, app, chunks[1usize]);
    render_log_table(frame, app, chunks[2usize]);

    match app.overlay {
        Overlay::DevicePicker => {
            let area = centered_rect(72, 78, frame.area());
            render_device_picker(frame, &app.devices, &mut app.device_palette, area);
        }
        Overlay::FileDialog => {
            let area = centered_rect(80, 75, frame.area());
            render_file_explorer_overlay(frame, app, area);
        }
        Overlay::Help => {
            let area = centered_rect(72, 78, frame.area());
            render_help(frame, app, area);
        }
        Overlay::CopyMenu => {
            if let Some(entry) = app.selected_log_entry().cloned() {
                let frame_area = frame.area();
                let dialog_width = frame_area
                    .width
                    .saturating_mul(72)
                    .saturating_div(100)
                    .max(40);
                let preview_width = dialog_width.saturating_sub(4) as i16;
                let preview_lines =
                    super::copy::entry_preview_lines(&entry, &app.state, &app.args, preview_width)
                        .len();
                let option_count = super::copy::available_copy_options(&app.args).len();
                let dialog_height = super::copy::copy_dialog_height(preview_lines, option_count);
                let area = centered_rect_size(
                    dialog_width,
                    dialog_height.min(frame_area.height.saturating_sub(2)),
                    frame_area,
                );
                render_copy_menu(
                    frame,
                    &mut app.copy_palette,
                    &entry,
                    &app.state,
                    &app.args,
                    area,
                );
            }
        }
        Overlay::ExportFormat => {
            let area = centered_rect_size(
                EXPORT_FORMAT_DIALOG_WIDTH,
                export_format_dialog_height(),
                frame.area(),
            );
            render_export_format_menu(frame, &mut app.export_format_palette, area);
        }
        Overlay::None => {}
    }
}

const EXPORT_FORMAT_DIALOG_WIDTH: u16 = 44;

const FILTER_PLACEHOLDER: &str = "e.g. package:com.example tag:ActivityManager level:debug";

fn render_filter_bar(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let inner = render_labeled_panel(frame, area, Some("filter"), app.filter_focused);
    let visible_width = inner.width as usize;
    let (text, scroll_chars) = input_field_line(
        &app.filter_input,
        FILTER_PLACEHOLDER,
        app.filter_cursor,
        visible_width.max(1),
    );

    frame.render_widget(
        Paragraph::new(text).style(theme::app_background_style()),
        inner,
    );

    if app.filter_focused {
        let cursor_x = inner.x.saturating_add(
            app.filter_cursor
                .saturating_sub(scroll_chars)
                .min(visible_width.saturating_sub(1)) as u16,
        );
        frame.set_cursor_position((cursor_x, inner.y));
    }
}

fn render_status_bar(frame: &mut Frame, app: &TuiApp, area: Rect) {
    let waiting_for_device =
        matches!(app.source_mode, SourceMode::Live) && app.selected_device.is_none();
    let status_icon = if app.paused {
        "○ paused"
    } else if waiting_for_device {
        "○ idle"
    } else {
        match &app.source_mode {
            SourceMode::Live => "● running",
            SourceMode::Pipe => "● pipe",
            SourceMode::File(_) => "● file",
        }
    };
    let source_detail = match &app.source_mode {
        SourceMode::Live if waiting_for_device => Some("live (no device)"),
        SourceMode::Live => Some("live"),
        SourceMode::Pipe => None,
        SourceMode::File(path) => Some(path.as_str()),
    };
    let device_serial = app.selected_device.as_deref().unwrap_or("no device");
    let device_state = app
        .selected_device
        .as_deref()
        .and_then(|serial| device_state_label(&app.devices, serial));
    let live_running =
        matches!(app.source_mode, SourceMode::Live) && !waiting_for_device && !app.paused;
    let status_style = if live_running {
        theme::status_accent()
    } else {
        theme::status_idle()
    };

    let mut spans = vec![Span::styled(status_icon, status_style)];
    if let Some(detail) = source_detail {
        spans.push(Span::styled("  │  ", theme::dim_style()));
        spans.push(Span::styled(detail, Style::default().fg(theme::ACCENT)));
    }
    spans.push(Span::styled("  │  ", theme::dim_style()));
    spans.push(Span::styled(
        device_serial,
        Style::default().fg(theme::MAUVE),
    ));
    if let Some(state) = device_state {
        spans.push(Span::styled(
            format!(" ({state})"),
            Style::default().fg(theme::SUBTEXT),
        ));
    }
    spans.extend([
        Span::styled("  │  ", theme::dim_style()),
        Span::styled(
            format!(
                "{}/{} entries",
                format_usize_separated(app.shown_entry_count),
                format_usize_separated(app.total_entry_count),
            ),
            Style::default().fg(theme::SUBTEXT),
        ),
    ]);
    if let Some(feedback) = &app.status_feedback {
        spans.push(Span::styled("  │  ", theme::dim_style()));
        spans.push(Span::styled(
            feedback.clone(),
            Style::default().fg(theme::GREEN),
        ));
    }

    let line = Line::from(spans);

    frame.render_widget(Paragraph::new(line).style(theme::status_style()), area);
}

fn log_panel_inner_dims(area: Rect) -> (i16, usize) {
    (
        area.width.saturating_sub(2) as i16,
        area.height.saturating_sub(2) as usize,
    )
}

fn refresh_log_view(app: &mut TuiApp, log_area: Rect) {
    use super::display_cache::DISPLAY_BUILD_BUDGET;

    let (width, viewport_lines) = log_panel_inner_dims(log_area);
    app.viewport_lines = viewport_lines;

    let build_budget = if app.paused { 0 } else { DISPLAY_BUILD_BUDGET };

    app.display_cache.ensure(
        &app.filtered_indices,
        &app.entries,
        &app.state,
        &app.args,
        width,
        app.filter_generation,
        build_budget,
    );

    let rendered_lines = app.display_cache.rendered_line_count();
    app.shown_entry_count = app.display_cache.rendered_entry_count();
    app.total_entry_count = app.entries.len();

    let max_scroll = rendered_lines.saturating_sub(viewport_lines);
    app.max_scroll = max_scroll;

    if !app.paused && app.auto_scroll {
        if app.select_mode {
            app.selected_filtered_index = app.filtered_indices.len().saturating_sub(1);
        }
        app.scroll_offset = max_scroll;
    } else {
        app.scroll_offset = app.scroll_offset.min(max_scroll);
        if app.select_mode {
            app.clamp_selected_filtered_index();
            app.ensure_selection_visible();
        }
    }
}

fn render_log_table(frame: &mut Frame, app: &mut TuiApp, area: Rect) {
    let columns = crate::tui_log_border_columns(&app.args);
    let hints = theme::main_shortcut_hints(app.select_mode, app.has_exportable_entries());
    let inner = render_log_table_panel(frame, area, &columns, &hints);
    let viewport_lines = inner.height as usize;
    let total_lines = app.display_cache.rendered_line_count();

    let visible_lines: Vec<Line> = app
        .display_cache
        .lines()
        .iter()
        .enumerate()
        .skip(app.scroll_offset)
        .take(viewport_lines)
        .map(|(line_index, line)| {
            if !app.select_mode || app.overlay != Overlay::None {
                return line.clone();
            }

            let selected = app
                .display_cache
                .line_filtered_index_at(line_index)
                .is_some_and(|index| index == app.selected_filtered_index);
            if selected {
                highlight_selected_line(line)
            } else {
                line.clone()
            }
        })
        .collect();

    frame.render_widget(
        Paragraph::new(visible_lines).style(theme::app_background_style()),
        inner,
    );

    if total_lines > viewport_lines {
        render_log_scrollbar(frame, area, app.scroll_offset, total_lines, viewport_lines);
    }
}

fn highlight_selected_line(line: &Line<'static>) -> Line<'static> {
    let bg = theme::selection_line_style();
    Line::from(
        line.spans
            .iter()
            .map(|span| {
                Span::styled(
                    span.content.clone(),
                    span.style
                        .fg(span.style.fg.unwrap_or(theme::TEXT))
                        .bg(theme::SEL_BG),
                )
            })
            .collect::<Vec<_>>(),
    )
    .style(bg)
}

fn render_log_scrollbar(
    frame: &mut Frame,
    area: Rect,
    scroll_offset: usize,
    total_lines: usize,
    viewport_lines: usize,
) {
    let inner = Rect {
        x: area.x.saturating_add(1),
        y: area.y.saturating_add(1),
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let scrollbar_area = Rect {
        x: inner.x.saturating_add(inner.width.saturating_sub(1)),
        y: inner.y.saturating_add(1),
        width: 1,
        height: inner.height.saturating_sub(1).max(1),
    };

    let max_scroll = total_lines.saturating_sub(viewport_lines);
    let scrollbar_position =
        map_scroll_offset_to_scrollbar_position(scroll_offset, max_scroll, total_lines);

    let mut scrollbar_state = ScrollbarState::new(total_lines)
        .position(scrollbar_position)
        .viewport_content_length(viewport_lines);

    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight),
        scrollbar_area,
        &mut scrollbar_state,
    );
}

/// Ratatui's scrollbar uses `position = content_length - 1` for the bottom of the track,
/// while our log view uses `scroll_offset = total_lines - viewport_lines` as the last page.
fn map_scroll_offset_to_scrollbar_position(
    scroll_offset: usize,
    max_scroll: usize,
    content_length: usize,
) -> usize {
    if content_length <= 1 {
        return 0;
    }

    let last_position = content_length - 1;

    if max_scroll == 0 {
        return 0;
    }

    if scroll_offset >= max_scroll {
        return last_position;
    }

    scroll_offset * last_position / max_scroll
}

const HELP_KEYS_WIDTH: usize = 22;

const HELP_HINTS: &[(&str, &str)] = &[
    ("↑↓", " navigate"),
    ("enter", " execute"),
    ("esc", " close"),
];

fn render_help(frame: &mut Frame, app: &mut TuiApp, area: Rect) {
    let inner = render_dialog(frame, area, "command palette", HELP_HINTS, false);

    let chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(4)]).split(inner);

    render_search_field(
        frame,
        chunks[0usize],
        &app.help_palette,
        "search commands or keybindings...",
    );
    render_help_list(frame, app, chunks[1usize]);
}

fn render_help_list(frame: &mut Frame, app: &mut TuiApp, area: Rect) {
    let rows = build_help_rows(&app.help_palette.query);
    let list_height = area.height as usize;

    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "no matching commands",
                theme::dim_style(),
            )))
            .style(theme::app_background_style()),
            area,
        );
        return;
    }

    app.help_palette.clamp_selection(rows.len());
    app.help_palette.ensure_list_top_visible(list_height);

    let visible_lines: Vec<Line> = rows
        .iter()
        .enumerate()
        .skip(app.help_palette.list_top)
        .take(list_height)
        .map(|(row_index, row)| help_row_line(row, row_index == app.help_palette.selected))
        .collect();

    frame.render_widget(
        Paragraph::new(visible_lines).style(theme::app_background_style()),
        area,
    );
}

fn help_row_line(row: &HelpRow, selected: bool) -> Line<'static> {
    match row {
        HelpRow::Section(section) => Line::from(Span::styled(*section, theme::heading_style())),
        HelpRow::Entry(index) => {
            let entry = &HELP_CATALOG[*index];
            let executable = entry.action.is_some();
            let (keys_style, desc_style) = if selected {
                (theme::help_selected_style(), theme::help_selected_style())
            } else {
                (
                    theme::help_keys_style(executable),
                    theme::help_desc_style(executable),
                )
            };

            Line::from(vec![
                Span::styled("  ", keys_style),
                Span::styled(format!("{:<HELP_KEYS_WIDTH$}", entry.keys), keys_style),
                Span::styled(entry.description.to_string(), desc_style),
            ])
        }
    }
}

const FILE_OPEN_HINTS: &[(&str, &str)] = &[
    ("↑↓", " select"),
    ("tab", " complete"),
    ("enter", " open"),
    ("esc", " close"),
];

const FILE_SAVE_HINTS: &[(&str, &str)] = &[
    ("↑↓", " select"),
    ("tab", " complete"),
    ("enter", " save"),
    ("esc", " close"),
];

const FILE_PATH_PLACEHOLDER: &str = "type a path...";

fn render_file_explorer_overlay(frame: &mut Frame, app: &mut TuiApp, area: Rect) {
    let export_scope = app.export_scope();
    let Some(explorer) = &mut app.file_explorer else {
        return;
    };

    let (title, hints) = match app.file_dialog_mode {
        FileDialogMode::Open => ("open log file".to_string(), FILE_OPEN_HINTS),
        FileDialogMode::Save => (
            format!(
                "export {export_scope} entries as {}",
                app.export_format.label()
            ),
            FILE_SAVE_HINTS,
        ),
    };
    let inner = render_dialog(frame, area, &title, hints, false);
    let error_height = if app.file_open_error.is_some() { 3 } else { 0 };
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(error_height),
        Constraint::Min(0),
    ])
    .split(inner);

    let path_inner = render_labeled_panel(frame, chunks[0usize], Some("path"), true);
    render_search_field(
        frame,
        path_inner,
        &app.file_path_input,
        FILE_PATH_PLACEHOLDER,
    );

    if let Some(err) = app.file_open_error.as_deref() {
        let error_inner = render_labeled_panel(frame, chunks[1usize], Some("error"), false);
        frame.render_widget(
            Paragraph::new(err)
                .style(theme::app_background_style())
                .style(theme::error_style()),
            error_inner,
        );
    }

    render_themed(explorer, frame, chunks[2usize], &app.explorer_theme);
}

fn centered_rect_size(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1usize])[1usize]
        .inner(Margin::new(1, 1))
}

pub(super) fn line_from_ansi(text: &str) -> Line<'static> {
    use crate::model::ansi::AnsiToken;
    use crate::model::ansi::tokenize_ansi;

    let mut spans = Vec::default();
    let mut current = String::new();
    let mut style = Style::default();

    for token in tokenize_ansi(text) {
        match token {
            AnsiToken::Text(text) => current.push_str(&text),
            AnsiToken::Escape(code) => {
                if !current.is_empty() {
                    spans.push(Span::styled(std::mem::take(&mut current), style));
                }
                style = apply_ansi_code(style, &code);
            }
        }
    }

    if !current.is_empty() {
        spans.push(Span::styled(current, style));
    }

    Line::from(spans)
}

fn apply_ansi_code(mut base: Style, code: &str) -> Style {
    if !code.starts_with("\x1b[") {
        return base;
    }

    let inner = code.trim_start_matches("\x1b[").trim_end_matches('m');
    if inner.is_empty() {
        return base;
    }

    let parts: Vec<&str> = inner.split(';').collect();
    let mut index = 0usize;

    while index < parts.len() {
        if parts[index] == "0" {
            base = Style::default();
            index += 1;
            continue;
        }

        if parts[index] == "38" && index + 1 < parts.len() {
            if parts[index + 1] == "2" && index + 4 < parts.len() {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    parts[index + 2].parse::<u8>(),
                    parts[index + 3].parse::<u8>(),
                    parts[index + 4].parse::<u8>(),
                ) {
                    base = base.fg(Color::Rgb(r, g, b));
                }
                index += 5;
                continue;
            }
            if parts[index + 1] == "5" && index + 2 < parts.len() {
                if let Ok(color_index) = parts[index + 2].parse::<u8>() {
                    base = base.fg(ansi256_to_color(color_index));
                }
                index += 3;
                continue;
            }
        }

        if parts[index] == "48" && index + 1 < parts.len() {
            if parts[index + 1] == "2" && index + 4 < parts.len() {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    parts[index + 2].parse::<u8>(),
                    parts[index + 3].parse::<u8>(),
                    parts[index + 4].parse::<u8>(),
                ) {
                    base = base.bg(Color::Rgb(r, g, b));
                }
                index += 5;
                continue;
            }
            if parts[index + 1] == "5" && index + 2 < parts.len() {
                if let Ok(color_index) = parts[index + 2].parse::<u8>() {
                    base = base.bg(ansi256_to_color(color_index));
                }
                index += 3;
                continue;
            }
        }

        if let Ok(value) = parts[index].parse::<u16>() {
            base = apply_basic_sgr(base, value);
        }

        index += 1;
    }

    base
}

fn apply_basic_sgr(style: Style, code: u16) -> Style {
    match code {
        1 => style.add_modifier(Modifier::BOLD),
        22 => style.remove_modifier(Modifier::BOLD),
        30 => style.fg(Color::Black),
        31 => style.fg(Color::Red),
        32 => style.fg(Color::Green),
        33 => style.fg(Color::Yellow),
        34 => style.fg(Color::Blue),
        35 => style.fg(Color::Magenta),
        36 => style.fg(Color::Cyan),
        37 => style.fg(Color::White),
        39 => style.fg(Color::Reset),
        40 => style.bg(Color::Black),
        41 => style.bg(Color::Red),
        42 => style.bg(Color::Green),
        43 => style.bg(Color::Yellow),
        44 => style.bg(Color::Blue),
        45 => style.bg(Color::Magenta),
        46 => style.bg(Color::Cyan),
        47 => style.bg(Color::White),
        49 => style.bg(Color::Reset),
        90 => style.fg(Color::DarkGray),
        91 => style.fg(Color::LightRed),
        92 => style.fg(Color::LightGreen),
        93 => style.fg(Color::LightYellow),
        94 => style.fg(Color::LightBlue),
        95 => style.fg(Color::LightMagenta),
        96 => style.fg(Color::LightCyan),
        97 => style.fg(Color::White),
        100 => style.bg(Color::DarkGray),
        101 => style.bg(Color::LightRed),
        102 => style.bg(Color::LightGreen),
        103 => style.bg(Color::LightYellow),
        104 => style.bg(Color::LightBlue),
        105 => style.bg(Color::LightMagenta),
        106 => style.bg(Color::LightCyan),
        107 => style.bg(Color::White),
        _ => style,
    }
}

fn ansi256_to_color(index: u8) -> Color {
    match index {
        0..=15 => {
            let code = match index {
                0 => 30,
                1 => 31,
                2 => 32,
                3 => 33,
                4 => 34,
                5 => 35,
                6 => 36,
                7 => 37,
                8 => 90,
                9 => 91,
                10 => 92,
                11 => 93,
                12 => 94,
                13 => 95,
                14 => 96,
                _ => 97,
            };
            apply_basic_sgr(Style::default(), code)
                .fg
                .unwrap_or(Color::White)
        }
        232..=255 => {
            let gray = 8 + (index - 232) * 10;
            Color::Rgb(gray, gray, gray)
        }
        _ => {
            let index = index - 16;
            let r = index / 36;
            let g = (index / 6) % 6;
            let b = index % 6;
            let channel = |value: u8| {
                if value == 0 { 0 } else { 55 + value * 40 }
            };
            Color::Rgb(channel(r), channel(g), channel(b))
        }
    }
}
