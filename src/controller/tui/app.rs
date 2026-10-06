#![deny(clippy::unwrap_used)]

use std::collections::VecDeque;
use std::io::stdin;
use std::io::stdout;
use std::sync::atomic::Ordering::Relaxed;
use std::time::Duration;

use colored::control::set_override;
use colored::control::unset_override;
use crossterm::ExecutableCommand;
use crossterm::event::DisableMouseCapture;
use crossterm::event::EnableMouseCapture;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use crossterm::event::MouseEventKind;
use is_terminal::IsTerminal;

use crate::AdbDevice;
use crate::CliArgs;
use crate::LogEntry;
use crate::State;
use crate::TuiFilterSet;
use crate::ValueOrPanic;
use crate::Writer;
use crate::build_adb_command;
use crate::get_adb_devices;
use crate::open_output_writer;
use crate::render_entry;
use crate::resolve_initial_device;
use crate::set_running;

use crate::controller::setup::bootstrap_adb_tui;
use crate::controller::setup::build_state;
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
use super::copy::copy_to_clipboard;
use super::device_picker::filter_device_indices;
use super::device_picker::is_selectable;
use super::display_cache::DisplayCache;
use super::file_source::default_browse_directory;
use super::file_source::validate_log_file;
use super::help::HelpAction;
use super::help::HelpRow;
use super::help::build_help_rows;
use super::help::row_action;
use super::log_ingest::LogIngest;
use super::palette::PaletteKeyAction;
use super::palette::PaletteSearch;
use super::palette::handle_palette_key;
use super::theme;
use super::ui;

use tui_file_explorer::ExplorerCommand;
use tui_file_explorer::ExplorerOutcome;
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
pub enum Overlay {
    None,
    DevicePicker,
    FileOpen,
    Help,
    CopyMenu,
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
    pub paused: bool,
    pub scroll_offset: usize,
    pub max_scroll: usize,
    pub auto_scroll: bool,
    pub selected_filtered_index: usize,
    pub viewport_lines: usize,
    pub copy_palette: PaletteSearch,
    pub copy_feedback: Option<String>,
    pub select_mode: bool,
    pub source_mode: SourceMode,
    pub selected_device: Option<String>,
    pub devices: Vec<AdbDevice>,
    pub overlay: Overlay,
    pub help_palette: PaletteSearch,
    pub device_palette: PaletteSearch,
    pub file_explorer: Option<FileExplorer>,
    pub file_open_error: Option<String>,
    pub explorer_theme: Theme,
    pub catchall_packages: Vec<String>,
    pub tokio_handle: Option<tokio::runtime::Handle>,
    pub ingest: LogIngest,
    pub file_writer: Option<Writer>,
    pub need_device_picker: bool,
    pub display_cache: DisplayCache,
    pub filter_generation: u64,
    pub shown_entry_count: usize,
    pub total_entry_count: usize,
}

impl TuiApp {
    pub fn new(args: CliArgs) -> Self {
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
            paused: false,
            scroll_offset: 0,
            max_scroll: 0,
            auto_scroll: true,
            selected_filtered_index: 0,
            viewport_lines: 0,
            copy_palette: PaletteSearch::default(),
            copy_feedback: None,
            select_mode: false,
            source_mode: SourceMode::Live,
            selected_device: None,
            devices: Vec::default(),
            overlay: Overlay::None,
            help_palette: PaletteSearch::default(),
            device_palette: PaletteSearch::default(),
            file_explorer: None,
            file_open_error: None,
            explorer_theme: theme::explorer_theme(),
            catchall_packages: Vec::default(),
            tokio_handle: None,
            ingest: LogIngest::idle(),
            file_writer: None,
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
        self.copy_feedback = None;
        let len = self.filtered_indices.len();
        let next = (self.selected_filtered_index as i32 + delta).clamp(0, len as i32 - 1) as usize;
        self.selected_filtered_index = next;
    }

    fn push_entry(&mut self, entry: LogEntry) {
        self.entries.push_back(entry);
        let index = self.entries.len() - 1;

        if let Some(writer) = &mut self.file_writer
            && let Some(last) = self.entries.back()
            && self.tui_filters.matches(last, &self.state)
        {
            render_entry(
                last,
                &mut self.state,
                &self.args,
                std::slice::from_mut(writer),
            );
        }

        if self.tui_filters.matches(&self.entries[index], &self.state) {
            self.filtered_indices.push(index);
        }
    }

    fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
        self.ingest.paused_flag().store(paused, Relaxed);
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
        self.ingest = LogIngest::start(
            handle,
            source,
            self.args.clone(),
            self.state.clone(),
            self.selected_device.clone(),
        );
        self.ingest.paused_flag().store(self.paused, Relaxed);
    }

    fn drain_ingest(&mut self) {
        if self.paused {
            return;
        }

        while let Some(batch) = self.ingest.try_recv_batch() {
            for item in batch {
                self.state = item.state;
                self.push_entry(item.entry);
            }
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
        self.refresh_device_maps();

        if self.source_mode == SourceMode::Live {
            self.start_ingest(SourceMode::Live);
        }
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
        self.exit_select_mode();
        self.file_open_error = None;
        self.file_explorer = Some(
            FileExplorer::builder(default_browse_directory())
                .show_hidden(true)
                .show_sizes(true)
                .sort_mode(SortMode::Name)
                .build(),
        );
        self.overlay = Overlay::FileOpen;
    }

    fn apply_filter(&mut self) {
        self.tui_filters = TuiFilterSet::parse(&self.filter_input);
        self.filter_focused = false;
        self.recompute_filtered();
    }

    fn scroll_up(&mut self, amount: usize) {
        self.auto_scroll = false;
        self.copy_feedback = None;
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
        if self.select_mode {
            self.sync_selection_to_viewport();
        }
    }

    fn scroll_down(&mut self, amount: usize) {
        self.auto_scroll = false;
        self.copy_feedback = None;
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

        if self.overlay == Overlay::FileOpen {
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

        if self.filter_focused {
            self.handle_filter_key(key);
            return;
        }

        self.copy_feedback = None;

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
        let Some(&entry_index) = self.filtered_indices.get(self.selected_filtered_index) else {
            return;
        };
        let Some(entry) = self.entries.get(entry_index) else {
            return;
        };

        let feedback = copy_action_feedback(action, &self.args);
        let text = copy_text_for_entry(entry, &self.state, &self.args, action);
        self.overlay = Overlay::None;
        self.copy_feedback = Some(match copy_to_clipboard(&text) {
            Ok(()) => format!("copied {feedback}"),
            Err(err) => format!("copy failed: {err}"),
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
            Overlay::FileOpen => {
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
        let Some(explorer) = &mut self.file_explorer else {
            self.overlay = Overlay::None;
            return;
        };

        match explorer.handle_key(KeyEvent::new(key, modifiers)) {
            ExplorerOutcome::Selected(path) => {
                let path_str = path.to_string_lossy().to_string();
                match validate_log_file(&path_str) {
                    Ok(valid_path) => {
                        self.overlay = Overlay::None;
                        self.file_open_error = None;
                        self.file_explorer = None;
                        self.switch_to_file(valid_path);
                    }
                    Err(err) => {
                        self.file_open_error = Some(err);
                    }
                }
            }
            ExplorerOutcome::Dismissed => {
                self.overlay = Overlay::None;
                self.file_open_error = None;
                self.file_explorer = None;
            }
            _ => {}
        }
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

    let (packages, catchall_packages, named_processes) = resolve_packages(args);
    let _ = packages;

    let stdin_is_tty = stdin().is_terminal();
    let initial_source = if stdin_is_tty {
        SourceMode::Live
    } else {
        SourceMode::Pipe
    };

    let mut app = TuiApp::new(args.clone());
    app.catchall_packages = catchall_packages.clone();
    app.filter_input = seed_filter_input(&app.args);
    app.tui_filters = TuiFilterSet::parse(&app.filter_input);
    if let Some(path) = app.args.output_path.clone() {
        app.file_writer = Some(open_output_writer(&path));
    }

    if stdin_is_tty {
        app.devices = bootstrap_adb_tui(&app.args);
        app.selected_device = resolve_initial_device(&app.args, &app.devices);
        maybe_clear_logcat(&app.args, app.selected_device.as_deref());

        if app.selected_device.is_none() {
            app.need_device_picker = true;
            app.overlay = Overlay::DevicePicker;
        }
    }

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

    match &initial_source {
        SourceMode::Pipe => app.start_ingest(SourceMode::Pipe),
        SourceMode::Live if app.selected_device.is_some() => app.start_ingest(SourceMode::Live),
        SourceMode::File(_) | SourceMode::Live => {}
    }

    stdout()
        .execute(EnableMouseCapture)
        .unwrap_or_panic("Failed to enable mouse capture");

    if !args.no_color {
        set_override(true);
    }

    let mut terminal = ratatui::init();

    while crate::is_running() {
        app.drain_ingest();

        if crossterm::event::poll(Duration::from_millis(50)).unwrap_or(false) {
            match crossterm::event::read().unwrap_or_panic("Failed to read event") {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        set_running(false);
                    } else {
                        app.handle_key(key.code, key.modifiers);
                    }
                }
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollUp => app.handle_mouse_scroll(true),
                    MouseEventKind::ScrollDown => app.handle_mouse_scroll(false),
                    _ => {}
                },
                _ => {}
            }
        }

        terminal
            .draw(|frame| ui::render(frame, &mut app))
            .unwrap_or_panic("Failed to draw frame");
    }

    app.stop_ingest();
    drop(runtime);
    ratatui::restore();
    let _ = stdout().execute(DisableMouseCapture);

    if !args.no_color {
        unset_override();
    }
}
