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

//! The "copy log entry" menu.
//!
//! In select mode the user can open a small dialog that previews the selected
//! log entry and offers to copy individual fields (message, tag, PID, UID) or
//! the entire rendered entry to the system clipboard. This module defines the
//! available options, builds the text to copy, performs the (blocking)
//! clipboard write on a worker thread and renders the dialog.

#![deny(clippy::unwrap_used)]

use crate::CliArgs;
use crate::LogEntry;
use crate::LogEntryKind;
use crate::State;
use crate::render_entry_lines;
use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use strip_ansi_escapes::strip_str;
use terminal_clipboard::set_string;

use super::app::StatusUpdate;
use super::border::render_dialog;
use super::border::render_labeled_panel;
use super::palette::PaletteSearch;
use super::theme;

/// Virtual render width used when copying an entire entry.
///
/// Large enough that no line is ever wrapped or truncated in the copied text.
const COPY_WIDTH: i16 = 10_000;
/// Width (in characters) of the key column in each option row.
const COPY_KEYS_WIDTH: usize = 4;

/// Shortcut hints shown in the bottom border of the copy dialog.
const COPY_HINTS: &[(&str, &str)] = &[("↑↓", " navigate"), ("enter", " copy"), ("esc", " close")];

/// What part of a log entry should be copied to the clipboard.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CopyAction {
    /// The log message text, exactly as captured (or the banner text for
    /// process start/death entries).
    Message,
    /// The log tag.
    Tag,
    /// The process ID.
    Pid,
    /// The user ID.
    Uid,
    /// The entry exactly as it is rendered in the log view (ANSI stripped),
    /// including all of its lines.
    EntireEntry,
}

/// One selectable row of the copy dialog.
pub struct CopyOption {
    /// Single-key shortcut that triggers this option directly.
    pub key: char,
    /// Human-readable label shown in the menu (e.g. `"copy message"`).
    pub label: &'static str,
    /// Short noun used in status messages (e.g. `"message"` in `copied message`).
    pub feedback: &'static str,
    /// The action performed when this option is chosen.
    pub action: CopyAction,
}

/// Lists the copy options that make sense for the current configuration.
///
/// Message and entire-entry options are always present. The tag, PID and UID
/// options only appear when the corresponding column is visible
/// (`tag_width > 0`, `show_pid`, `show_uid`).
///
/// # Arguments
///
/// * `args` - The active CLI arguments, used to detect visible columns.
///
/// # Returns
///
/// The options in menu order.
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

/// Maps a pressed key to the copy action it triggers.
///
/// # Arguments
///
/// * `key` - The character typed by the user.
/// * `args` - The active CLI arguments (determines which options exist).
///
/// # Returns
///
/// The matching [`CopyAction`], or `None` if no available option uses `key`.
pub fn copy_action_for_key(key: char, args: &CliArgs) -> Option<CopyAction> {
    available_copy_options(args)
        .into_iter()
        .find(|option| option.key == key)
        .map(|option| option.action)
}

/// Returns the short noun describing what `action` copies.
///
/// # Arguments
///
/// * `action` - The action being performed.
/// * `args` - The active CLI arguments (determines which options exist).
///
/// # Returns
///
/// The option's feedback noun (e.g. `"tag"`), or `"text"` if the action is not
/// currently available.
pub fn copy_action_feedback(action: CopyAction, args: &CliArgs) -> &'static str {
    available_copy_options(args)
        .into_iter()
        .find(|option| option.action == action)
        .map_or("text", |option| option.feedback)
}

/// Writes `text` to the system clipboard without blocking the async runtime.
///
/// The clipboard call itself is synchronous, so it is moved onto Tokio's
/// blocking thread pool.
///
/// # Arguments
///
/// * `text` - The text to place on the clipboard.
///
/// # Returns
///
/// `Ok(())` on success, or `Err` with a human-readable message if the
/// clipboard rejected the text or the blocking task was cancelled.
pub async fn copy_to_clipboard(text: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || set_string(&text).map_err(|err| err.to_string()))
        .await
        .map_err(|_| "copy cancelled".to_string())?
}

/// Copies `text` to the clipboard and reports the outcome to the UI.
///
/// Sends a [`StatusUpdate::Message`] (`copied <feedback>` or
/// `copy failed: <error>`) followed by [`StatusUpdate::CopyFinished`] so the
/// UI can clear its "copy in progress" flag.
///
/// # Arguments
///
/// * `status_tx` - Channel used to report the result to the application.
/// * `text` - The text to copy.
/// * `feedback` - Noun inserted into the success message (e.g. `"message"`).
pub async fn run_copy(
    status_tx: tokio::sync::mpsc::UnboundedSender<StatusUpdate>,
    text: String,
    feedback: &'static str,
) {
    let message = match copy_to_clipboard(text).await {
        Ok(()) => format!("copied {feedback}"),
        Err(err) => format!("copy failed: {err}"),
    };

    let _ = status_tx.send(StatusUpdate::Message(message));
    let _ = status_tx.send(StatusUpdate::CopyFinished);
}

/// Computes the outer dialog height for a given entry preview and option count.
///
/// # Arguments
///
/// * `preview_lines` - Number of lines in the entry preview.
/// * `option_count` - Number of selectable options.
///
/// # Returns
///
/// The height in rows: the preview and option rows plus the dialog borders (2)
/// and the nested preview panel borders (2).
pub fn copy_dialog_height(preview_lines: usize, option_count: usize) -> u16 {
    // dialog borders (2) + preview panel borders (2) + preview content + options
    (preview_lines + option_count + 4) as u16
}

/// Renders `entry` into styled lines for the preview panel.
///
/// The entry is rendered with a cloned [`State`] (so the live state is not
/// mutated) and with `last_tag` reset so the tag is always shown. ANSI escape
/// codes are converted to ratatui styles, or stripped when `no_color` is set.
///
/// # Arguments
///
/// * `entry` - The log entry to preview.
/// * `state` - The current render state (PID/UID maps, tokens, ...).
/// * `args` - The active CLI arguments.
/// * `width` - Available width in characters for the rendered entry.
///
/// # Returns
///
/// One [`Line`] per rendered output line.
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

/// Produces the plain text that a given [`CopyAction`] puts on the clipboard.
///
/// Message text is copied verbatim — its case and formatting are never
/// altered. For process start/death entries the banner text is used instead
/// of the (empty) message.
///
/// # Arguments
///
/// * `entry` - The selected log entry.
/// * `state` - The current render state, used for [`CopyAction::EntireEntry`].
/// * `args` - The active CLI arguments, used for [`CopyAction::EntireEntry`].
/// * `action` - Which part of the entry to copy.
///
/// # Returns
///
/// The text to copy. For [`CopyAction::EntireEntry`] this is the rendered
/// entry with ANSI codes stripped and lines joined by `\n`.
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

/// Builds the display line for a single option row.
///
/// # Arguments
///
/// * `option` - The option to display.
/// * `selected` - Whether the row is currently highlighted.
///
/// # Returns
///
/// A line of the form `"  <key>  <label>"`, styled as selected or normal.
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

/// Renders the copy dialog: a preview of the entry above the option list.
///
/// The preview is clipped to the space left after reserving one row per
/// option. The selection in `search` is clamped to the number of options.
///
/// # Arguments
///
/// * `frame` - Frame to draw on.
/// * `search` - Selection state of the option list (mutated to clamp it).
/// * `entry` - The log entry being copied.
/// * `state` - The current render state, used to render the preview.
/// * `args` - The active CLI arguments.
/// * `area` - Outer rectangle of the dialog.
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

    // Match `render_labeled_panel` inner width (dialog inner minus panel borders).
    let preview_width = inner.width.saturating_sub(2) as i16;
    let all_preview_lines = entry_preview_lines(entry, state, args, preview_width);
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

/// Draws the nested `preview` panel containing the rendered entry lines.
///
/// # Arguments
///
/// * `frame` - Frame to draw on.
/// * `lines` - The (already clipped) preview lines.
/// * `area` - Outer rectangle of the preview panel, including its border.
fn render_entry_preview(frame: &mut Frame, lines: Vec<Line<'_>>, area: Rect) {
    let inner = render_labeled_panel(frame, area, Some("preview"), false);
    frame.render_widget(
        Paragraph::new(lines).style(theme::app_background_style()),
        inner,
    );
}
