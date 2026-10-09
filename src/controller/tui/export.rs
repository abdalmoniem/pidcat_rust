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

//! Exporting the filtered log entries to a file.
//!
//! The user first picks an [`ExportFormat`] from a small menu, then chooses a
//! destination in the save dialog. [`run_export`] performs the actual write
//! on the async runtime and reports the result back to the UI through a
//! [`StatusUpdate`] channel.

#![deny(clippy::unwrap_used)]

use std::collections::VecDeque;
use std::path::Path;

use crate::CliArgs;
use crate::LogEntry;
use crate::State;
use crate::Writer;
use crate::render_entry;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use tokio::io::AsyncWriteExt;

use super::app::SourceMode;
use super::app::StatusUpdate;
use super::border::render_dialog;
use super::palette::PaletteSearch;
use super::theme;

/// Width (in characters) of the key column in each format row.
const EXPORT_FORMAT_KEYS_WIDTH: usize = 4;
/// Shortcut hints shown in the bottom border of the format menu.
const EXPORT_FORMAT_HINTS: &[(&str, &str)] =
    &[("↑↓", " navigate"), ("enter", " choose"), ("esc", " close")];

/// The on-disk representation used when exporting entries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportFormat {
    /// The output as rendered by `pidcatrs` itself (columns, tags, ...),
    /// written through the crate's file [`Writer`].
    Pidcatrs,
    /// The original raw `adb logcat` lines, one per entry.
    Adb,
}

/// One selectable row of the export format menu.
pub struct ExportFormatOption {
    /// Single-key shortcut that selects this format directly.
    pub key: char,
    /// Label displayed in the menu and used as file-name prefix.
    pub label: &'static str,
    /// The format selected by this option.
    pub format: ExportFormat,
}

/// All export formats, in menu order.
pub const EXPORT_FORMAT_OPTIONS: &[ExportFormatOption] = &[
    ExportFormatOption {
        key: 'p',
        label: env!("CARGO_PKG_NAME"),
        format: ExportFormat::Pidcatrs,
    },
    ExportFormatOption {
        key: 'a',
        label: "adb",
        format: ExportFormat::Adb,
    },
];

impl ExportFormat {
    /// Looks up the format bound to a shortcut key.
    ///
    /// # Arguments
    ///
    /// * `key` - The character typed by the user.
    ///
    /// # Returns
    ///
    /// The matching format, or `None` if no option uses `key`.
    pub fn for_key(key: char) -> Option<Self> {
        EXPORT_FORMAT_OPTIONS
            .iter()
            .find(|option| option.key == key)
            .map(|option| option.format)
    }

    /// Returns the display label of this format.
    ///
    /// # Returns
    ///
    /// The label (e.g. `"adb"`), also used as the default file-name prefix.
    pub fn label(self) -> &'static str {
        EXPORT_FORMAT_OPTIONS
            .iter()
            .find(|option| option.format == self)
            .map_or("", |option| option.label)
    }
}

/// Writes `entries` to `path` and reports the outcome to the UI.
///
/// Sends a [`StatusUpdate::Message`] (`exported N entries to <path>` or
/// `export failed: <error>`) followed by [`StatusUpdate::ExportFinished`] so
/// the UI can clear its "export in progress" flag.
///
/// # Arguments
///
/// * `status_tx` - Channel used to report the result to the application.
/// * `path` - Destination file path (created or truncated).
/// * `format` - Which representation to write.
/// * `entries` - The entries to export, in order.
/// * `render_state` - Render state cloned from the app; `last_tag` is reset
///   so the first entry always shows its tag.
/// * `args` - The active CLI arguments, used for rendering.
pub async fn run_export(
    status_tx: tokio::sync::mpsc::UnboundedSender<StatusUpdate>,
    path: String,
    format: ExportFormat,
    entries: VecDeque<LogEntry>,
    mut render_state: State,
    args: CliArgs,
) {
    render_state.last_tag = None;

    let message = match format {
        ExportFormat::Pidcatrs => write_rendered_entries(&path, &entries, &mut render_state, &args)
            .await
            .map(|count| {
                format!(
                    "exported {} entries to {}",
                    crate::controller::util::format_usize_separated(count),
                    path
                )
            }),
        ExportFormat::Adb => write_raw_entries(&path, &entries).await.map(|count| {
            format!(
                "exported {} entries to {}",
                crate::controller::util::format_usize_separated(count),
                path
            )
        }),
    }
    .unwrap_or_else(|err| format!("export failed: {err}"));

    let _ = status_tx.send(StatusUpdate::Message(message));
    let _ = status_tx.send(StatusUpdate::ExportFinished);
}

/// Writes entries to `path` rendered the same way as the log view.
///
/// # Arguments
///
/// * `path` - Destination file path (created or truncated).
/// * `entries` - The entries to render and write.
/// * `state` - Render state, updated as entries are rendered.
/// * `args` - The active CLI arguments.
///
/// # Returns
///
/// The number of entries written, or `Err` with a message if the file could
/// not be created.
async fn write_rendered_entries(
    path: &str,
    entries: &VecDeque<LogEntry>,
    state: &mut State,
    args: &CliArgs,
) -> Result<usize, String> {
    let file = tokio::fs::File::create(path)
        .await
        .map_err(|err| format!("cannot create {path}: {err}"))?;
    let std_file = file.into_std().await;
    let mut writer = Writer::new_file(std_file);

    for entry in entries {
        render_entry(entry, state, args, std::slice::from_mut(&mut writer));
    }

    writer.flush();
    Ok(entries.len())
}

/// Writes the original `adb logcat` line of every entry to `path`.
///
/// A trailing newline is added to lines that lack one.
///
/// # Arguments
///
/// * `path` - Destination file path (created or truncated).
/// * `entries` - The entries whose raw text is written.
///
/// # Returns
///
/// The number of entries written, or `Err` with a message if the file could
/// not be created or written.
async fn write_raw_entries(path: &str, entries: &VecDeque<LogEntry>) -> Result<usize, String> {
    let mut file = tokio::fs::File::create(path)
        .await
        .map_err(|err| format!("cannot create {path}: {err}"))?;
    let write_err = |err: std::io::Error| format!("cannot write {path}: {err}");

    for entry in entries {
        file.write_all(entry.raw.as_bytes())
            .await
            .map_err(write_err)?;
        if !entry.raw.ends_with('\n') {
            file.write_all(b"\n").await.map_err(write_err)?;
        }
    }

    file.flush().await.map_err(write_err)?;
    Ok(entries.len())
}

/// Suggests a file name for an export.
///
/// The name has the form `<format>_log_<origin>_<time>.log`, where `origin`
/// is the device serial (live), the source file's stem (file) or `pipe`, and
/// `time` is the local time formatted as `%I%M%S%.3f%P`.
///
/// # Arguments
///
/// * `format` - The chosen export format (its label prefixes the name).
/// * `source` - Where the entries came from.
/// * `device` - The selected device serial, if any (used for live sources).
///
/// # Returns
///
/// The suggested file name, without any directory.
pub fn default_export_file_name(
    format: ExportFormat,
    source: &SourceMode,
    device: Option<&str>,
) -> String {
    let origin = match source {
        SourceMode::Live => device.unwrap_or("pipe").to_string(),
        SourceMode::File(path) => Path::new(path)
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_else(|| "pipe".to_string()),
        SourceMode::Pipe => "pipe".to_string(),
    };
    let time = chrono::Local::now().format("%I%M%S%.3f%P");
    format!("{}_log_{origin}_{time}.log", format.label())
}

/// Returns the outer height of the export format dialog.
///
/// # Returns
///
/// One row per format option plus the two border rows.
pub fn export_format_dialog_height() -> u16 {
    EXPORT_FORMAT_OPTIONS.len() as u16 + 2
}

/// Builds the display line for a single format row.
///
/// # Arguments
///
/// * `option` - The option to display.
/// * `selected` - Whether the row is currently highlighted.
///
/// # Returns
///
/// A line of the form `"  <key>  <label>"`, styled as selected or normal.
fn export_format_row_line(option: &ExportFormatOption, selected: bool) -> Line<'static> {
    let (keys_style, desc_style) = if selected {
        (theme::help_selected_style(), theme::help_selected_style())
    } else {
        (theme::help_keys_style(true), theme::help_desc_style(true))
    };

    Line::from(vec![
        Span::styled("  ", keys_style),
        Span::styled(
            format!("{:<EXPORT_FORMAT_KEYS_WIDTH$}", option.key),
            keys_style,
        ),
        Span::styled(option.label, desc_style),
    ])
}

/// Renders the export format dialog.
///
/// The selection in `search` is clamped to the number of format options.
///
/// # Arguments
///
/// * `frame` - Frame to draw on.
/// * `search` - Selection state of the format list (mutated to clamp it).
/// * `area` - Outer rectangle of the dialog.
pub fn render_export_format_menu(frame: &mut Frame, search: &mut PaletteSearch, area: Rect) {
    let inner = render_dialog(frame, area, "export format", EXPORT_FORMAT_HINTS, false);

    search.clamp_selection(EXPORT_FORMAT_OPTIONS.len());

    let option_lines: Vec<Line> = EXPORT_FORMAT_OPTIONS
        .iter()
        .enumerate()
        .map(|(index, option)| export_format_row_line(option, index == search.selected))
        .collect();

    frame.render_widget(
        Paragraph::new(option_lines).style(theme::app_background_style()),
        inner,
    );
}
