#![deny(clippy::unwrap_used)]

use arboard::Clipboard;
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use strip_ansi_escapes::strip_str;

use crate::CliArgs;
use crate::LogEntry;
use crate::LogEntryKind;
use crate::State;
use crate::render_entry_lines;

use super::border::render_dialog;
use super::border::render_labeled_panel;
use super::palette::PaletteSearch;
use super::theme;

const COPY_WIDTH: i16 = 10_000;
const COPY_KEYS_WIDTH: usize = 4;

const COPY_HINTS: &[(&str, &str)] = &[("↑↓", " navigate"), ("enter", " copy"), ("esc", " close")];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CopyAction {
    Message,
    Tag,
    Pid,
    Uid,
    EntireEntry,
}

pub struct CopyOption {
    pub key: char,
    pub label: &'static str,
    pub feedback: &'static str,
    pub action: CopyAction,
}

pub fn available_copy_options(args: &CliArgs) -> Vec<CopyOption> {
    let mut options = vec![CopyOption {
        key: 'm',
        label: "copy message",
        feedback: "message",
        action: CopyAction::Message,
    }];

    if args.tag_width > 0 {
        options.push(CopyOption {
            key: 't',
            label: "copy tag",
            feedback: "tag",
            action: CopyAction::Tag,
        });
    }

    if args.show_pid {
        options.push(CopyOption {
            key: 'p',
            label: "copy pid",
            feedback: "pid",
            action: CopyAction::Pid,
        });
    }

    if args.show_uid {
        options.push(CopyOption {
            key: 'u',
            label: "copy uid",
            feedback: "uid",
            action: CopyAction::Uid,
        });
    }

    options.push(CopyOption {
        key: 'e',
        label: "copy entire entry",
        feedback: "entry",
        action: CopyAction::EntireEntry,
    });

    options
}

pub fn copy_action_for_key(key: char, args: &CliArgs) -> Option<CopyAction> {
    available_copy_options(args)
        .into_iter()
        .find(|option| option.key == key)
        .map(|option| option.action)
}

pub fn copy_action_feedback(action: CopyAction, args: &CliArgs) -> &'static str {
    available_copy_options(args)
        .into_iter()
        .find(|option| option.action == action)
        .map_or("text", |option| option.feedback)
}

pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    Clipboard::new()
        .map_err(|err| err.to_string())?
        .set_text(text.to_owned())
        .map_err(|err| err.to_string())
}

/// Outer dialog height for a given entry preview line count and option count.
pub fn copy_dialog_height(preview_lines: usize, option_count: usize) -> u16 {
    // dialog borders (2) + preview panel borders (2) + preview content + options
    (preview_lines + option_count + 4) as u16
}

pub fn entry_preview_lines(
    entry: &LogEntry,
    state: &State,
    args: &CliArgs,
    width: i16,
) -> Vec<Line<'static>> {
    let mut render_state = state.clone();
    render_state.last_tag = None;

    render_entry_lines(entry, &mut render_state, args, width)
        .into_iter()
        .map(|text| {
            if args.no_color {
                Line::from(strip_str(&text))
            } else {
                super::ui::line_from_ansi(&text)
            }
        })
        .collect()
}

pub fn copy_text_for_entry(
    entry: &LogEntry,
    state: &State,
    args: &CliArgs,
    action: CopyAction,
) -> String {
    match action {
        CopyAction::Message => match entry.kind {
            // Preserve log text exactly as captured — never alter case or formatting.
            LogEntryKind::Normal => entry.message.clone(),
            LogEntryKind::ProcessStart | LogEntryKind::ProcessDeath => entry.banner_text.clone(),
        },
        CopyAction::Tag => entry.tag.clone(),
        CopyAction::Pid => entry.pid.clone(),
        CopyAction::Uid => entry.uid.clone(),
        CopyAction::EntireEntry => {
            let mut render_state = state.clone();
            render_state.last_tag = None;
            render_entry_lines(entry, &mut render_state, args, COPY_WIDTH)
                .into_iter()
                .map(|line| strip_str(&line))
                .collect::<Vec<_>>()
                .join("\n")
        }
    }
}

fn copy_row_line(option: &CopyOption, selected: bool) -> Line<'static> {
    let (keys_style, desc_style) = if selected {
        (theme::help_selected_style(), theme::help_selected_style())
    } else {
        (theme::help_keys_style(true), theme::help_desc_style(true))
    };

    Line::from(vec![
        Span::styled("  ", keys_style),
        Span::styled(format!("{:<COPY_KEYS_WIDTH$}", option.key), keys_style),
        Span::styled(option.label, desc_style),
    ])
}

pub fn render_copy_menu(
    frame: &mut Frame,
    search: &mut PaletteSearch,
    entry: &LogEntry,
    state: &State,
    args: &CliArgs,
    area: Rect,
) {
    let options = available_copy_options(args);
    let option_count = options.len() as u16;

    let inner = render_dialog(frame, area, "copy log entry", COPY_HINTS, false);

    let all_preview_lines = entry_preview_lines(entry, state, args, inner.width as i16);
    let full_preview_lines = all_preview_lines.len() as u16;

    let preview_borders = 2;
    let max_preview_content = inner.height.saturating_sub(option_count + preview_borders);
    let preview_content_height = full_preview_lines.max(1).min(max_preview_content.max(1));

    let chunks = Layout::vertical([
        Constraint::Length(preview_content_height + preview_borders),
        Constraint::Length(option_count),
    ])
    .split(inner);

    let visible_preview: Vec<Line> = all_preview_lines
        .into_iter()
        .take(preview_content_height as usize)
        .collect();

    render_entry_preview(frame, visible_preview, chunks[0usize]);

    search.clamp_selection(options.len());

    let option_lines: Vec<Line> = options
        .iter()
        .enumerate()
        .map(|(index, option)| copy_row_line(option, index == search.selected))
        .collect();

    frame.render_widget(
        Paragraph::new(option_lines).style(theme::app_background_style()),
        chunks[1usize],
    );
}

fn render_entry_preview(frame: &mut Frame, lines: Vec<Line<'_>>, area: Rect) {
    let inner = render_labeled_panel(frame, area, Some("preview"), false);
    frame.render_widget(
        Paragraph::new(lines).style(theme::app_background_style()),
        inner,
    );
}
