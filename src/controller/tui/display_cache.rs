#![deny(clippy::unwrap_used)]

use std::collections::VecDeque;

use ratatui::text::Line;

use crate::CliArgs;
use crate::LogEntry;
use crate::State;
use crate::render_entry_lines;

use super::ui::line_from_ansi;

/// How many filtered entries to render into the display cache per frame.
pub const DISPLAY_BUILD_BUDGET: usize = 400;

pub struct DisplayCache {
    lines: Vec<Line<'static>>,
    line_filtered_index: Vec<usize>,
    rendered_filtered_count: usize,
    render_state: State,
    filter_generation: u64,
    width: i16,
    no_color: bool,
    valid: bool,
}

impl DisplayCache {
    pub fn new() -> Self {
        Self {
            lines: Vec::default(),
            line_filtered_index: Vec::default(),
            rendered_filtered_count: 0,
            render_state: State {
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
            filter_generation: 0,
            width: 0,
            no_color: false,
            valid: false,
        }
    }

    pub fn invalidate(&mut self) {
        self.lines.clear();
        self.line_filtered_index.clear();
        self.rendered_filtered_count = 0;
        self.valid = false;
    }

    pub fn lines(&self) -> &[Line<'static>] {
        &self.lines
    }

    pub fn line_filtered_index_at(&self, line: usize) -> Option<usize> {
        self.line_filtered_index.get(line).copied()
    }

    pub fn filtered_entry_line_range(&self, filtered_index: usize) -> Option<(usize, usize)> {
        let start = self
            .line_filtered_index
            .iter()
            .position(|index| *index == filtered_index)?;
        let count = self
            .line_filtered_index
            .iter()
            .skip(start)
            .take_while(|index| **index == filtered_index)
            .count();
        Some((start, count))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn ensure(
        &mut self,
        filtered_indices: &[usize],
        entries: &VecDeque<LogEntry>,
        ingest_state: &State,
        args: &CliArgs,
        width: i16,
        filter_generation: u64,
        budget: usize,
    ) {
        let no_color = args.no_color;

        if self.needs_full_rebuild(filter_generation, width, no_color) {
            self.start_rebuild(ingest_state, width, no_color, filter_generation);
        }

        if budget == 0 || self.rendered_filtered_count >= filtered_indices.len() {
            return;
        }

        let end = (self.rendered_filtered_count + budget).min(filtered_indices.len());
        for (offset, &entry_index) in filtered_indices[self.rendered_filtered_count..end]
            .iter()
            .enumerate()
        {
            let filtered_index = self.rendered_filtered_count + offset;
            let Some(entry) = entries.get(entry_index) else {
                continue;
            };
            append_entry(
                &mut self.lines,
                &mut self.line_filtered_index,
                filtered_index,
                entry,
                &mut self.render_state,
                args,
                width,
                no_color,
            );
        }
        self.rendered_filtered_count = end;
    }

    fn needs_full_rebuild(&self, filter_generation: u64, width: i16, no_color: bool) -> bool {
        !self.valid
            || self.filter_generation != filter_generation
            || self.width != width
            || self.no_color != no_color
    }

    fn start_rebuild(
        &mut self,
        ingest_state: &State,
        width: i16,
        no_color: bool,
        filter_generation: u64,
    ) {
        self.lines.clear();
        self.line_filtered_index.clear();
        self.rendered_filtered_count = 0;
        self.render_state = ingest_state.clone();
        self.render_state.last_tag = None;
        self.filter_generation = filter_generation;
        self.width = width;
        self.no_color = no_color;
        self.valid = true;
    }
}

fn append_entry(
    lines: &mut Vec<Line<'static>>,
    line_filtered_index: &mut Vec<usize>,
    filtered_index: usize,
    entry: &LogEntry,
    render_state: &mut State,
    args: &CliArgs,
    width: i16,
    no_color: bool,
) {
    for text in render_entry_lines(entry, render_state, args, width) {
        let line = if no_color {
            Line::from(text)
        } else {
            line_from_ansi(&text)
        };
        lines.push(line);
        line_filtered_index.push(filtered_index);
    }
}
