#![deny(clippy::unwrap_used)]

use std::collections::VecDeque;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::thread;
use std::thread::JoinHandle;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;

use crate::CliArgs;
use crate::LogEntry;
use crate::State;
use crate::Writer;
use crate::render_entry;

use super::app::SourceMode;
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

pub struct ExportJob {
    pub path: String,
    handle: JoinHandle<Result<usize, String>>,
}

impl ExportJob {
    pub fn start(
        path: String,
        format: ExportFormat,
        entries: VecDeque<LogEntry>,
        state: &State,
        args: &CliArgs,
    ) -> Result<Self, String> {
        let mut render_state = state.clone();
        render_state.last_tag = None;
        let args = args.clone();
        let target = path.clone();

        let handle = thread::Builder::new()
            .name(format!("{}-export", env!("CARGO_PKG_NAME")))
            .spawn(move || match format {
                ExportFormat::Pidcat => {
                    write_rendered_entries(&target, &entries, &mut render_state, &args)
                }
                ExportFormat::Adb => write_raw_entries(&target, &entries),
            })
            .map_err(|err| format!("cannot start export: {err}"))?;

        Ok(Self { path, handle })
    }

    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }

    pub fn finish(self) -> String {
        match self.handle.join() {
            Ok(Ok(count)) => format!(
                "exported {} entries to {}",
                crate::controller::util::format_usize_separated(count),
                self.path
            ),
            Ok(Err(err)) => format!("export failed: {err}"),
            Err(_) => format!("export failed: {}", self.path),
        }
    }
}

fn create_file(path: &str) -> Result<File, String> {
    File::create(path).map_err(|err| format!("cannot create {path}: {err}"))
}

fn write_rendered_entries(
    path: &str,
    entries: &VecDeque<LogEntry>,
    state: &mut State,
    args: &CliArgs,
) -> Result<usize, String> {
    let mut writer = Writer::new_file(create_file(path)?);
    for entry in entries {
        render_entry(entry, state, args, std::slice::from_mut(&mut writer));
    }
    writer.flush();
    Ok(entries.len())
}

fn write_raw_entries(path: &str, entries: &VecDeque<LogEntry>) -> Result<usize, String> {
    let mut writer = BufWriter::new(create_file(path)?);
    let write_err = |err: std::io::Error| format!("cannot write {path}: {err}");
    for entry in entries {
        writeln!(writer, "{}", entry.raw).map_err(write_err)?;
    }
    writer.flush().map_err(write_err)?;
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
