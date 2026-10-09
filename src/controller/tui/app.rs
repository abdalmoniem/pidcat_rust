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

//! Application state and event loop of the TUI.
//!
//! [`TuiApp`] holds everything the interface needs: the buffered log
//! entries, the active filter, scroll/selection state, the open overlay and
//! the handles to background work (log ingestion, clipboard, export). Input
//! events are translated into state changes by its `handle_*` methods, and
//! [`run_tui`] drives the whole thing: it prepares the terminal, starts log
//! ingestion, and then runs an event loop that multiplexes keyboard/mouse
//! input, newly ingested entries and status updates, redrawing after every
//! iteration.

#![deny(clippy::unwrap_used)]

use colored::control::set_override;
use crossterm::ExecutableCommand;
use crossterm::event::EnableMouseCapture;
use crossterm::event::Event;
use crossterm::event::EventStream;
use crossterm::event::KeyCode;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use crossterm::event::MouseEventKind;
use futures::StreamExt;
use is_terminal::IsTerminal;
use std::collections::VecDeque;
use std::io::stdin;
use std::io::stdout;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;

use crate::AdbDevice;
use crate::CliArgs;
use crate::LogEntry;
use crate::State;
use crate::TuiFilterSet;
use crate::ValueOrPanic;
use crate::build_adb_command;
use crate::controller::terminal::register_tui_color_override;
use crate::controller::terminal::register_tui_mouse_capture;
use crate::controller::terminal::register_tui_terminal;
use crate::controller::terminal::restore_tui_terminal;
use crate::get_adb_devices;
use crate::resolve_initial_device;
use crate::set_running;

use crate::controller::setup::bootstrap_adb_tui;
use crate::controller::setup::build_state;
use crate::controller::setup::current_app_packages;
use crate::controller::setup::maybe_clear_logcat;
use crate::controller::setup::normalize_cli_args;
use crate::controller::setup::refresh_process_maps;
use crate::controller::setup::resolve_packages;
use crate::controller::setup::seed_filter_input;

use super::copy::CopyAction;
use super::copy::available_copy_options;
use super::copy::copy_action_feedback;
use super::copy::copy_action_for_key;
use super::copy::copy_text_for_entry;
use super::copy::run_copy;
use super::device_picker::filter_device_indices;
use super::device_picker::is_selectable;
use super::display_cache::DisplayCache;
use super::export::EXPORT_FORMAT_OPTIONS;
use super::export::ExportFormat;
use super::export::default_export_file_name;
use super::export::run_export;
use super::file_source::default_browse_directory;
use super::file_source::default_export_directory;
use super::file_source::directory_input;
use super::file_source::expand_path;
use super::file_source::split_path_input;
use super::file_source::validate_log_file;
use super::help::HelpAction;
use super::help::HelpRow;
use super::help::build_help_rows;
use super::help::row_action;
use super::log_ingest::IngestUpdate;
use super::log_ingest::LogIngest;
use super::palette::PaletteKeyAction;
use super::palette::PaletteSearch;
use super::palette::handle_palette_key;
use super::theme;
use super::ui;

use tui_file_explorer::ExplorerCommand;
use tui_file_explorer::FileExplorer;
use tui_file_explorer::SortMode;
use tui_file_explorer::Theme;

/// Where the log lines shown in the TUI come from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceMode {
    /// A live `adb logcat` stream from the selected device.
    Live,
    /// Standard input (logs piped into `pidcatrs`).
    Pipe,
    /// A previously saved log file, identified by its path.
    File(String),
}

/// Purpose of the file dialog.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileDialogMode {
    /// Choose an existing log file to view.
    Open,
    /// Choose a destination file for an export.
    Save,
}

/// A message sent from a background task (copy/export) to the UI loop.
pub enum StatusUpdate {
    /// Text to display as the transient status feedback.
    Message(String),
    /// The clipboard copy task has finished (successfully or not).
    CopyFinished,
    /// The export task has finished (successfully or not).
    ExportFinished,
}

/// The modal dialog currently shown above the log view, if any.
///
/// While an overlay is open it receives all keyboard input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Overlay {
    /// No overlay; the log view and filter bar handle input.
    None,
    /// The "select device" dialog.
    DevicePicker,
    /// The open-file or export-destination dialog (see [`FileDialogMode`]).
    FileDialog,
    /// The command palette / help dialog.
    Help,
    /// The "copy log entry" menu.
    CopyMenu,
    /// The "export format" menu.
    ExportFormat,
}

/// Complete state of the interactive viewer.
///
/// The UI layer reads most fields directly when drawing; mutation happens
/// through the key/mouse handlers and the ingest/status update methods.
pub struct TuiApp {
    /// The active CLI arguments (normalized for TUI use).
    pub args: CliArgs,
    /// Parser/render state as of the most recently ingested entry.
    pub state: State,
    /// All buffered log entries, oldest first.
    pub entries: VecDeque<LogEntry>,
    /// Indices into `entries` of the entries that pass the current filter.
    pub filtered_indices: Vec<usize>,
    /// Text currently in the filter input.
    pub filter_input: String,
    /// Cursor position within `filter_input`, in characters.
    pub filter_cursor: usize,
    /// Whether the filter input has keyboard focus.
    pub filter_focused: bool,
    /// The parsed, applied filter.
    pub tui_filters: TuiFilterSet,
    /// Copy of the applied filter shared with the ingest task.
    filters_shared: Arc<RwLock<TuiFilterSet>>,
    /// Whether ingestion is paused.
    pub paused: bool,
    /// Index of the first visible display line.
    pub scroll_offset: usize,
    /// Largest valid `scroll_offset` (updated every frame).
    pub max_scroll: usize,
    /// Whether the view follows the newest entries ("live tail").
    pub auto_scroll: bool,
    /// Index (into the filtered list) of the entry selected in select mode.
    pub selected_filtered_index: usize,
    /// Number of log lines visible in the viewport (updated every frame).
    pub viewport_lines: usize,
    /// Selection state of the copy menu.
    pub copy_palette: PaletteSearch,
    /// Transient status message shown in the status bar.
    pub status_feedback: Option<String>,
    /// Whether select mode (entry-wise navigation and copy) is active.
    pub select_mode: bool,
    /// Where log lines currently come from.
    pub source_mode: SourceMode,
    /// Serial of the selected adb device, if any.
    pub selected_device: Option<String>,
    /// Devices found by the last `adb devices` query.
    pub devices: Vec<AdbDevice>,
    /// The overlay currently shown.
    pub overlay: Overlay,
    /// Search/selection state of the command palette.
    pub help_palette: PaletteSearch,
    /// Search/selection state of the device picker.
    pub device_palette: PaletteSearch,
    /// File explorer of the open file dialog (`Some` while it is open).
    pub file_explorer: Option<FileExplorer>,
    /// Error displayed inside the file dialog, if any.
    pub file_open_error: Option<String>,
    /// Path input of the file dialog.
    pub file_path_input: PaletteSearch,
    /// Whether the file dialog is opening a file or saving an export.
    pub file_dialog_mode: FileDialogMode,
    /// Path the user must confirm a second time to overwrite it.
    pending_overwrite: Option<String>,
    /// Suggested file name for the pending export.
    export_file_name: String,
    /// Selection state of the export format menu.
    pub export_format_palette: PaletteSearch,
    /// The export format chosen for the pending export.
    pub export_format: ExportFormat,
    /// Whether a clipboard copy is currently running.
    copy_in_progress: bool,
    /// Whether an export is currently running.
    export_in_progress: bool,
    /// Sender cloned into background tasks to report status updates.
    status_tx: tokio::sync::mpsc::UnboundedSender<StatusUpdate>,
    /// Receiver for entries produced by the ingest task.
    ingest_update_rx: tokio::sync::mpsc::UnboundedReceiver<IngestUpdate>,
    /// Color theme of the file explorer widget.
    pub explorer_theme: Theme,
    /// Catch-all packages resolved at startup or from the current app.
    pub catchall_packages: Vec<String>,
    /// Whether the current-app (`-c`) packages have been resolved on a device.
    current_app_resolved: bool,
    /// Handle of the Tokio runtime used to spawn background tasks.
    pub tokio_handle: Option<tokio::runtime::Handle>,
    /// Controller of the running log ingestion task.
    pub ingest: LogIngest,
    /// Whether the device picker was opened because no device was found at startup.
    pub need_device_picker: bool,
    /// Cache of the rendered log lines.
    pub display_cache: DisplayCache,
    /// Counter bumped whenever the filtered list is recomputed from scratch.
    pub filter_generation: u64,
    /// Number of filtered entries rendered so far (shown in the status bar).
    pub shown_entry_count: usize,
    /// Total number of buffered entries (shown in the status bar).
    pub total_entry_count: usize,
}

impl TuiApp {
    /// Creates the application in its initial state.
    ///
    /// The app starts in live mode with no device, no filter, auto-scroll
    /// enabled, an idle ingest controller and no runtime handle. The caller
    /// is expected to fill in the device, filter and runtime before starting
    /// ingestion (see [`run_tui`]).
    ///
    /// # Arguments
    ///
    /// * `args` - The CLI arguments to run with.
    pub fn new(args: CliArgs) -> Self {
        let (ingest, ingest_update_rx) = LogIngest::idle();
        let (status_tx, _status_rx) = tokio::sync::mpsc::unbounded_channel();

        Self {
            args,
            state: State {
                pids_map: Default::default(),
                uids_map: Default::default(),
                last_tag: None,
                app_pid: None,
                log_level: Default::default(),
                named_processes: Vec::default(),
                catchall_packages: Vec::default(),
                token_colors: Vec::default(),
                known_tokens: Default::default(),
                long_pending: None,
            },
            entries: VecDeque::default(),
            filtered_indices: Vec::default(),
            filter_input: String::default(),
            filter_cursor: 0,
            filter_focused: false,
            tui_filters: TuiFilterSet::default(),
            filters_shared: Arc::new(RwLock::new(TuiFilterSet::default())),
            paused: false,
            scroll_offset: 0,
            max_scroll: 0,
            auto_scroll: true,
            selected_filtered_index: 0,
            viewport_lines: 0,
            copy_palette: PaletteSearch::default(),
            status_feedback: None,
            select_mode: false,
            source_mode: SourceMode::Live,
            selected_device: None,
            devices: Vec::default(),
            overlay: Overlay::None,
            help_palette: PaletteSearch::default(),
            device_palette: PaletteSearch::default(),
            file_explorer: None,
            file_open_error: None,
            file_path_input: PaletteSearch::default(),
            file_dialog_mode: FileDialogMode::Open,
            pending_overwrite: None,
            export_file_name: String::new(),
            export_format_palette: PaletteSearch::default(),
            export_format: ExportFormat::Pidcatrs,
            copy_in_progress: false,
            export_in_progress: false,
            status_tx,
            ingest_update_rx,
            explorer_theme: theme::explorer_theme(),
            catchall_packages: Vec::default(),
            current_app_resolved: false,
            tokio_handle: None,
            ingest,
            need_device_picker: false,
            display_cache: DisplayCache::new(),
            filter_generation: 0,
            shown_entry_count: 0,
            total_entry_count: 0,
        }
    }

    /// Re-evaluates the filter against every buffered entry.
    ///
    /// Bumps the filter generation, invalidates the display cache and
    /// rebuilds `filtered_indices`. With auto-scroll the selection jumps to
    /// the last entry; otherwise the selection and scroll offset are clamped
    /// into the new range.
    fn recompute_filtered(&mut self) {
        self.filter_generation += 1;
        self.display_cache.invalidate();
        self.filtered_indices = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| self.tui_filters.matches(entry, &self.state))
            .map(|(index, _)| index)
            .collect();

        if self.auto_scroll {
            self.selected_filtered_index = self.filtered_indices.len().saturating_sub(1);
        } else {
            self.clamp_selected_filtered_index();
            let max_scroll = self.max_scroll_offset();
            self.scroll_offset = self.scroll_offset.min(max_scroll);
        }
    }

    /// Returns the largest valid scroll offset as of the last rendered frame.
    fn max_scroll_offset(&self) -> usize {
        self.max_scroll
    }

    /// Returns the entry currently selected in select mode.
    ///
    /// # Returns
    ///
    /// The selected entry, or `None` if the filtered list is empty or the
    /// selection is out of range.
    pub(crate) fn selected_log_entry(&self) -> Option<&LogEntry> {
        self.filtered_indices
            .get(self.selected_filtered_index)
            .and_then(|index| self.entries.get(*index))
    }

    /// Clamps the selected entry index into the filtered list (`0` if empty).
    pub(crate) fn clamp_selected_filtered_index(&mut self) {
        if self.filtered_indices.is_empty() {
            self.selected_filtered_index = 0;
        } else {
            self.selected_filtered_index = self
                .selected_filtered_index
                .min(self.filtered_indices.len() - 1);
        }
    }

    /// Scrolls the minimum amount needed to bring the selected entry into view.
    ///
    /// Uses the display cache to find the lines the entry occupies. Does
    /// nothing when there are no entries, the viewport is empty or the entry
    /// has not been rendered yet.
    pub(crate) fn ensure_selection_visible(&mut self) {
        if self.filtered_indices.is_empty() || self.viewport_lines == 0 {
            return;
        }

        let Some((start, count)) = self
            .display_cache
            .filtered_entry_line_range(self.selected_filtered_index)
        else {
            return;
        };

        let end = start + count.saturating_sub(1);
        if self.scroll_offset > start {
            self.scroll_offset = start;
        } else if self.viewport_lines > 0 && self.scroll_offset + self.viewport_lines <= end {
            self.scroll_offset = end.saturating_add(1).saturating_sub(self.viewport_lines);
        }
    }

    /// Selects the entry displayed in the middle of the viewport.
    ///
    /// Keeps the selection in sync with the view when the user scrolls (or
    /// enters select mode) so the highlighted entry is always on screen.
    fn sync_selection_to_viewport(&mut self) {
        if self.viewport_lines == 0 {
            return;
        }

        let anchor = self
            .scroll_offset
            .saturating_add(self.viewport_lines / 2)
            .min(self.display_cache.lines().len().saturating_sub(1));

        if let Some(index) = self.display_cache.line_filtered_index_at(anchor) {
            self.selected_filtered_index = index;
        }
    }

    /// Moves the select-mode selection by `delta` entries.
    ///
    /// Disables auto-scroll and clears the status feedback. The result is
    /// clamped to the filtered list.
    ///
    /// # Arguments
    ///
    /// * `delta` - Signed number of entries to move (negative moves up).
    fn move_entry_selection(&mut self, delta: i32) {
        if self.filtered_indices.is_empty() {
            return;
        }

        self.auto_scroll = false;
        self.status_feedback = None;
        let len = self.filtered_indices.len();
        let next = (self.selected_filtered_index as i32 + delta).clamp(0, len as i32 - 1) as usize;
        self.selected_filtered_index = next;
    }

    /// Pauses or resumes the app and the ingest task.
    ///
    /// # Arguments
    ///
    /// * `paused` - `true` to pause, `false` to resume.
    fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
        self.ingest.set_paused(paused);
    }

    /// Stops the running ingest task, if any.
    fn stop_ingest(&mut self) {
        self.ingest.stop();
    }

    /// Stops any running ingestion and starts reading from `source`.
    ///
    /// The new task inherits the current parser state, device, shared filter
    /// and pause flag.
    ///
    /// # Arguments
    ///
    /// * `source` - The new log source.
    ///
    /// # Panics
    ///
    /// Panics if the Tokio runtime handle has not been set.
    fn start_ingest(&mut self, source: SourceMode) {
        let handle = self
            .tokio_handle
            .as_ref()
            .unwrap_or_panic("tokio runtime handle is not set");
        self.ingest.stop();
        let (ingest, update_rx) = LogIngest::start(
            handle,
            source,
            self.args.clone(),
            self.state.clone(),
            self.selected_device.clone(),
            Arc::clone(&self.filters_shared),
        );
        self.ingest = ingest;
        self.ingest_update_rx = update_rx;
        self.ingest.set_paused(self.paused);
    }

    /// Appends a batch of ingested entries to the buffer.
    ///
    /// Entries that match the filter are also added to the filtered list, and
    /// the parser state is replaced by the one from the last update.
    ///
    /// # Arguments
    ///
    /// * `updates` - Updates received from the ingest task, oldest first.
    fn apply_ingest_updates(&mut self, updates: &[IngestUpdate]) {
        for update in updates {
            self.entries.push_back(update.entry.clone());
            if update.matches_filter {
                self.filtered_indices.push(self.entries.len() - 1);
            }
        }

        if let Some(last) = updates.last() {
            self.state = last.state.clone();
        }
    }

    /// Applies a status update from a background task.
    ///
    /// # Arguments
    ///
    /// * `update` - Message to show, or notification that a task finished.
    fn handle_status_update(&mut self, update: StatusUpdate) {
        match update {
            StatusUpdate::Message(message) => self.status_feedback = Some(message),
            StatusUpdate::CopyFinished => self.copy_in_progress = false,
            StatusUpdate::ExportFinished => self.export_in_progress = false,
        }
    }

    /// Publishes the applied filter to the ingest task.
    ///
    /// Silently skipped if the shared lock is poisoned.
    fn sync_shared_filters(&self) {
        if let Ok(mut shared) = self.filters_shared.write() {
            *shared = self.tui_filters.clone();
        }
    }

    /// Reloads the PID/UID/package maps for the selected device.
    fn refresh_device_maps(&mut self) {
        refresh_process_maps(
            &mut self.state,
            &self.args,
            &self.catchall_packages,
            self.selected_device.as_deref(),
        );
    }

    /// Clears the buffer and (re)starts live capture from the selected device.
    ///
    /// If no device is selected and a device is required, the device picker
    /// is opened instead.
    fn switch_to_live(&mut self) {
        if self.selected_device.is_none() && self.need_device_picker {
            self.overlay = Overlay::DevicePicker;
            return;
        }

        self.entries.clear();
        self.source_mode = SourceMode::Live;
        self.refresh_device_maps();
        self.start_ingest(SourceMode::Live);
        self.recompute_filtered();
    }

    /// Clears the buffer and loads the log file at `path`.
    ///
    /// # Arguments
    ///
    /// * `path` - Path of an already validated log file.
    fn switch_to_file(&mut self, path: String) {
        self.entries.clear();
        self.source_mode = SourceMode::File(path.clone());
        self.start_ingest(SourceMode::File(path));
        self.recompute_filtered();
    }

    /// Makes `serial` the active device and closes the overlay.
    ///
    /// Resolves the current-app filter if needed, refreshes the process maps
    /// and restarts live ingestion when currently in live mode.
    ///
    /// # Arguments
    ///
    /// * `serial` - Serial number of the chosen device.
    fn select_device(&mut self, serial: String) {
        self.selected_device = Some(serial);
        self.overlay = Overlay::None;
        self.apply_current_app_filter();
        self.refresh_device_maps();

        if self.source_mode == SourceMode::Live {
            self.start_ingest(SourceMode::Live);
        }
    }

    /// Resolves the current-app (`-c`) filter once a device is available.
    ///
    /// `-c` needs a device to query; when none was known at startup, it is
    /// resolved on first selection. The foreground app's packages are added
    /// to the filter as `package:<name>` tokens (skipping ones already
    /// present), the filter is re-applied and the buffer re-filtered. Runs at
    /// most once and only when `-c` was requested.
    fn apply_current_app_filter(&mut self) {
        if self.current_app_resolved || !self.args.current_app {
            return;
        }
        self.current_app_resolved = true;

        let packages = current_app_packages(&self.args, self.selected_device.as_deref());
        let existing = self.filter_input.split_whitespace().collect::<Vec<_>>();
        let additions = packages
            .iter()
            .filter(|package| !package.contains(':'))
            .map(|package| format!("package:{package}"))
            .filter(|token| !existing.contains(&token.as_str()))
            .collect::<Vec<_>>();
        if additions.is_empty() {
            return;
        }

        for package in packages {
            if !self.catchall_packages.contains(&package) {
                self.catchall_packages.push(package);
            }
        }
        self.filter_input = std::iter::once(self.filter_input.trim().to_string())
            .chain(additions)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        self.filter_cursor = self.filter_input.chars().count();
        self.tui_filters = TuiFilterSet::parse(&self.filter_input);
        self.sync_shared_filters();
        self.recompute_filtered();
    }

    /// Opens the device picker with a freshly queried device list.
    ///
    /// Leaves select mode and resets the picker's search field. If querying
    /// `adb devices` fails the list is empty.
    fn open_device_picker(&mut self) {
        self.exit_select_mode();
        let base = build_adb_command(&self.args, None);
        self.devices = get_adb_devices(&base, true).unwrap_or_default();
        self.device_palette.reset();
        self.overlay = Overlay::DevicePicker;
    }

    /// Selects the highlighted device in the picker.
    ///
    /// Ignored when the highlighted device is not selectable (e.g. offline).
    /// If the app is not showing live logs it switches back to live mode.
    fn confirm_device_picker(&mut self) {
        let filtered = filter_device_indices(&self.devices, &self.device_palette.query);
        let Some(&device_index) = filtered.get(self.device_palette.selected) else {
            return;
        };

        if let Some(device) = self.devices.get(device_index)
            && is_selectable(device)
        {
            let serial = device.device_id.clone();
            self.select_device(serial);

            if self.source_mode != SourceMode::Live {
                self.switch_to_live();
            }
        }
    }

    /// Opens the "open log file" dialog.
    fn open_file_dialog(&mut self) {
        self.show_file_dialog(FileDialogMode::Open, String::new());
    }

    /// Tells whether there is anything to export.
    ///
    /// # Returns
    ///
    /// `true` if at least one entry passes the current filter.
    pub(crate) fn has_exportable_entries(&self) -> bool {
        !self.filtered_indices.is_empty()
    }

    /// Describes which entries an export would include.
    ///
    /// # Returns
    ///
    /// `"all"` when no filter is applied, otherwise `"filtered"`.
    pub(crate) fn export_scope(&self) -> &'static str {
        match self.tui_filters.is_empty() {
            true => "all",
            false => "filtered",
        }
    }

    /// Opens the export format menu.
    ///
    /// Does nothing if there are no entries to export. If an export is
    /// already running only a status message is shown.
    fn open_export_dialog(&mut self) {
        if !self.has_exportable_entries() {
            return;
        }
        if self.export_in_progress {
            self.status_feedback = Some("export already in progress".to_string());
            return;
        }
        self.exit_select_mode();
        self.export_format_palette.reset();
        self.export_format_palette.select_first(0);
        self.overlay = Overlay::ExportFormat;
    }

    /// Handles a key press while the export format menu is open.
    ///
    /// A format's shortcut key chooses it immediately; other keys are
    /// processed by the shared palette handler (navigation, `Enter`, `Esc`).
    ///
    /// # Arguments
    ///
    /// * `key` - The pressed key.
    /// * `modifiers` - The active modifier keys.
    fn handle_export_format_key(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        if let KeyCode::Char(ch) = key
            && let Some(format) = ExportFormat::for_key(ch)
        {
            self.choose_export_format(format);
            return;
        }

        let action = handle_palette_key(
            &mut self.export_format_palette,
            key,
            modifiers,
            EXPORT_FORMAT_OPTIONS.len(),
            |_| true,
        );

        match action {
            PaletteKeyAction::Enter => {
                if let Some(option) = EXPORT_FORMAT_OPTIONS.get(self.export_format_palette.selected)
                {
                    self.choose_export_format(option.format);
                }
            }
            PaletteKeyAction::Close => self.overlay = Overlay::None,
            _ => {}
        }
    }

    /// Records the chosen export format and opens the save dialog.
    ///
    /// The dialog is pre-filled with a default file name for the format.
    ///
    /// # Arguments
    ///
    /// * `format` - The format to export in.
    fn choose_export_format(&mut self, format: ExportFormat) {
        self.export_format = format;
        let file_name =
            default_export_file_name(format, &self.source_mode, self.selected_device.as_deref());
        self.show_file_dialog(FileDialogMode::Save, file_name);
    }

    /// Opens the file dialog in the given mode.
    ///
    /// Leaves select mode, clears any previous error or pending overwrite,
    /// creates a file explorer in the mode's start directory (the home
    /// directory to open, the working directory to save) and pre-fills the
    /// path input with that directory plus `file_name`.
    ///
    /// # Arguments
    ///
    /// * `mode` - Whether the dialog opens or saves a file.
    /// * `file_name` - Initial file name appended to the directory (may be empty).
    fn show_file_dialog(&mut self, mode: FileDialogMode, file_name: String) {
        self.exit_select_mode();
        self.file_open_error = None;
        self.pending_overwrite = None;
        self.file_dialog_mode = mode;
        self.export_file_name = file_name.clone();
        let start_directory = match mode {
            FileDialogMode::Open => default_browse_directory(),
            FileDialogMode::Save => default_export_directory(),
        };
        self.file_path_input.reset();
        self.file_explorer = Some(
            FileExplorer::builder(start_directory.clone())
                .show_hidden(true)
                .show_sizes(true)
                .sort_mode(SortMode::Name)
                .build(),
        );
        self.set_file_path_input(directory_input(&start_directory) + &file_name);
        self.overlay = Overlay::FileDialog;
    }

    /// Starts exporting the filtered entries to `path` in the background.
    ///
    /// Snapshots the filtered entries, marks an export as in progress and
    /// spawns [`run_export`]. Does nothing if no runtime handle is set.
    ///
    /// # Arguments
    ///
    /// * `path` - Destination file path.
    fn start_export(&mut self, path: String) {
        let Some(handle) = self.tokio_handle.as_ref() else {
            return;
        };

        let entries = self
            .filtered_indices
            .iter()
            .filter_map(|&index| self.entries.get(index).cloned())
            .collect::<VecDeque<_>>();

        self.export_in_progress = true;
        self.status_feedback = Some(format!("exporting to {path}..."));

        let status_tx = self.status_tx.clone();
        let format = self.export_format;
        let state = self.state.clone();
        let args = self.args.clone();

        handle.spawn(async move {
            run_export(status_tx, path, format, entries, state, args).await;
        });
    }

    /// Parses and applies the text in the filter input.
    ///
    /// Removes focus from the input, publishes the filter to the ingest task
    /// and re-filters the buffer.
    fn apply_filter(&mut self) {
        self.tui_filters = TuiFilterSet::parse(&self.filter_input);
        self.filter_focused = false;
        self.sync_shared_filters();
        self.recompute_filtered();
    }

    /// Scrolls the log view up by `amount` lines.
    ///
    /// Disables auto-scroll and, in select mode, moves the selection along.
    ///
    /// # Arguments
    ///
    /// * `amount` - Number of lines to scroll.
    fn scroll_up(&mut self, amount: usize) {
        self.auto_scroll = false;
        self.status_feedback = None;
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
        if self.select_mode {
            self.sync_selection_to_viewport();
        }
    }

    /// Scrolls the log view down by `amount` lines.
    ///
    /// Reaching the bottom re-enables auto-scroll (live tail). In select mode
    /// the selection follows the view.
    ///
    /// # Arguments
    ///
    /// * `amount` - Number of lines to scroll.
    fn scroll_down(&mut self, amount: usize) {
        self.auto_scroll = false;
        self.status_feedback = None;
        let max = self.max_scroll_offset();
        self.scroll_offset = (self.scroll_offset + amount).min(max);

        if self.scroll_offset >= max {
            self.auto_scroll = true;
            if self.select_mode {
                self.selected_filtered_index = self.filtered_indices.len().saturating_sub(1);
            }
        } else if self.select_mode {
            self.sync_selection_to_viewport();
        }
    }

    /// Jumps to the newest entry and resumes live tailing.
    fn scroll_to_bottom(&mut self) {
        self.auto_scroll = true;
        if self.select_mode {
            self.selected_filtered_index = self.filtered_indices.len().saturating_sub(1);
        }
    }

    /// Toggles select mode.
    ///
    /// Entering syncs the selection to the viewport; leaving closes the copy
    /// menu if it is open.
    fn toggle_select_mode(&mut self) {
        self.select_mode = !self.select_mode;
        if self.select_mode {
            self.sync_selection_to_viewport();
        } else if self.overlay == Overlay::CopyMenu {
            self.overlay = Overlay::None;
        }
    }

    /// Leaves select mode if active, closing the copy menu if it is open.
    fn exit_select_mode(&mut self) {
        if !self.select_mode {
            return;
        }
        self.select_mode = false;
        if self.overlay == Overlay::CopyMenu {
            self.overlay = Overlay::None;
        }
    }

    /// Dispatches a key press to the right handler.
    ///
    /// Priority order: the open overlay (which captures all input), the
    /// global `Ctrl+S` export shortcut, the focused filter input, and finally
    /// the main log view bindings (quit, pause, filter focus, pickers, help,
    /// select mode, scrolling and jumping to top/bottom).
    ///
    /// # Arguments
    ///
    /// * `key` - The pressed key.
    /// * `modifiers` - The active modifier keys.
    fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        if self.overlay == Overlay::DevicePicker {
            self.handle_device_picker_key(key, modifiers);
            return;
        }

        if self.overlay == Overlay::FileDialog {
            self.handle_file_open_key(key, modifiers);
            return;
        }

        if self.overlay == Overlay::Help {
            self.handle_help_key(key, modifiers);
            return;
        }

        if self.overlay == Overlay::CopyMenu {
            self.handle_copy_menu_key(key, modifiers);
            return;
        }

        if self.overlay == Overlay::ExportFormat {
            self.handle_export_format_key(key, modifiers);
            return;
        }

        if key == KeyCode::Char('s') && modifiers.contains(KeyModifiers::CONTROL) {
            self.filter_focused = false;
            self.open_export_dialog();
            return;
        }

        if self.filter_focused {
            self.handle_filter_key(key);
            return;
        }

        self.status_feedback = None;

        match key {
            KeyCode::Char('q') => set_running(false),
            KeyCode::Char('p') | KeyCode::Char(' ') => self.set_paused(!self.paused),
            KeyCode::Char('/') => {
                self.filter_focused = true;
                self.filter_cursor = self.filter_input.chars().count();
            }
            KeyCode::Char('d') => self.open_device_picker(),
            KeyCode::Char('o') => self.open_file_dialog(),
            KeyCode::Char('l') => self.switch_to_live(),
            KeyCode::Char('?') => self.open_help(),
            KeyCode::Char('v') => self.toggle_select_mode(),
            KeyCode::Esc => self.exit_select_mode(),
            KeyCode::Char('y') | KeyCode::Enter if self.select_mode => self.open_copy_menu(),
            KeyCode::Char('j') | KeyCode::Down if self.select_mode => self.move_entry_selection(1),
            KeyCode::Char('k') | KeyCode::Up if self.select_mode => self.move_entry_selection(-1),
            KeyCode::PageDown if self.select_mode => self.move_entry_selection(10),
            KeyCode::PageUp if self.select_mode => self.move_entry_selection(-10),
            KeyCode::Char('j') | KeyCode::Down => self.scroll_down(1),
            KeyCode::Char('k') | KeyCode::Up => self.scroll_up(1),
            KeyCode::PageDown => self.scroll_down(10),
            KeyCode::PageUp => self.scroll_up(10),
            KeyCode::Char('G') if modifiers.contains(KeyModifiers::SHIFT) => {
                self.scroll_to_bottom()
            }
            KeyCode::Char('g') => {
                self.auto_scroll = false;
                self.scroll_offset = 0;
                if self.select_mode {
                    self.selected_filtered_index = 0;
                }
            }
            KeyCode::End => self.scroll_to_bottom(),
            KeyCode::Home => {
                self.auto_scroll = false;
                self.scroll_offset = 0;
                if self.select_mode {
                    self.selected_filtered_index = 0;
                }
            }
            _ => {}
        }
    }

    /// Opens the copy menu for the selected entry.
    ///
    /// Does nothing when there are no filtered entries.
    fn open_copy_menu(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }

        self.clamp_selected_filtered_index();
        self.copy_palette.reset();
        self.copy_palette.select_first(0);
        self.overlay = Overlay::CopyMenu;
    }

    /// Handles a key press while the copy menu is open.
    ///
    /// An option's shortcut key runs it immediately; other keys are processed
    /// by the shared palette handler (navigation, `Enter`, `Esc`).
    ///
    /// # Arguments
    ///
    /// * `key` - The pressed key.
    /// * `modifiers` - The active modifier keys.
    fn handle_copy_menu_key(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        let options = available_copy_options(&self.args);

        if let KeyCode::Char(ch) = key
            && let Some(action) = copy_action_for_key(ch, &self.args)
        {
            self.execute_copy_action(action);
            return;
        }

        let action = handle_palette_key(
            &mut self.copy_palette,
            key,
            modifiers,
            options.len(),
            |_| true,
        );

        match action {
            PaletteKeyAction::Enter => self.execute_copy_selected_option(),
            PaletteKeyAction::Close => self.overlay = Overlay::None,
            _ => {}
        }
    }

    /// Runs the copy option highlighted in the menu.
    fn execute_copy_selected_option(&mut self) {
        let options = available_copy_options(&self.args);
        let Some(option) = options.get(self.copy_palette.selected) else {
            return;
        };
        self.execute_copy_action(option.action);
    }

    /// Copies part of the selected entry to the clipboard in the background.
    ///
    /// Closes the menu, marks a copy as in progress and spawns [`run_copy`].
    /// If a copy is already running only a status message is shown. Does
    /// nothing when there is no selected entry or no runtime handle.
    ///
    /// # Arguments
    ///
    /// * `action` - Which part of the entry to copy.
    fn execute_copy_action(&mut self, action: CopyAction) {
        if self.copy_in_progress {
            self.status_feedback = Some("copy already in progress".to_string());
            return;
        }

        let Some(&entry_index) = self.filtered_indices.get(self.selected_filtered_index) else {
            return;
        };
        let Some(entry) = self.entries.get(entry_index) else {
            return;
        };

        let Some(handle) = self.tokio_handle.as_ref() else {
            return;
        };

        let feedback = copy_action_feedback(action, &self.args);
        let text = copy_text_for_entry(entry, &self.state, &self.args, action);
        self.overlay = Overlay::None;

        self.copy_in_progress = true;
        self.status_feedback = Some(format!("copying {feedback}..."));

        let status_tx = self.status_tx.clone();
        handle.spawn(async move {
            run_copy(status_tx, text, feedback).await;
        });
    }

    /// Handles a key press while the filter input has focus.
    ///
    /// `Esc` drops focus, `Enter` applies the filter, the cursor keys move
    /// the cursor, and printable characters / `Backspace` / `Delete` edit
    /// the text.
    ///
    /// # Arguments
    ///
    /// * `key` - The pressed key.
    fn handle_filter_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => self.filter_focused = false,
            KeyCode::Enter => self.apply_filter(),
            KeyCode::Backspace => self.delete_filter_char_before_cursor(),
            KeyCode::Delete => self.delete_filter_char_at_cursor(),
            KeyCode::Left => self.move_filter_cursor_left(),
            KeyCode::Right => self.move_filter_cursor_right(),
            KeyCode::Home => self.filter_cursor = 0,
            KeyCode::End => self.filter_cursor = self.filter_input.chars().count(),
            KeyCode::Char(ch) => self.insert_filter_char(ch),
            _ => {}
        }
    }

    /// Inserts `ch` into the filter input at the cursor.
    ///
    /// # Arguments
    ///
    /// * `ch` - The character to insert.
    fn insert_filter_char(&mut self, ch: char) {
        let byte_index = char_index_to_byte(&self.filter_input, self.filter_cursor);
        self.filter_input.insert(byte_index, ch);
        self.filter_cursor += 1;
    }

    /// Deletes the filter character before the cursor (`Backspace`).
    fn delete_filter_char_before_cursor(&mut self) {
        if self.filter_cursor == 0 {
            return;
        }

        let byte_index = char_index_to_byte(&self.filter_input, self.filter_cursor - 1);
        self.filter_input.remove(byte_index);
        self.filter_cursor -= 1;
    }

    /// Deletes the filter character under the cursor (`Delete`).
    fn delete_filter_char_at_cursor(&mut self) {
        let char_count = self.filter_input.chars().count();
        if self.filter_cursor >= char_count {
            return;
        }

        let byte_index = char_index_to_byte(&self.filter_input, self.filter_cursor);
        self.filter_input.remove(byte_index);
    }

    /// Moves the filter cursor one character left (stops at the start).
    fn move_filter_cursor_left(&mut self) {
        self.filter_cursor = self.filter_cursor.saturating_sub(1);
    }

    /// Moves the filter cursor one character right (stops at the end).
    fn move_filter_cursor_right(&mut self) {
        let char_count = self.filter_input.chars().count();
        if self.filter_cursor < char_count {
            self.filter_cursor += 1;
        }
    }

    /// Opens the command palette with an empty query.
    fn open_help(&mut self) {
        self.exit_select_mode();
        self.help_palette.reset();
        self.overlay = Overlay::Help;
        self.help_reset_selection();
    }

    /// Runs a command chosen in the command palette.
    ///
    /// # Arguments
    ///
    /// * `action` - The command to execute.
    fn execute_help_action(&mut self, action: HelpAction) {
        match action {
            HelpAction::Quit => set_running(false),
            HelpAction::PauseResume => self.set_paused(!self.paused),
            HelpAction::RestartLive => self.switch_to_live(),
            HelpAction::OpenDevicePicker => self.open_device_picker(),
            HelpAction::OpenFileDialog => self.open_file_dialog(),
            HelpAction::ExportEntries => self.open_export_dialog(),
            HelpAction::FocusFilter => {
                self.filter_focused = true;
                self.filter_cursor = self.filter_input.chars().count();
            }
        }
    }

    /// Returns the command palette rows for the current query.
    fn help_rows(&self) -> Vec<HelpRow> {
        build_help_rows(&self.help_palette.query)
    }

    /// Moves the palette highlight to the first entry row (skipping headings).
    fn help_reset_selection(&mut self) {
        let rows = self.help_rows();
        let first = rows
            .iter()
            .position(|row| matches!(row, HelpRow::Entry(_)))
            .unwrap_or(0);
        self.help_palette.select_first(first);
    }

    /// Executes the highlighted palette row, if it has an action.
    ///
    /// Closes the palette before running the command. Informational entries
    /// and headings are ignored.
    fn help_execute_selected(&mut self) {
        let rows = self.help_rows();
        let Some(row) = rows.get(self.help_palette.selected) else {
            return;
        };

        if let Some(action) = row_action(row) {
            self.overlay = Overlay::None;
            self.execute_help_action(action);
        }
    }

    /// Handles a key press while the command palette is open.
    ///
    /// `?` closes the palette (or clears a non-empty query); everything else
    /// goes to the shared palette handler, with only entry rows selectable.
    ///
    /// # Arguments
    ///
    /// * `key` - The pressed key.
    /// * `modifiers` - The active modifier keys.
    fn handle_help_key(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        if matches!(key, KeyCode::Char('?')) {
            if self.help_palette.query.is_empty() {
                self.overlay = Overlay::None;
            } else {
                self.help_palette.reset();
                self.help_reset_selection();
            }
            return;
        }

        let rows = self.help_rows();
        let action = handle_palette_key(
            &mut self.help_palette,
            key,
            modifiers,
            rows.len(),
            |index| matches!(rows.get(index), Some(HelpRow::Entry(_))),
        );

        match action {
            PaletteKeyAction::Enter => self.help_execute_selected(),
            PaletteKeyAction::Close => self.overlay = Overlay::None,
            PaletteKeyAction::ClearSearch | PaletteKeyAction::QueryChanged => {
                self.help_reset_selection();
            }
            PaletteKeyAction::None => {}
        }
    }

    /// Moves the palette highlight up by `amount` entry rows.
    ///
    /// # Arguments
    ///
    /// * `amount` - Number of selectable rows to move.
    fn scroll_help_up(&mut self, amount: usize) {
        let rows = self.help_rows();
        for _ in 0..amount {
            self.help_palette.move_selection(rows.len(), -1, |index| {
                matches!(rows.get(index), Some(HelpRow::Entry(_)))
            });
        }
    }

    /// Moves the palette highlight down by `amount` entry rows.
    ///
    /// # Arguments
    ///
    /// * `amount` - Number of selectable rows to move.
    fn scroll_help_down(&mut self, amount: usize) {
        let rows = self.help_rows();
        for _ in 0..amount {
            self.help_palette.move_selection(rows.len(), 1, |index| {
                matches!(rows.get(index), Some(HelpRow::Entry(_)))
            });
        }
    }

    /// Handles a mouse wheel event.
    ///
    /// The wheel moves the selection of the open overlay by 3 rows, or
    /// scrolls the log view by 3 lines when no overlay is open and the
    /// filter input is not focused.
    ///
    /// # Arguments
    ///
    /// * `scroll_up` - `true` for wheel up, `false` for wheel down.
    fn handle_mouse_scroll(&mut self, scroll_up: bool) {
        match self.overlay {
            Overlay::Help => {
                if scroll_up {
                    self.scroll_help_up(3);
                } else {
                    self.scroll_help_down(3);
                }
            }
            Overlay::DevicePicker => {
                let filtered = filter_device_indices(&self.devices, &self.device_palette.query);
                for _ in 0..3 {
                    self.device_palette.move_selection(
                        filtered.len(),
                        if scroll_up { -1 } else { 1 },
                        |_| true,
                    );
                }
            }
            Overlay::FileDialog => {
                if let Some(explorer) = &mut self.file_explorer {
                    let command = if scroll_up {
                        ExplorerCommand::MoveUp
                    } else {
                        ExplorerCommand::MoveDown
                    };
                    for _ in 0..3 {
                        let _ = explorer.handle_command(command);
                    }
                }
            }
            Overlay::CopyMenu => {
                let option_count = available_copy_options(&self.args).len();
                for _ in 0..3 {
                    self.copy_palette.move_selection(
                        option_count,
                        if scroll_up { -1 } else { 1 },
                        |_| true,
                    );
                }
            }
            Overlay::ExportFormat => {
                for _ in 0..3 {
                    self.export_format_palette.move_selection(
                        EXPORT_FORMAT_OPTIONS.len(),
                        if scroll_up { -1 } else { 1 },
                        |_| true,
                    );
                }
            }
            Overlay::None if !self.filter_focused => {
                if scroll_up {
                    self.scroll_up(3);
                } else {
                    self.scroll_down(3);
                }
            }
            _ => {}
        }
    }

    /// Handles a key press while the device picker is open.
    ///
    /// `Ctrl+R` refreshes the device list, `o` (with an empty query) switches
    /// to the open-file dialog; everything else goes to the shared palette
    /// handler.
    ///
    /// # Arguments
    ///
    /// * `key` - The pressed key.
    /// * `modifiers` - The active modifier keys.
    fn handle_device_picker_key(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        if key == KeyCode::Char('r') && modifiers.contains(KeyModifiers::CONTROL) {
            self.open_device_picker();
            return;
        }

        if self.device_palette.query.is_empty() && key == KeyCode::Char('o') {
            self.overlay = Overlay::None;
            self.open_file_dialog();
            return;
        }

        let filtered = filter_device_indices(&self.devices, &self.device_palette.query);
        let action = handle_palette_key(
            &mut self.device_palette,
            key,
            modifiers,
            filtered.len(),
            |_| true,
        );

        match action {
            PaletteKeyAction::Enter => self.confirm_device_picker(),
            PaletteKeyAction::Close => self.overlay = Overlay::None,
            PaletteKeyAction::ClearSearch | PaletteKeyAction::QueryChanged => {
                let filtered = filter_device_indices(&self.devices, &self.device_palette.query);
                self.device_palette.select_first(0);
                self.device_palette.clamp_selection(filtered.len());
            }
            PaletteKeyAction::None => {}
        }
    }

    /// Handles a key press while the file dialog (open or save) is open.
    ///
    /// `Esc` closes, `Enter` confirms according to the dialog mode, `Tab`
    /// completes the path from the explorer selection, the arrow/page keys
    /// move the explorer selection, and the remaining keys edit the path
    /// input (which keeps the explorer in sync).
    ///
    /// # Arguments
    ///
    /// * `key` - The pressed key.
    /// * `modifiers` - The active modifier keys.
    fn handle_file_open_key(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        if self.file_explorer.is_none() {
            self.overlay = Overlay::None;
            return;
        }

        let input = &mut self.file_path_input;
        match key {
            KeyCode::Esc => self.close_file_dialog(),
            KeyCode::Enter => match self.file_dialog_mode {
                FileDialogMode::Open => self.confirm_file_path(),
                FileDialogMode::Save => self.confirm_save_path(),
            },
            KeyCode::Tab => self.complete_file_path(),
            KeyCode::Up => self.move_file_selection(ExplorerCommand::MoveUp),
            KeyCode::Down => self.move_file_selection(ExplorerCommand::MoveDown),
            KeyCode::PageUp => self.move_file_selection(ExplorerCommand::PageUp),
            KeyCode::PageDown => self.move_file_selection(ExplorerCommand::PageDown),
            KeyCode::Left => input.cursor = input.cursor.saturating_sub(1),
            KeyCode::Right => input.cursor = (input.cursor + 1).min(input.query.chars().count()),
            KeyCode::Home => input.cursor = 0,
            KeyCode::End => input.cursor = input.query.chars().count(),
            KeyCode::Backspace => {
                input.delete_before_cursor();
                self.sync_file_explorer_to_input();
            }
            KeyCode::Delete => {
                input.delete_at_cursor();
                self.sync_file_explorer_to_input();
            }
            KeyCode::Char(ch) if !modifiers.contains(KeyModifiers::CONTROL) => {
                input.insert_char(ch);
                self.sync_file_explorer_to_input();
            }
            _ => {}
        }
    }

    /// Sends a navigation command to the file explorer, if one is open.
    ///
    /// # Arguments
    ///
    /// * `command` - The explorer command (move up/down, page up/down).
    fn move_file_selection(&mut self, command: ExplorerCommand) {
        if let Some(explorer) = &mut self.file_explorer {
            let _ = explorer.handle_command(command);
        }
    }

    /// Returns the entry highlighted in the file explorer.
    ///
    /// # Returns
    ///
    /// `(path, is_dir)` of the entry, or `None` if there is no explorer or
    /// nothing is highlighted.
    fn selected_file_entry(&self) -> Option<(PathBuf, bool)> {
        self.file_explorer
            .as_ref()
            .and_then(|explorer| explorer.current_entry())
            .map(|entry| (entry.path.clone(), entry.is_dir))
    }

    /// Replaces the file dialog's path input and syncs the explorer to it.
    ///
    /// The cursor is moved to the end of the new text.
    ///
    /// # Arguments
    ///
    /// * `value` - The new path text.
    fn set_file_path_input(&mut self, value: String) {
        self.file_path_input.cursor = value.chars().count();
        self.file_path_input.query = value;
        self.sync_file_explorer_to_input();
    }

    /// Makes the file explorer reflect the typed path.
    ///
    /// Navigates to the typed directory (setting `file_open_error` if it does
    /// not exist or cannot be entered) and uses the trailing file-name
    /// fragment as the explorer's search query. Clears the error and any
    /// pending overwrite confirmation on success.
    fn sync_file_explorer_to_input(&mut self) {
        let Some(explorer) = &mut self.file_explorer else {
            return;
        };

        let (directory, fragment) = split_path_input(&self.file_path_input.query);
        if let Some(directory) = directory
            && directory != explorer.current_dir
        {
            if !directory.is_dir() {
                self.file_open_error =
                    Some(format!("directory not found: {}", directory.display()));
                return;
            }
            if let Err(err) = explorer.try_navigate_to(directory) {
                self.file_open_error = Some(err.to_string());
                return;
            }
        }

        self.file_open_error = None;
        self.pending_overwrite = None;
        explorer.search_query = fragment;
        explorer.cursor = 0;
        explorer.reload();
    }

    /// Completes the typed path with the highlighted explorer entry.
    ///
    /// Directories get a trailing separator so typing can continue inside.
    fn complete_file_path(&mut self) {
        let Some((path, is_dir)) = self.selected_file_entry() else {
            return;
        };
        let completed = if is_dir {
            directory_input(&path)
        } else {
            path.to_string_lossy().to_string()
        };
        self.set_file_path_input(completed);
    }

    /// Confirms the open-file dialog (`Enter`).
    ///
    /// If the typed path is an existing file it is opened. Otherwise the
    /// highlighted explorer entry is used: directories are entered, files are
    /// opened. With nothing highlighted the typed path is tried (and its
    /// validation error shown).
    fn confirm_file_path(&mut self) {
        let typed = expand_path(self.file_path_input.query.trim());
        if Path::new(&typed).is_file() {
            self.open_log_file(&typed);
            return;
        }

        match self.selected_file_entry() {
            Some((_, true)) => self.complete_file_path(),
            Some((path, false)) => self.open_log_file(&path.to_string_lossy()),
            None => self.open_log_file(&typed),
        }
    }

    /// Confirms the save dialog (`Enter`).
    ///
    /// - If only a directory is typed, the default file name is appended.
    /// - If the path is an existing directory, the dialog navigates into it.
    /// - If the parent directory does not exist, an error is shown.
    /// - If the file already exists, an error asks the user to press `Enter`
    ///   again; the second confirmation overwrites it.
    /// - Otherwise the dialog closes and the export starts.
    fn confirm_save_path(&mut self) {
        let typed = expand_path(self.file_path_input.query.trim());
        let (directory, file_name) = split_path_input(&typed);

        if file_name.is_empty() {
            let with_name = typed + &self.export_file_name;
            self.set_file_path_input(with_name);
            return;
        }

        let target = Path::new(&typed);
        if target.is_dir() {
            self.set_file_path_input(directory_input(target));
            return;
        }

        if let Some(directory) = directory
            && !directory.is_dir()
        {
            self.file_open_error = Some(format!("directory not found: {}", directory.display()));
            return;
        }

        if target.exists() && self.pending_overwrite.as_deref() != Some(typed.as_str()) {
            self.file_open_error = Some(format!(
                "file exists: {typed} (press enter again to overwrite)"
            ));
            self.pending_overwrite = Some(typed);
            return;
        }

        self.close_file_dialog();
        self.start_export(typed);
    }

    /// Opens `path` as the log source after validating it.
    ///
    /// On success the dialog closes and the app switches to the file; on
    /// failure the validation error is shown in the dialog.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to open.
    fn open_log_file(&mut self, path: &str) {
        match validate_log_file(path) {
            Ok(valid_path) => {
                self.close_file_dialog();
                self.switch_to_file(valid_path);
            }
            Err(err) => self.file_open_error = Some(err),
        }
    }

    /// Closes the file dialog and discards its transient state.
    fn close_file_dialog(&mut self) {
        self.overlay = Overlay::None;
        self.file_open_error = None;
        self.file_explorer = None;
        self.file_path_input.reset();
        self.pending_overwrite = None;
    }
}

/// Converts a character index into a byte index of `text`.
///
/// # Arguments
///
/// * `text` - The string to index.
/// * `char_index` - Position in characters.
///
/// # Returns
///
/// The byte offset of that character, or `text.len()` if `char_index` is
/// past the end.
fn char_index_to_byte(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map_or(text.len(), |(index, _)| index)
}

/// Runs the interactive TUI until the user quits.
///
/// Startup sequence:
///
/// 1. Normalizes the CLI arguments and forces "capture everything"
///    (`tui_mode`, `all`), because filtering is done in the filter bar.
/// 2. Chooses the source: live logcat when stdin is a terminal, otherwise
///    the piped input. For live mode it bootstraps adb, resolves the device
///    and optionally clears logcat; without a device the device picker opens.
/// 3. Resolves the packages, seeds the filter input, builds the parser state
///    and starts a multi-threaded Tokio runtime.
/// 4. Enables mouse capture and color overrides, initializes the terminal and
///    starts ingestion.
///
/// The event loop then waits on keyboard/mouse events, batches of ingested
/// entries (not while paused) and background status updates, and redraws
/// after each one. When the loop ends the ingest task is stopped and the
/// terminal is restored.
///
/// # Arguments
///
/// * `args` - The CLI arguments; normalized and modified in place.
///
/// # Panics
///
/// Panics if the Tokio runtime cannot be created, mouse capture cannot be
/// enabled, or a frame cannot be drawn.
pub fn run_tui(args: &mut CliArgs) {
    normalize_cli_args(args);

    // Capture all log lines and filter in the TUI bar (same as Android Studio).
    args.tui_mode = true;
    args.all = true;

    let stdin_is_tty = stdin().is_terminal();
    let initial_source = if stdin_is_tty {
        SourceMode::Live
    } else {
        SourceMode::Pipe
    };

    let mut app = TuiApp::new(args.clone());

    if stdin_is_tty {
        app.devices = bootstrap_adb_tui(&app.args);
        app.selected_device = resolve_initial_device(&app.args, &app.devices);
        maybe_clear_logcat(&app.args, app.selected_device.as_deref());

        if app.selected_device.is_none() {
            app.need_device_picker = true;
            app.overlay = Overlay::DevicePicker;
        }
    }

    let (packages, catchall_packages, named_processes) =
        resolve_packages(&mut app.args, app.selected_device.as_deref());
    app.current_app_resolved = app.selected_device.is_some();
    app.catchall_packages = catchall_packages.clone();
    app.filter_input = seed_filter_input(&app.args, &packages);
    app.tui_filters = TuiFilterSet::parse(&app.filter_input);
    app.sync_shared_filters();

    app.state = build_state(
        &app.args,
        &catchall_packages,
        named_processes,
        app.selected_device.as_deref(),
    );

    set_running(true);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap_or_panic("Failed to start tokio runtime");
    app.tokio_handle = Some(runtime.handle().clone());

    app.source_mode = initial_source.clone();

    stdout()
        .execute(EnableMouseCapture)
        .unwrap_or_panic("Failed to enable mouse capture");
    register_tui_mouse_capture();

    if !args.no_color {
        set_override(true);
        register_tui_color_override();
    }

    let (status_tx, mut status_rx) = tokio::sync::mpsc::unbounded_channel();
    app.status_tx = status_tx;

    match &initial_source {
        SourceMode::Pipe => app.start_ingest(SourceMode::Pipe),
        SourceMode::Live if app.selected_device.is_some() => app.start_ingest(SourceMode::Live),
        SourceMode::File(_) | SourceMode::Live => {}
    }

    runtime.block_on(async {
        let mut terminal = ratatui::init();
        register_tui_terminal();
        let mut events = EventStream::new();

        while crate::is_running() {
            tokio::select! {
                event = events.next() => match event {
                    Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                        if key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            set_running(false);
                        } else {
                            app.handle_key(key.code, key.modifiers);
                        }
                    }
                    Some(Ok(Event::Mouse(mouse))) => match mouse.kind {
                        MouseEventKind::ScrollUp => app.handle_mouse_scroll(true),
                        MouseEventKind::ScrollDown => app.handle_mouse_scroll(false),
                        _ => {}
                    },
                    Some(Ok(Event::Resize(_, _))) => {}
                    Some(Err(_)) | None => set_running(false),
                    _ => {}
                },
                    ingest = async {
                        let mut updates = Vec::new();
                        let count = app.ingest_update_rx.recv_many(&mut updates, 256).await;
                        (count, updates)
                    }, if !app.paused => {
                        let (count, updates) = ingest;
                        if count > 0 {
                            app.apply_ingest_updates(&updates);
                        }
                    }
                update = status_rx.recv() => match update {
                    Some(update) => app.handle_status_update(update),
                    None => set_running(false),
                }
            }

            terminal
                .draw(|frame| ui::render(frame, &mut app))
                .unwrap_or_panic("Failed to draw frame");
        }

        app.stop_ingest();
        restore_tui_terminal();
    });

    drop(runtime);
}
