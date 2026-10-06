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

/// `╭─┐label┌──────────────────────────────╮`
pub fn build_labeled_top_border(width: usize, label: &str) -> String {
    if width < 4 {
        return "─".repeat(width);
    }

    let prefix = format!("╭─┐{label}┌");
    let fill = width.saturating_sub(prefix.chars().count() + 1);
    format!("{prefix}{}╮", "─".repeat(fill))
}

/// Plain bottom: `╰──────────────────────────────╯`
pub fn build_simple_bottom_border(width: usize) -> String {
    if width < 2 {
        return String::new();
    }
    format!("╰{}╯", "─".repeat(width.saturating_sub(2)))
}

/// Log table top border: `╭─┐PID┌─┐UID┌─┐PACKAGE┌…┐TAG┌…┐L┌──────────────╮`
pub fn build_log_table_top_border(total_width: usize, columns: &[(String, usize)]) -> String {
    build_segmented_border(total_width, columns)
}

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

enum BottomBorder<'a> {
    Plain(&'a str),
    Hints(&'a [(&'a str, &'a str)]),
}

fn border_style(focused: bool) -> Style {
    if focused {
        Style::default()
            .fg(theme::accent())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::accent())
    }
}

fn render_border_line(frame: &mut Frame, area: Rect, line: &str, style: Style) {
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(line.to_string(), style)))
            .style(theme::app_background_style()),
        area,
    );
}

fn render_styled_border_line(frame: &mut Frame, area: Rect, line: Line<'_>) {
    frame.render_widget(
        Paragraph::new(line).style(theme::app_background_style()),
        area,
    );
}

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

/// Draw a labeled panel (`filter`, nested `preview`, etc.).
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

/// Draw the log table with column headers and shortcut hints in the borders.
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

/// Draw a dialog overlay with a title and hint segments in the bottom border.
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
