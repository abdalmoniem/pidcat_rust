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

const EXPORT_FORMAT_KEYS_WIDTH: usize = 4;
const EXPORT_FORMAT_HINTS: &[(&str, &str)] =
    &[("↑↓", " navigate"), ("enter", " choose"), ("esc", " close")];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportFormat {
    Pidcat,
    Adb,
}

pub struct ExportFormatOption {
    pub key: char,
    pub label: &'static str,
    pub format: ExportFormat,
}

pub const EXPORT_FORMAT_OPTIONS: &[ExportFormatOption] = &[
    ExportFormatOption {
        key: 'p',
        label: env!("CARGO_PKG_NAME"),
        format: ExportFormat::Pidcat,
    },
    ExportFormatOption {
        key: 'a',
        label: "adb",
        format: ExportFormat::Adb,
    },
];

impl ExportFormat {
    pub fn for_key(key: char) -> Option<Self> {
        EXPORT_FORMAT_OPTIONS
            .iter()
            .find(|option| option.key == key)
            .map(|option| option.format)
    }

    pub fn label(self) -> &'static str {
        EXPORT_FORMAT_OPTIONS
            .iter()
            .find(|option| option.format == self)
            .map_or("", |option| option.label)
    }
}

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
        ExportFormat::Pidcat => write_rendered_entries(&path, &entries, &mut render_state, &args)
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

async fn write_raw_entries(path: &str, entries: &VecDeque<LogEntry>) -> Result<usize, String> {
    let mut file = tokio::fs::File::create(path)
        .await
        .map_err(|err| format!("cannot create {path}: {err}"))?;
    let write_err = |err: std::io::Error| format!("cannot write {path}: {err}");

    for entry in entries {
        file.write_all(entry.raw.as_bytes())
            .await
            .map_err(write_err)?;
        file.write_all(b"\n").await.map_err(write_err)?;
    }

    file.flush().await.map_err(write_err)?;
    Ok(entries.len())
}

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

pub fn export_format_dialog_height() -> u16 {
    EXPORT_FORMAT_OPTIONS.len() as u16 + 2
}

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
