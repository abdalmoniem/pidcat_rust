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

//! Incremental cache of rendered log lines.
//!
//! Rendering a log entry (column layout, coloring, wrapping) and parsing its
//! ANSI output into ratatui spans is comparatively expensive. [`DisplayCache`]
//! keeps the result of that work between frames and only renders a bounded
//! number of new entries per frame ([`DISPLAY_BUILD_BUDGET`]), so the UI stays
//! responsive while a large backlog of entries is being displayed. The cache
//! is rebuilt from scratch when the filter, the width or the color mode
//! changes.

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

/// Cache of the rendered display lines of all filtered log entries.
///
/// A single log entry may render into several lines (e.g. when a long message
/// wraps), so the cache keeps a parallel vector mapping every line back to the
/// index of its entry in the *filtered* list.
pub struct DisplayCache {
    /// Rendered lines, in display order.
    lines: Vec<Line<'static>>,
    /// For each line in `lines`, the filtered-entry index it belongs to.
    line_filtered_index: Vec<usize>,
    /// Number of filtered entries rendered so far.
    rendered_filtered_count: usize,
    /// Render state advanced as entries are rendered (tag de-duplication etc.).
    render_state: State,
    /// Filter generation the cache content was built for.
    filter_generation: u64,
    /// Render width the cache content was built for.
    width: i16,
    /// Color mode the cache content was built for.
    no_color: bool,
    /// Whether the cache holds usable content (`false` after [`invalidate`](Self::invalidate)).
    valid: bool,
}

impl DisplayCache {
    /// Creates an empty, invalid cache.
    ///
    /// # Returns
    ///
    /// A cache that will do a full rebuild on the first call to
    /// [`ensure`](Self::ensure).
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
                long_pending: None,
            },
            filter_generation: 0,
            width: 0,
            no_color: false,
            valid: false,
        }
    }

    /// Discards all cached lines and marks the cache as needing a rebuild.
    ///
    /// Call this whenever the set of filtered entries changes in a way other
    /// than appending (e.g. the filter was edited or the buffer was cleared).
    pub fn invalidate(&mut self) {
        self.lines.clear();
        self.line_filtered_index.clear();
        self.rendered_filtered_count = 0;
        self.valid = false;
    }

    /// Returns all lines rendered so far, in display order.
    pub fn lines(&self) -> &[Line<'static>] {
        &self.lines
    }

    /// Returns the number of lines rendered so far.
    pub fn rendered_line_count(&self) -> usize {
        self.lines.len()
    }

    /// Returns the number of filtered entries rendered so far.
    pub fn rendered_entry_count(&self) -> usize {
        self.rendered_filtered_count
    }

    /// Maps a display line to the filtered entry it belongs to.
    ///
    /// # Arguments
    ///
    /// * `line` - Index into [`lines`](Self::lines).
    ///
    /// # Returns
    ///
    /// The filtered-entry index, or `None` if `line` is out of range.
    pub fn line_filtered_index_at(&self, line: usize) -> Option<usize> {
        self.line_filtered_index.get(line).copied()
    }

    /// Finds the span of display lines occupied by a filtered entry.
    ///
    /// # Arguments
    ///
    /// * `filtered_index` - Index of the entry within the filtered list.
    ///
    /// # Returns
    ///
    /// `Some((first_line, line_count))`, or `None` if the entry has not been
    /// rendered yet.
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

    /// Brings the cache up to date, rendering at most `budget` new entries.
    ///
    /// If the filter generation, width or color mode differ from what the
    /// cache was built with, it is cleared and rebuilt from the start. Then
    /// the next not-yet-rendered filtered entries are rendered and appended.
    /// A `budget` of `0` (used while paused) renders nothing new.
    ///
    /// # Arguments
    ///
    /// * `filtered_indices` - Indices into `entries` of the entries that pass
    ///   the filter, in display order.
    /// * `entries` - The full entry buffer.
    /// * `ingest_state` - Latest render state from ingestion, used as the
    ///   starting state on a full rebuild.
    /// * `args` - The active CLI arguments.
    /// * `width` - Available render width in characters.
    /// * `filter_generation` - Counter that changes whenever the filter
    ///   result is recomputed from scratch.
    /// * `budget` - Maximum number of entries to render in this call.
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
                &mut AppendEntryContext {
                    entry,
                    render_state: &mut self.render_state,
                    args,
                    width,
                    no_color,
                },
            );
        }
        self.rendered_filtered_count = end;
    }

    /// Tests whether the cache content is stale and must be rebuilt.
    ///
    /// # Arguments
    ///
    /// * `filter_generation` - Current filter generation.
    /// * `width` - Current render width.
    /// * `no_color` - Current color mode.
    ///
    /// # Returns
    ///
    /// `true` if the cache is invalid or was built with different parameters.
    fn needs_full_rebuild(&self, filter_generation: u64, width: i16, no_color: bool) -> bool {
        !self.valid
            || self.filter_generation != filter_generation
            || self.width != width
            || self.no_color != no_color
    }

    /// Resets the cache for a full rebuild with the given parameters.
    ///
    /// Clears all lines, snapshots `ingest_state` as the new render state
    /// (with `last_tag` cleared) and marks the cache valid.
    ///
    /// # Arguments
    ///
    /// * `ingest_state` - Render state to start from.
    /// * `width` - Render width to build for.
    /// * `no_color` - Color mode to build for.
    /// * `filter_generation` - Filter generation to build for.
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

/// Parameters needed to render one entry into the cache.
///
/// Bundled into a struct to keep [`append_entry`]'s signature short.
struct AppendEntryContext<'a> {
    /// The entry to render.
    entry: &'a LogEntry,
    /// Render state, advanced by rendering this entry.
    render_state: &'a mut State,
    /// The active CLI arguments.
    args: &'a CliArgs,
    /// Render width in characters.
    width: i16,
    /// Whether to skip ANSI parsing and emit unstyled text.
    no_color: bool,
}

/// Renders one entry and appends its lines to the cache vectors.
///
/// # Arguments
///
/// * `lines` - Destination vector of display lines.
/// * `line_filtered_index` - Parallel vector receiving `filtered_index` once
///   per appended line.
/// * `filtered_index` - Index of the entry within the filtered list.
/// * `ctx` - The entry and render parameters.
fn append_entry(
    lines: &mut Vec<Line<'static>>,
    line_filtered_index: &mut Vec<usize>,
    filtered_index: usize,
    ctx: &mut AppendEntryContext<'_>,
) {
    for text in render_entry_lines(ctx.entry, ctx.render_state, ctx.args, ctx.width) {
        let line = if ctx.no_color {
            Line::from(text)
        } else {
            line_from_ansi(&text)
        };
        lines.push(line);
        line_filtered_index.push(filtered_index);
    }
}
