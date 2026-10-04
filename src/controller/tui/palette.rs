#![deny(clippy::unwrap_used)]

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;

use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;

use super::theme;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PaletteSearch {
    pub query: String,
    pub cursor: usize,
    pub selected: usize,
    pub list_top: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaletteKeyAction {
    None,
    Enter,
    Close,
    ClearSearch,
    QueryChanged,
}

impl PaletteSearch {
    pub fn reset(&mut self) {
        self.query.clear();
        self.cursor = 0;
        self.selected = 0;
        self.list_top = 0;
    }

    pub fn select_first(&mut self, first_row: usize) {
        self.selected = first_row;
        self.list_top = 0;
    }

    pub fn insert_char(&mut self, ch: char) {
        let byte_index = char_index_to_byte(&self.query, self.cursor);
        self.query.insert(byte_index, ch);
        self.cursor += 1;
    }

    pub fn delete_before_cursor(&mut self) {
        if self.cursor == 0 {
            return;
        }

        let byte_index = char_index_to_byte(&self.query, self.cursor - 1);
        self.query.remove(byte_index);
        self.cursor -= 1;
    }

    pub fn delete_at_cursor(&mut self) {
        let char_count = self.query.chars().count();
        if self.cursor >= char_count {
            return;
        }

        let byte_index = char_index_to_byte(&self.query, self.cursor);
        self.query.remove(byte_index);
    }

    pub fn move_selection(
        &mut self,
        row_count: usize,
        delta: i32,
        is_selectable: impl Fn(usize) -> bool,
    ) {
        if row_count == 0 {
            return;
        }

        let start = self.selected;
        let mut index = start as i32;

        loop {
            index = (index + delta).clamp(0, row_count as i32 - 1);
            if is_selectable(index as usize) {
                self.selected = index as usize;
                break;
            }
            if index as usize == start {
                break;
            }
        }
    }

    pub fn clamp_selection(&mut self, row_count: usize) {
        if row_count == 0 {
            self.selected = 0;
            return;
        }

        if self.selected >= row_count {
            self.selected = row_count - 1;
        }
    }

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

pub fn query_tokens(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

pub fn matches_query(query: &str, haystack: &str) -> bool {
    let tokens = query_tokens(query);
    if tokens.is_empty() {
        return true;
    }

    let haystack = haystack.to_ascii_lowercase();
    tokens.iter().all(|token| haystack.contains(token))
}

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

fn char_index_to_byte(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map_or(text.len(), |(index, _)| index)
}

fn first_selectable_row(row_count: usize, is_selectable: &impl Fn(usize) -> bool) -> usize {
    (0..row_count)
        .find(|&index| is_selectable(index))
        .unwrap_or(0)
}
