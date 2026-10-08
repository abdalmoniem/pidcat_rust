#![deny(clippy::unwrap_used)]

use colored::control::set_override;
use colored::control::unset_override;
use crossterm::ExecutableCommand;
use crossterm::event::DisableMouseCapture;
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceMode {
    Live,
    Pipe,
    File(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileDialogMode {
    Open,
    Save,
}

pub enum StatusUpdate {
    Message(String),
    CopyFinished,
    ExportFinished,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Overlay {
    None,
    DevicePicker,
    FileDialog,
    Help,
    CopyMenu,
    ExportFormat,
}

pub struct TuiApp {
    pub args: CliArgs,
    pub state: State,
    pub entries: VecDeque<LogEntry>,
    pub filtered_indices: Vec<usize>,
    pub filter_input: String,
    pub filter_cursor: usize,
    pub filter_focused: bool,
    pub tui_filters: TuiFilterSet,
    filters_shared: Arc<RwLock<TuiFilterSet>>,
    pub paused: bool,
    pub scroll_offset: usize,
    pub max_scroll: usize,
    pub auto_scroll: bool,
    pub selected_filtered_index: usize,
    pub viewport_lines: usize,
    pub copy_palette: PaletteSearch,
    pub status_feedback: Option<String>,
    pub select_mode: bool,
    pub source_mode: SourceMode,
    pub selected_device: Option<String>,
    pub devices: Vec<AdbDevice>,
    pub overlay: Overlay,
    pub help_palette: PaletteSearch,
    pub device_palette: PaletteSearch,
    pub file_explorer: Option<FileExplorer>,
    pub file_open_error: Option<String>,
    pub file_path_input: PaletteSearch,
    pub file_dialog_mode: FileDialogMode,
    pending_overwrite: Option<String>,
    export_file_name: String,
    pub export_format_palette: PaletteSearch,
    pub export_format: ExportFormat,
    copy_in_progress: bool,
    export_in_progress: bool,
    status_tx: tokio::sync::mpsc::UnboundedSender<StatusUpdate>,
    ingest_update_rx: tokio::sync::mpsc::UnboundedReceiver<IngestUpdate>,
    pub explorer_theme: Theme,
    pub catchall_packages: Vec<String>,
    current_app_resolved: bool,
    pub tokio_handle: Option<tokio::runtime::Handle>,
    pub ingest: LogIngest,
    pub need_device_picker: bool,
    pub display_cache: DisplayCache,
    pub filter_generation: u64,
    pub shown_entry_count: usize,
    pub total_entry_count: usize,
}

impl TuiApp {
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
            export_format: ExportFormat::Pidcat,
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

    fn max_scroll_offset(&self) -> usize {
        self.max_scroll
    }

    pub(crate) fn selected_log_entry(&self) -> Option<&LogEntry> {
        self.filtered_indices
            .get(self.selected_filtered_index)
            .and_then(|index| self.entries.get(*index))
    }

    pub(crate) fn clamp_selected_filtered_index(&mut self) {
        if self.filtered_indices.is_empty() {
            self.selected_filtered_index = 0;
        } else {
            self.selected_filtered_index = self
                .selected_filtered_index
                .min(self.filtered_indices.len() - 1);
        }
    }

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

    fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
        self.ingest.set_paused(paused);
    }

    fn stop_ingest(&mut self) {
        self.ingest.stop();
    }

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

    fn handle_status_update(&mut self, update: StatusUpdate) {
        match update {
            StatusUpdate::Message(message) => self.status_feedback = Some(message),
            StatusUpdate::CopyFinished => self.copy_in_progress = false,
            StatusUpdate::ExportFinished => self.export_in_progress = false,
        }
    }

    fn sync_shared_filters(&self) {
        if let Ok(mut shared) = self.filters_shared.write() {
            *shared = self.tui_filters.clone();
        }
    }

    fn refresh_device_maps(&mut self) {
        refresh_process_maps(
            &mut self.state,
            &self.args,
            &self.catchall_packages,
            self.selected_device.as_deref(),
        );
    }

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

    fn switch_to_file(&mut self, path: String) {
        self.entries.clear();
        self.source_mode = SourceMode::File(path.clone());
        self.start_ingest(SourceMode::File(path));
        self.recompute_filtered();
    }

    fn select_device(&mut self, serial: String) {
        self.selected_device = Some(serial);
        self.overlay = Overlay::None;
        self.apply_current_app_filter();
        self.refresh_device_maps();

        if self.source_mode == SourceMode::Live {
            self.start_ingest(SourceMode::Live);
        }
    }

    /// `-c` needs a device to query; when none was known at startup, resolve on first selection.
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

    fn open_device_picker(&mut self) {
        self.exit_select_mode();
        let base = build_adb_command(&self.args, None);
        self.devices = get_adb_devices(&base, true).unwrap_or_default();
        self.device_palette.reset();
        self.overlay = Overlay::DevicePicker;
    }

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

    fn open_file_dialog(&mut self) {
        self.show_file_dialog(FileDialogMode::Open, String::new());
    }

    pub(crate) fn has_exportable_entries(&self) -> bool {
        !self.filtered_indices.is_empty()
    }

    pub(crate) fn export_scope(&self) -> &'static str {
        match self.tui_filters.is_empty() {
            true => "all",
            false => "filtered",
        }
    }

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

    fn choose_export_format(&mut self, format: ExportFormat) {
        self.export_format = format;
        let file_name =
            default_export_file_name(format, &self.source_mode, self.selected_device.as_deref());
        self.show_file_dialog(FileDialogMode::Save, file_name);
    }

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

    fn apply_filter(&mut self) {
        self.tui_filters = TuiFilterSet::parse(&self.filter_input);
        self.filter_focused = false;
        self.sync_shared_filters();
        self.recompute_filtered();
    }

    fn scroll_up(&mut self, amount: usize) {
        self.auto_scroll = false;
        self.status_feedback = None;
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
        if self.select_mode {
            self.sync_selection_to_viewport();
        }
    }

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

    fn scroll_to_bottom(&mut self) {
        self.auto_scroll = true;
        if self.select_mode {
            self.selected_filtered_index = self.filtered_indices.len().saturating_sub(1);
        }
    }

    fn toggle_select_mode(&mut self) {
        self.select_mode = !self.select_mode;
        if self.select_mode {
            self.sync_selection_to_viewport();
        } else if self.overlay == Overlay::CopyMenu {
            self.overlay = Overlay::None;
        }
    }

    fn exit_select_mode(&mut self) {
        if !self.select_mode {
            return;
        }
        self.select_mode = false;
        if self.overlay == Overlay::CopyMenu {
            self.overlay = Overlay::None;
        }
    }

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

    fn open_copy_menu(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }

        self.clamp_selected_filtered_index();
        self.copy_palette.reset();
        self.copy_palette.select_first(0);
        self.overlay = Overlay::CopyMenu;
    }

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

    fn execute_copy_selected_option(&mut self) {
        let options = available_copy_options(&self.args);
        let Some(option) = options.get(self.copy_palette.selected) else {
            return;
        };
        self.execute_copy_action(option.action);
    }

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

    fn insert_filter_char(&mut self, ch: char) {
        let byte_index = char_index_to_byte(&self.filter_input, self.filter_cursor);
        self.filter_input.insert(byte_index, ch);
        self.filter_cursor += 1;
    }

    fn delete_filter_char_before_cursor(&mut self) {
        if self.filter_cursor == 0 {
            return;
        }

        let byte_index = char_index_to_byte(&self.filter_input, self.filter_cursor - 1);
        self.filter_input.remove(byte_index);
        self.filter_cursor -= 1;
    }

    fn delete_filter_char_at_cursor(&mut self) {
        let char_count = self.filter_input.chars().count();
        if self.filter_cursor >= char_count {
            return;
        }

        let byte_index = char_index_to_byte(&self.filter_input, self.filter_cursor);
        self.filter_input.remove(byte_index);
    }

    fn move_filter_cursor_left(&mut self) {
        self.filter_cursor = self.filter_cursor.saturating_sub(1);
    }

    fn move_filter_cursor_right(&mut self) {
        let char_count = self.filter_input.chars().count();
        if self.filter_cursor < char_count {
            self.filter_cursor += 1;
        }
    }

    fn open_help(&mut self) {
        self.exit_select_mode();
        self.help_palette.reset();
        self.overlay = Overlay::Help;
        self.help_reset_selection();
    }

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

    fn help_rows(&self) -> Vec<HelpRow> {
        build_help_rows(&self.help_palette.query)
    }

    fn help_reset_selection(&mut self) {
        let rows = self.help_rows();
        let first = rows
            .iter()
            .position(|row| matches!(row, HelpRow::Entry(_)))
            .unwrap_or(0);
        self.help_palette.select_first(first);
    }

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

    fn scroll_help_up(&mut self, amount: usize) {
        let rows = self.help_rows();
        for _ in 0..amount {
            self.help_palette.move_selection(rows.len(), -1, |index| {
                matches!(rows.get(index), Some(HelpRow::Entry(_)))
            });
        }
    }

    fn scroll_help_down(&mut self, amount: usize) {
        let rows = self.help_rows();
        for _ in 0..amount {
            self.help_palette.move_selection(rows.len(), 1, |index| {
                matches!(rows.get(index), Some(HelpRow::Entry(_)))
            });
        }
    }

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

    fn move_file_selection(&mut self, command: ExplorerCommand) {
        if let Some(explorer) = &mut self.file_explorer {
            let _ = explorer.handle_command(command);
        }
    }

    fn selected_file_entry(&self) -> Option<(PathBuf, bool)> {
        self.file_explorer
            .as_ref()
            .and_then(|explorer| explorer.current_entry())
            .map(|entry| (entry.path.clone(), entry.is_dir))
    }

    fn set_file_path_input(&mut self, value: String) {
        self.file_path_input.cursor = value.chars().count();
        self.file_path_input.query = value;
        self.sync_file_explorer_to_input();
    }

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

    fn open_log_file(&mut self, path: &str) {
        match validate_log_file(path) {
            Ok(valid_path) => {
                self.close_file_dialog();
                self.switch_to_file(valid_path);
            }
            Err(err) => self.file_open_error = Some(err),
        }
    }

    fn close_file_dialog(&mut self) {
        self.overlay = Overlay::None;
        self.file_open_error = None;
        self.file_explorer = None;
        self.file_path_input.reset();
        self.pending_overwrite = None;
    }
}

fn char_index_to_byte(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map_or(text.len(), |(index, _)| index)
}

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

    if !args.no_color {
        set_override(true);
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
        ratatui::restore();
    });

    drop(runtime);
    let _ = stdout().execute(DisableMouseCapture);

    if !args.no_color {
        unset_override();
    }
}
