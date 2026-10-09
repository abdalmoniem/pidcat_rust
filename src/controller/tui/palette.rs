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

//! Shared "palette" widget state used by every searchable list dialog.
//!
//! The command palette, device picker, copy menu, export-format menu and file
//! path input all share the same behavior: a single-line text field with a
//! cursor above a list with a highlighted row. [`PaletteSearch`] stores that
//! state, [`handle_palette_key`] translates key presses into state changes,
//! and the remaining helpers implement token matching and field rendering.

#![deny(clippy::unwrap_used)]

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;

use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;

use super::theme;

/// State of a palette-style search field and its result list.
///
/// All indices (`cursor`, `selected`, `list_top`) are expressed in
/// characters / rows, not bytes.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PaletteSearch {
    /// The text currently typed into the search field.
    pub query: String,
    /// Cursor position inside `query`, measured in characters.
    pub cursor: usize,
    /// Index of the highlighted row in the result list.
    pub selected: usize,
    /// Index of the first visible row of the result list (scroll offset).
    pub list_top: usize,
}

/// The outcome of feeding a key press to [`handle_palette_key`].
///
/// The caller uses this to decide whether it needs to react (re-filter the
/// list, activate the selection, close the dialog, ...).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaletteKeyAction {
    /// Nothing the caller needs to react to (cursor or selection moved, or
    /// the key was ignored).
    None,
    /// The user confirmed the highlighted row (`Enter`).
    Enter,
    /// The user asked to close the dialog (`Esc` with an empty query).
    Close,
    /// The query was cleared by `Esc`; the list should be reset.
    ClearSearch,
    /// The query text changed; the list should be re-filtered.
    QueryChanged,
}

impl PaletteSearch {
    /// Clears the query and resets cursor, selection and scroll to the start.
    pub fn reset(&mut self) {
        self.query.clear();
        self.cursor = 0;
        self.selected = 0;
        self.list_top = 0;
    }

    /// Highlights `first_row` and scrolls the list back to the top.
    ///
    /// # Arguments
    ///
    /// * `first_row` - Row index to select (typically the first selectable row).
    pub fn select_first(&mut self, first_row: usize) {
        self.selected = first_row;
        self.list_top = 0;
    }

    /// Inserts `ch` at the cursor and advances the cursor by one character.
    ///
    /// # Arguments
    ///
    /// * `ch` - The character to insert.
    pub fn insert_char(&mut self, ch: char) {
        let byte_index = char_index_to_byte(&self.query, self.cursor);
        self.query.insert(byte_index, ch);
        self.cursor += 1;
    }

    /// Deletes the character before the cursor (`Backspace`).
    ///
    /// Does nothing when the cursor is at the start of the query.
    pub fn delete_before_cursor(&mut self) {
        if self.cursor == 0 {
            return;
        }

        let byte_index = char_index_to_byte(&self.query, self.cursor - 1);
        self.query.remove(byte_index);
        self.cursor -= 1;
    }

    /// Deletes the character under the cursor (`Delete`).
    ///
    /// Does nothing when the cursor is at the end of the query.
    pub fn delete_at_cursor(&mut self) {
        let char_count = self.query.chars().count();
        if self.cursor >= char_count {
            return;
        }

        let byte_index = char_index_to_byte(&self.query, self.cursor);
        self.query.remove(byte_index);
    }

    /// Moves the highlighted row by `delta`, skipping non-selectable rows.
    ///
    /// The target row is `selected + delta`, clamped to the list. The first
    /// selectable row at or beyond the target (in the direction of travel) is
    /// chosen; if the edge of the list is reached without finding one, the
    /// selectable row closest to the target on the way back is used. If no
    /// row qualifies the selection does not change.
    ///
    /// # Arguments
    ///
    /// * `row_count` - Total number of rows in the list.
    /// * `delta` - Signed number of rows to move (negative moves up).
    /// * `is_selectable` - Predicate telling whether a row index can be
    ///   highlighted (e.g. section headers cannot).
    pub fn move_selection(
        &mut self,
        row_count: usize,
        delta: i32,
        is_selectable: impl Fn(usize) -> bool,
    ) {
        if row_count == 0 || delta == 0 {
            return;
        }

        let last = row_count - 1;
        let start = self.selected.min(last);
        let target = start.saturating_add_signed(delta as isize).min(last);

        // Prefer the first selectable row at or past the target; when the edge is reached
        // without one, fall back to the selectable row closest to the target, else stay.
        let found = match delta > 0 {
            true => (target..=last)
                .find(|&index| is_selectable(index))
                .or_else(|| {
                    (start + 1..target)
                        .rev()
                        .find(|&index| is_selectable(index))
                }),
            false => (0..=target)
                .rev()
                .find(|&index| is_selectable(index))
                .or_else(|| (target + 1..start).find(|&index| is_selectable(index))),
        };

        if let Some(index) = found {
            self.selected = index;
        }
    }

    /// Clamps the highlighted row into `0..row_count`.
    ///
    /// # Arguments
    ///
    /// * `row_count` - Total number of rows in the list; `0` resets the
    ///   selection to row `0`.
    pub fn clamp_selection(&mut self, row_count: usize) {
        if row_count == 0 {
            self.selected = 0;
            return;
        }

        if self.selected >= row_count {
            self.selected = row_count - 1;
        }
    }

    /// Adjusts `list_top` so the highlighted row is inside the viewport.
    ///
    /// # Arguments
    ///
    /// * `list_height` - Number of rows visible at once; `0` is a no-op.
    pub fn ensure_list_top_visible(&mut self, list_height: usize) {
        if list_height == 0 {
            return;
        }

        if self.selected < self.list_top {
            self.list_top = self.selected;
        } else if self.selected >= self.list_top + list_height {
            self.list_top = self.selected + 1 - list_height;
        }
    }
}

/// Splits a search query into lowercase, whitespace-separated tokens.
///
/// # Arguments
///
/// * `query` - The raw text typed by the user.
///
/// # Returns
///
/// The ASCII-lowercased tokens; empty for a blank query.
pub fn query_tokens(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

/// Tests whether `haystack` contains every token of `query`.
///
/// Matching is case-insensitive (ASCII) and order-independent: each
/// whitespace-separated token of the query must appear somewhere in the
/// haystack.
///
/// # Arguments
///
/// * `query` - The user's search text.
/// * `haystack` - The text to search in.
///
/// # Returns
///
/// `true` if all tokens match, or if the query has no tokens.
pub fn matches_query(query: &str, haystack: &str) -> bool {
    let tokens = query_tokens(query);
    if tokens.is_empty() {
        return true;
    }

    let haystack = haystack.to_ascii_lowercase();
    tokens.iter().all(|token| haystack.contains(token))
}

/// Applies a key press to a [`PaletteSearch`].
///
/// Handled keys:
///
/// - `Esc` — clears a non-empty query ([`PaletteKeyAction::ClearSearch`]) or
///   asks to close ([`PaletteKeyAction::Close`]) when it is already empty;
/// - `Enter` — [`PaletteKeyAction::Enter`];
/// - `Backspace` / `Delete` / printable characters (without `Ctrl`) — edit the
///   query ([`PaletteKeyAction::QueryChanged`]);
/// - `Left` / `Right` / `Home` / `End` — move the text cursor;
/// - `j` / `Down`, `k` / `Up`, `PageDown`, `PageUp` — move the highlighted
///   row by 1 or 5 rows.
///
/// # Arguments
///
/// * `search` - The palette state to mutate.
/// * `key` - The pressed key.
/// * `modifiers` - The active modifier keys.
/// * `row_count` - Number of rows in the result list.
/// * `row_selectable` - Predicate telling whether a row can be highlighted.
///
/// # Returns
///
/// A [`PaletteKeyAction`] describing what the caller should do next.
pub fn handle_palette_key(
    search: &mut PaletteSearch,
    key: KeyCode,
    modifiers: KeyModifiers,
    row_count: usize,
    row_selectable: impl Fn(usize) -> bool,
) -> PaletteKeyAction {
    match key {
        KeyCode::Esc => {
            if search.query.is_empty() {
                PaletteKeyAction::Close
            } else {
                search.query.clear();
                search.cursor = 0;
                search.select_first(first_selectable_row(row_count, &row_selectable));
                PaletteKeyAction::ClearSearch
            }
        }
        KeyCode::Enter => PaletteKeyAction::Enter,
        KeyCode::Backspace => {
            search.delete_before_cursor();
            PaletteKeyAction::QueryChanged
        }
        KeyCode::Delete => {
            search.delete_at_cursor();
            PaletteKeyAction::QueryChanged
        }
        KeyCode::Left => {
            search.cursor = search.cursor.saturating_sub(1);
            PaletteKeyAction::None
        }
        KeyCode::Right => {
            let char_count = search.query.chars().count();
            if search.cursor < char_count {
                search.cursor += 1;
            }
            PaletteKeyAction::None
        }
        KeyCode::Home => {
            search.cursor = 0;
            PaletteKeyAction::None
        }
        KeyCode::End => {
            search.cursor = search.query.chars().count();
            PaletteKeyAction::None
        }
        KeyCode::Char('j') | KeyCode::Down => {
            search.move_selection(row_count, 1, &row_selectable);
            PaletteKeyAction::None
        }
        KeyCode::Char('k') | KeyCode::Up => {
            search.move_selection(row_count, -1, &row_selectable);
            PaletteKeyAction::None
        }
        KeyCode::PageDown => {
            search.move_selection(row_count, 5, &row_selectable);
            PaletteKeyAction::None
        }
        KeyCode::PageUp => {
            search.move_selection(row_count, -5, &row_selectable);
            PaletteKeyAction::None
        }
        KeyCode::Char(ch) if !modifiers.contains(KeyModifiers::CONTROL) => {
            search.insert_char(ch);
            PaletteKeyAction::QueryChanged
        }
        _ => PaletteKeyAction::None,
    }
}

/// Renders the search field of a palette and positions the terminal cursor.
///
/// Shows `placeholder` while the query is empty; otherwise shows the part of
/// the query that fits in `area`, scrolled so the cursor stays visible.
///
/// # Arguments
///
/// * `frame` - Frame to draw on (also receives the cursor position).
/// * `area` - Single-row region of the field.
/// * `search` - The palette state to display.
/// * `placeholder` - Hint text shown while the query is empty.
pub fn render_search_field(
    frame: &mut Frame,
    area: Rect,
    search: &PaletteSearch,
    placeholder: &str,
) {
    let visible_width = area.width as usize;
    let (field_line, scroll_chars) =
        input_field_line(&search.query, placeholder, search.cursor, visible_width);

    frame.render_widget(
        Paragraph::new(field_line).style(theme::app_background_style()),
        area,
    );

    let cursor_x = area.x.saturating_add(
        search
            .cursor
            .saturating_sub(scroll_chars)
            .min(visible_width.saturating_sub(1)) as u16,
    );
    frame.set_cursor_position((cursor_x, area.y));
}

/// Builds the visible line of a single-line text input.
///
/// # Arguments
///
/// * `input` - The full text of the input.
/// * `placeholder` - Hint shown (dimmed) when `input` is empty.
/// * `cursor_chars` - Cursor position in characters.
/// * `visible_width` - Number of columns available for the text.
///
/// # Returns
///
/// A tuple of the line to draw and the number of leading characters that
/// were scrolled out of view (needed to place the terminal cursor).
pub fn input_field_line(
    input: &str,
    placeholder: &str,
    cursor_chars: usize,
    visible_width: usize,
) -> (Line<'static>, usize) {
    if input.is_empty() {
        return (
            Line::from(Span::styled(
                placeholder.to_string(),
                theme::placeholder_style(),
            )),
            0,
        );
    }

    let (scroll_chars, display) = input_viewport(input, cursor_chars, visible_width);
    (Line::from(Span::raw(display)), scroll_chars)
}

/// Computes the horizontal window of `input` that keeps the cursor visible.
///
/// # Arguments
///
/// * `input` - The full text of the input.
/// * `cursor_chars` - Cursor position in characters (clamped to the text).
/// * `width` - Number of columns available; `0` yields an empty window.
///
/// # Returns
///
/// `(scroll_chars, display)` where `scroll_chars` is the number of characters
/// skipped from the start and `display` is the visible slice (at most `width`
/// characters).
fn input_viewport(input: &str, cursor_chars: usize, width: usize) -> (usize, String) {
    if width == 0 {
        return (0, String::new());
    }

    let chars: Vec<char> = input.chars().collect();
    let cursor_chars = cursor_chars.min(chars.len());

    let scroll_chars = if cursor_chars + 1 > width {
        cursor_chars.saturating_sub(width - 1)
    } else {
        0
    };

    let display: String = chars
        .iter()
        .skip(scroll_chars)
        .take(width)
        .copied()
        .collect();

    (scroll_chars, display)
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

/// Finds the first selectable row of a list.
///
/// # Arguments
///
/// * `row_count` - Total number of rows.
/// * `is_selectable` - Predicate telling whether a row can be highlighted.
///
/// # Returns
///
/// The index of the first selectable row, or `0` if there is none.
fn first_selectable_row(row_count: usize, is_selectable: &impl Fn(usize) -> bool) -> usize {
    (0..row_count)
        .find(|&index| is_selectable(index))
        .unwrap_or(0)
}
