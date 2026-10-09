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

//! Custom box-drawing borders for panels and dialogs.
//!
//! Unlike ratatui's built-in `Block` borders, the borders drawn here embed
//! labels in the top edge (`╭─┐label┌───╮`) and keyboard-shortcut hints in the
//! bottom edge (`╰─┘key desc└───╯`). The module provides:
//!
//! - pure string builders for the top/bottom edges, which are easy to test;
//! - widget helpers that draw a complete frame and return the inner [`Rect`]
//!   available for content.

#![deny(clippy::unwrap_used)]

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;

use super::theme;

/// Truncates `label` so it occupies at most `width` terminal cells.
///
/// Width is measured in `char`s. When truncation is needed and there is room
/// (`width > 1`), the last visible cell is replaced by an ellipsis (`…`).
///
/// # Arguments
///
/// * `label` - The text to fit.
/// * `width` - Maximum number of characters allowed in the result.
///
/// # Returns
///
/// `label` unchanged if it already fits, otherwise a truncated copy that is
/// exactly `width` characters long (or fewer if `label` is shorter).
fn fit_label(label: &str, width: usize) -> String {
    let char_count = label.chars().count();
    if char_count <= width {
        return label.to_string();
    }
    if width <= 1 {
        return label.chars().take(width).collect();
    }
    format!("{}…", label.chars().take(width - 1).collect::<String>())
}

/// Builds a top border divided into several labeled segments.
///
/// Each segment is rendered as `label` followed by `┌`, a run of `─` padding
/// and a closing `┐` (except for the final segment, which flows straight
/// into the trailing fill). The line always starts with `╭─┐` and ends with
/// `╮`.
///
/// If `total_width` is smaller than 4 or `segments` is empty, the function
/// falls back to an empty labeled border (see [`build_labeled_top_border`]).
///
/// # Arguments
///
/// * `total_width` - Total width of the border line in characters.
/// * `segments` - `(label, width)` pairs. A segment is widened if needed so
///   its label fits (label length + 2).
///
/// # Returns
///
/// The border line as a plain, unstyled string.
fn build_segmented_border(total_width: usize, segments: &[(String, usize)]) -> String {
    if total_width < 4 || segments.is_empty() {
        return build_labeled_top_border(total_width, "");
    }

    let mut out = "╭─┐".to_string();

    for (index, (label, width)) in segments.iter().enumerate() {
        let is_last = index + 1 == segments.len();
        let segment_width = (*width).max(label.chars().count() + 2);
        let fitted = fit_label(label, segment_width.saturating_sub(2));

        out.push_str(&fitted);
        out.push('┌');

        if is_last {
            let dashes = segment_width.saturating_sub(fitted.chars().count() + 1);
            out.push_str(&"─".repeat(dashes));
            break;
        }

        let dashes = segment_width.saturating_sub(fitted.chars().count() + 2);
        out.push_str(&"─".repeat(dashes));
        out.push('┐');
    }

    let used = out.chars().count();
    let remaining = total_width.saturating_sub(used + 1);
    if remaining > 0 {
        out.push_str(&"─".repeat(remaining));
    }
    out.push('╮');

    out
}

/// Builds a top border carrying a single label.
///
/// The result looks like `╭─┐label┌──────────────────────────────╮`.
///
/// # Arguments
///
/// * `width` - Total width of the border line in characters.
/// * `label` - Text embedded after the opening `╭─┐`. Pass `""` for an
///   unlabeled border.
///
/// # Returns
///
/// The border line as a plain string. For `width < 4` there is no room for
/// corners, so a run of `─` of length `width` is returned instead.
pub fn build_labeled_top_border(width: usize, label: &str) -> String {
    if width < 4 {
        return "─".repeat(width);
    }

    let prefix = format!("╭─┐{label}┌");
    let fill = width.saturating_sub(prefix.chars().count() + 1);
    format!("{prefix}{}╮", "─".repeat(fill))
}

/// Builds a plain bottom border without any embedded text.
///
/// The result looks like `╰──────────────────────────────╯`.
///
/// # Arguments
///
/// * `width` - Total width of the border line in characters.
///
/// # Returns
///
/// The border line, or an empty string when `width < 2` (no room for both
/// corners).
pub fn build_simple_bottom_border(width: usize) -> String {
    if width < 2 {
        return String::new();
    }
    format!("╰{}╯", "─".repeat(width.saturating_sub(2)))
}

/// Builds the top border of the log table, with one segment per column.
///
/// The result looks like
/// `╭─┐PID┌─┐UID┌─┐PACKAGE┌…┐TAG┌…┐L┌──────────────╮`, so the column headers
/// sit directly above the corresponding log columns.
///
/// # Arguments
///
/// * `total_width` - Total width of the border line in characters.
/// * `columns` - `(header, width)` pairs describing each visible column.
///
/// # Returns
///
/// The border line as a plain string.
pub fn build_log_table_top_border(total_width: usize, columns: &[(String, usize)]) -> String {
    build_segmented_border(total_width, columns)
}

/// Builds a styled bottom border whose segments are keyboard-shortcut hints.
///
/// Each hint is drawn as `<key><description>└──┘`, with the key portion in
/// the theme's hint-key style and the description in the normal text style.
/// Long hints are truncated with [`fit_label`]. The line starts with `╰─┘`
/// and ends with `╯`; unused width is filled with `─`.
///
/// # Arguments
///
/// * `total_width` - Total width of the border line in characters.
/// * `hints` - `(key, description)` pairs, e.g. `("esc", " close")`. Include
///   a leading space in the description to separate it from the key.
///
/// # Returns
///
/// A styled [`Line`]. If `total_width < 4` or `hints` is empty, a plain
/// bottom border is returned instead.
fn hint_bottom_border_line(total_width: usize, hints: &[(&str, &str)]) -> Line<'static> {
    let box_style = border_style(false);
    let key_style = theme::hint_key_style();
    let desc_style = Style::default().fg(theme::text()).bg(theme::background());

    if total_width < 4 || hints.is_empty() {
        return Line::from(Span::styled(
            build_simple_bottom_border(total_width),
            box_style,
        ));
    }

    let mut spans = vec![Span::styled("╰─┘".to_string(), box_style)];

    for (index, (key, text)) in hints.iter().enumerate() {
        let is_last = index + 1 == hints.len();
        let label = format!("{key}{text}");
        let segment_width = label.chars().count() + 2;
        let fitted = fit_label(&label, segment_width.saturating_sub(2));
        let key_len = key.chars().count();
        let fitted_chars: Vec<char> = fitted.chars().collect();

        if fitted_chars.len() <= key_len {
            spans.push(Span::styled(fitted.clone(), key_style));
        } else {
            let key_part: String = fitted_chars.iter().take(key_len).collect();
            let desc_part: String = fitted_chars.iter().skip(key_len).collect();
            spans.push(Span::styled(key_part, key_style));
            spans.push(Span::styled(desc_part, desc_style));
        }

        spans.push(Span::styled("└".to_string(), box_style));

        if is_last {
            let dashes = segment_width.saturating_sub(fitted.chars().count() + 1);
            if dashes > 0 {
                spans.push(Span::styled("─".repeat(dashes), box_style));
            }
        } else {
            let dashes = segment_width.saturating_sub(fitted.chars().count() + 2);
            if dashes > 0 {
                spans.push(Span::styled("─".repeat(dashes), box_style));
            }
            spans.push(Span::styled("┘".to_string(), box_style));
        }
    }

    let used: usize = spans.iter().map(|span| span.content.chars().count()).sum();
    let remaining = total_width.saturating_sub(used + 1);
    if remaining > 0 {
        spans.push(Span::styled("─".repeat(remaining), box_style));
    }
    spans.push(Span::styled("╯".to_string(), box_style));

    Line::from(spans)
}

/// The content of a panel's bottom edge.
enum BottomBorder<'a> {
    /// A pre-built, uniformly styled line (e.g. from [`build_simple_bottom_border`]).
    Plain(&'a str),
    /// A list of `(key, description)` shortcut hints rendered with
    /// [`hint_bottom_border_line`].
    Hints(&'a [(&'a str, &'a str)]),
}

/// Returns the style used to draw box-drawing characters.
///
/// # Arguments
///
/// * `focused` - Whether the panel currently has input focus; focused panels
///   are drawn in bold.
///
/// # Returns
///
/// The accent-colored style, with [`Modifier::BOLD`] added when focused.
fn border_style(focused: bool) -> Style {
    if focused {
        Style::default()
            .fg(theme::accent())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::accent())
    }
}

/// Renders a single-line, uniformly styled border string into `area`.
///
/// # Arguments
///
/// * `frame` - Frame to draw on.
/// * `area` - Single-row region that receives the line.
/// * `line` - The border text to draw.
/// * `style` - Style applied to the text.
fn render_border_line(frame: &mut Frame, area: Rect, line: &str, style: Style) {
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(line.to_string(), style)))
            .style(theme::app_background_style()),
        area,
    );
}

/// Renders an already styled, multi-span border [`Line`] into `area`.
///
/// # Arguments
///
/// * `frame` - Frame to draw on.
/// * `area` - Single-row region that receives the line.
/// * `line` - The pre-styled line to draw.
fn render_styled_border_line(frame: &mut Frame, area: Rect, line: Line<'_>) {
    frame.render_widget(
        Paragraph::new(line).style(theme::app_background_style()),
        area,
    );
}

/// Draws the four edges of a bordered panel and returns its inner area.
///
/// The top edge is the supplied `top` string, the sides are `│` columns and
/// the bottom edge is chosen by `bottom`.
///
/// # Arguments
///
/// * `frame` - Frame to draw on.
/// * `area` - Outer rectangle of the panel, including the border.
/// * `top` - Pre-built top border string.
/// * `bottom` - Bottom border content.
/// * `focused` - Whether to draw the border in its focused (bold) style.
///
/// # Returns
///
/// The rectangle inside the border, available for content. If `area` is too
/// small to draw a border (`width < 4` or `height < 2`) nothing is drawn and
/// `area` is returned unchanged.
fn render_panel_frame(
    frame: &mut Frame,
    area: Rect,
    top: &str,
    bottom: BottomBorder<'_>,
    focused: bool,
) -> Rect {
    if area.width < 4 || area.height < 2 {
        return area;
    }

    let style = border_style(focused);

    render_border_line(
        frame,
        Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        },
        top,
        style,
    );

    if area.height > 2 {
        let side_height = area.height.saturating_sub(2);
        let side_lines: Vec<Line> = (0..side_height)
            .map(|_| Line::from(Span::styled("│", style)))
            .collect();
        frame.render_widget(
            Paragraph::new(side_lines.clone()).style(theme::app_background_style()),
            Rect {
                x: area.x,
                y: area.y.saturating_add(1),
                width: 1,
                height: side_height,
            },
        );
        frame.render_widget(
            Paragraph::new(side_lines).style(theme::app_background_style()),
            Rect {
                x: area.x.saturating_add(area.width.saturating_sub(1)),
                y: area.y.saturating_add(1),
                width: 1,
                height: side_height,
            },
        );
    }

    if area.height > 1 {
        let bottom_area = Rect {
            x: area.x,
            y: area.y.saturating_add(area.height.saturating_sub(1)),
            width: area.width,
            height: 1,
        };
        match bottom {
            BottomBorder::Plain(line) => render_border_line(frame, bottom_area, line, style),
            BottomBorder::Hints(hints) => {
                let line = hint_bottom_border_line(area.width as usize, hints);
                render_styled_border_line(frame, bottom_area, line);
            }
        }
    }

    Rect {
        x: area.x.saturating_add(1),
        y: area.y.saturating_add(1),
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}

/// Draws a labeled panel (`filter`, nested `preview`, etc.).
///
/// # Arguments
///
/// * `frame` - Frame to draw on.
/// * `area` - Outer rectangle of the panel, including the border.
/// * `label` - Optional text embedded in the top edge.
/// * `focused` - Whether the panel has focus (drawn in bold when `true`).
///
/// # Returns
///
/// The inner rectangle available for the panel's content.
pub fn render_labeled_panel(
    frame: &mut Frame,
    area: Rect,
    label: Option<&str>,
    focused: bool,
) -> Rect {
    let width = area.width as usize;
    let top = build_labeled_top_border(width, label.unwrap_or(""));
    let bottom = build_simple_bottom_border(width);
    render_panel_frame(frame, area, &top, BottomBorder::Plain(&bottom), focused)
}

/// Draws the log table with column headers and shortcut hints in the borders.
///
/// # Arguments
///
/// * `frame` - Frame to draw on.
/// * `area` - Outer rectangle of the table, including the border.
/// * `columns` - `(header, width)` pairs placed in the top edge.
/// * `footer_hints` - `(key, description)` pairs placed in the bottom edge.
///
/// # Returns
///
/// The inner rectangle available for log lines.
pub fn render_log_table_panel(
    frame: &mut Frame,
    area: Rect,
    columns: &[(String, usize)],
    footer_hints: &[(&str, &str)],
) -> Rect {
    let width = area.width as usize;
    let top = build_log_table_top_border(width, columns);
    render_panel_frame(frame, area, &top, BottomBorder::Hints(footer_hints), false)
}

/// Draws a dialog overlay with a title and hint segments in the bottom border.
///
/// The dialog area is first cleared and filled with the application
/// background so the content underneath does not bleed through.
///
/// # Arguments
///
/// * `frame` - Frame to draw on.
/// * `area` - Outer rectangle of the dialog, including the border.
/// * `title` - Text embedded in the top edge.
/// * `footer_hints` - `(key, description)` pairs placed in the bottom edge.
/// * `focused` - Whether to draw the border in its focused (bold) style.
///
/// # Returns
///
/// The inner rectangle available for the dialog's content.
pub fn render_dialog(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    footer_hints: &[(&str, &str)],
    focused: bool,
) -> Rect {
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new("").style(theme::app_background_style()),
        area,
    );

    let width = area.width as usize;
    let top = build_labeled_top_border(width, title);
    render_panel_frame(
        frame,
        area,
        &top,
        BottomBorder::Hints(footer_hints),
        focused,
    )
}
