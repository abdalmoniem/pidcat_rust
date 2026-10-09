// Copyright (c) AbdAlMoniem AlHifnawy <hifnawy_moniem@hotmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

//! ANSI escape sequence handling for colored log text.
//!
//! This module provides two related facilities:
//!
//! - locating CSI escape sequences in a string and re-inserting them into slices of the plain
//!   text (`get_ansi_segments`, `get_active_codes_at_pos`, `insert_ansi_codes_in_range`),
//!   which is what line wrapping needs to keep colors intact across line breaks;
//! - converting ANSI-colored text into a ratatui `Line` of styled `Span`s
//!   (`line_from_ansi`) for display in the TUI.
//!
//! Only CSI sequences (`ESC [ ... <letter>`) are recognized. Positions are counted in
//! characters of the visible text (escape sequences excluded).
//!
//! Note: this file is currently not declared as a module in `controller/mod.rs`, so it is not
//! part of the compiled crate; equivalent helpers live in
//! `log_processor` and the TUI.

#![deny(clippy::unwrap_used)]

use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;

use crate::AnsiSegment;
use crate::ValueOrPanic;

/// One item produced while walking over text that may contain ANSI escape sequences.
enum AnsiWalkEvent {
    /// A visible character.
    Char(char),
    /// A complete escape sequence.
    Sequence {
        /// Number of visible characters that precede the sequence in the text.
        position: usize,
        /// The full sequence text, including the leading `\x1b`.
        code: String,
    },
}

/// Returns an iterator over `text` that yields visible characters and escape sequences in order.
///
/// See [`AnsiWalk`] for how sequences are recognized.
fn iter_ansi_walk(text: &str) -> impl Iterator<Item = AnsiWalkEvent> + '_ {
    AnsiWalk {
        chars: text.chars().peekable(),
        position: 0,
    }
}

/// Iterator state for [`iter_ansi_walk`].
///
/// A sequence starts at `\x1b` immediately followed by `[` and extends through the first ASCII
/// alphabetic character after it (or to the end of the text if there is none). A lone `\x1b`
/// that is not followed by `[` is treated as a visible character.
struct AnsiWalk<'a> {
    /// Remaining characters of the input, with one character of lookahead.
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    /// Count of visible characters yielded so far.
    position: usize,
}

/// Walks the text one visible character or escape sequence at a time.
impl Iterator for AnsiWalk<'_> {
    /// Either a visible character or a whole escape sequence.
    type Item = AnsiWalkEvent;

    /// Yields the next visible character or escape sequence, or `None` at the end of the text.
    ///
    /// Escape sequences do not advance the visible `position`; characters do.
    fn next(&mut self) -> Option<Self::Item> {
        let ch = self.chars.next()?;

        if ch == '\x1b' && self.chars.peek() == Some(&'[') {
            let position = self.position;
            let mut code = String::from("\x1b");
            code.push(
                self.chars
                    .next()
                    .unwrap_or_panic("Unexpected end of input after ESC"),
            );

            while let Some(&next_ch) = self.chars.peek() {
                code.push(
                    self.chars
                        .next()
                        .unwrap_or_panic("Unexpected end of input in ANSI code"),
                );
                if next_ch.is_ascii_alphabetic() {
                    break;
                }
            }

            return Some(AnsiWalkEvent::Sequence { position, code });
        }

        self.position += 1;
        Some(AnsiWalkEvent::Char(ch))
    }
}

/// Finds every escape sequence in `text` together with the index of the visible character it
/// precedes.
///
/// The returned segments are in text order. A segment's `pos` is the number of visible
/// characters before it, so sequences at the end of the text have `pos` equal to the visible
/// length.
pub fn get_ansi_segments(text: &str) -> Vec<AnsiSegment> {
    iter_ansi_walk(text)
        .filter_map(|event| match event {
            AnsiWalkEvent::Sequence { position, code } => {
                Some(AnsiSegment { pos: position, code })
            }
            AnsiWalkEvent::Char(_) => None,
        })
        .collect()
}

/// Returns the escape sequences still in effect just before visible position `pos`.
///
/// Only segments strictly before `pos` are considered (iteration stops at the first segment with
/// `pos >= pos`, so `segments` must be ordered). A segment containing `0m` is treated as a reset
/// and clears everything collected so far; every other segment is appended. The result can be
/// emitted at the start of a continuation line to restore the active colors.
pub fn get_active_codes_at_pos(segments: &[AnsiSegment], pos: usize) -> Vec<String> {
    let mut active = Vec::default();

    for seg in segments {
        if seg.pos >= pos {
            break;
        }

        if seg.code.contains("0m") {
            active.clear();
        } else {
            active.push(seg.code.clone());
        }
    }

    active
}

/// Rebuilds a colored slice of the text.
///
/// `plain_text` is the already-sliced visible text covering the visible range
/// `start_pos..end_pos` of the original string. The output starts with all `active_codes`, then
/// contains the characters of `plain_text` with each segment from `segments` re-inserted before
/// the character at its original position. Segments positioned at or after `end_pos` are not
/// inserted, and segments before `start_pos` are skipped (they are expected to be represented by
/// `active_codes`).
pub fn insert_ansi_codes_in_range(
    plain_text: &str,
    segments: &[AnsiSegment],
    start_pos: usize,
    end_pos: usize,
    active_codes: &[String],
) -> String {
    let mut result = String::default();
    let chars: Vec<char> = plain_text.chars().collect();

    for code in active_codes {
        result.push_str(code);
    }

    let mut segment_idx = 0usize;

    while segment_idx < segments.len() && segments[segment_idx].pos < start_pos {
        segment_idx += 1usize;
    }

    for (index, char) in chars.iter().enumerate() {
        let absolute_pos = start_pos + index;

        while segment_idx < segments.len() {
            let seg = &segments[segment_idx];

            if seg.pos >= end_pos {
                break;
            }

            if seg.pos == absolute_pos {
                result.push_str(&seg.code);
                segment_idx += 1usize;
            } else if seg.pos > absolute_pos {
                break;
            } else {
                segment_idx += 1usize;
            }
        }

        result.push(*char);
    }

    result
}

/// Converts ANSI-colored `text` into a ratatui [`Line`] of styled [`Span`]s.
///
/// Text between escape sequences becomes one span styled by the sequences seen so far (see
/// `apply_ansi_code`); the escape sequences themselves do not appear in the output. The returned
/// line owns its data (`'static`).
pub fn line_from_ansi(text: &str) -> Line<'static> {
    let mut spans = Vec::default();
    let mut current = String::new();
    let mut style = Style::default();

    for event in iter_ansi_walk(text) {
        match event {
            AnsiWalkEvent::Char(ch) => current.push(ch),
            AnsiWalkEvent::Sequence { code, .. } => {
                if !current.is_empty() {
                    spans.push(Span::styled(std::mem::take(&mut current), style));
                }
                style = apply_ansi_code(style, &code);
            }
        }
    }

    if !current.is_empty() {
        spans.push(Span::styled(current, style));
    }

    Line::from(spans)
}

/// Applies one SGR escape sequence (`ESC [ ... m`) to `base` and returns the new style.
///
/// Supported parameters, which may be combined with `;` in a single sequence:
///
/// - `0` resets the style;
/// - `38;2;r;g;b` / `48;2;r;g;b` set a true-color foreground / background;
/// - `38;5;n` / `48;5;n` set a 256-color foreground / background;
/// - everything else is passed to `apply_basic_sgr` (bold, 16 colors, default colors).
///
/// Sequences that do not start with `ESC [`, empty sequences, and parameters that cannot be
/// parsed leave the style unchanged.
fn apply_ansi_code(mut base: Style, code: &str) -> Style {
    if !code.starts_with("\x1b[") {
        return base;
    }

    let inner = code.trim_start_matches("\x1b[").trim_end_matches('m');
    if inner.is_empty() {
        return base;
    }

    let parts: Vec<&str> = inner.split(';').collect();
    let mut index = 0usize;

    while index < parts.len() {
        if parts[index] == "0" {
            base = Style::default();
            index += 1;
            continue;
        }

        if parts[index] == "38" && index + 1 < parts.len() {
            if parts[index + 1] == "2" && index + 4 < parts.len() {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    parts[index + 2].parse::<u8>(),
                    parts[index + 3].parse::<u8>(),
                    parts[index + 4].parse::<u8>(),
                ) {
                    base = base.fg(Color::Rgb(r, g, b));
                }
                index += 5;
                continue;
            }
            if parts[index + 1] == "5" && index + 2 < parts.len() {
                if let Ok(color_index) = parts[index + 2].parse::<u8>() {
                    base = base.fg(ansi256_to_color(color_index));
                }
                index += 3;
                continue;
            }
        }

        if parts[index] == "48" && index + 1 < parts.len() {
            if parts[index + 1] == "2" && index + 4 < parts.len() {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    parts[index + 2].parse::<u8>(),
                    parts[index + 3].parse::<u8>(),
                    parts[index + 4].parse::<u8>(),
                ) {
                    base = base.bg(Color::Rgb(r, g, b));
                }
                index += 5;
                continue;
            }
            if parts[index + 1] == "5" && index + 2 < parts.len() {
                if let Ok(color_index) = parts[index + 2].parse::<u8>() {
                    base = base.bg(ansi256_to_color(color_index));
                }
                index += 3;
                continue;
            }
        }

        if let Ok(value) = parts[index].parse::<u16>() {
            base = apply_basic_sgr(base, value);
        }

        index += 1;
    }

    base
}

/// Applies a single basic SGR parameter to `style`.
///
/// Handles bold on (`1`) and off (`22`), the eight standard foreground (`30`-`37`) and
/// background (`40`-`47`) colors, their bright variants (`90`-`97`, `100`-`107`), and the
/// default foreground (`39`) and background (`49`) colors. Bright white (`97`, `107`) maps to
/// [`Color::White`]. Unknown parameters return `style` unchanged.
fn apply_basic_sgr(style: Style, code: u16) -> Style {
    match code {
        1 => style.add_modifier(Modifier::BOLD),
        22 => style.remove_modifier(Modifier::BOLD),
        30 => style.fg(Color::Black),
        31 => style.fg(Color::Red),
        32 => style.fg(Color::Green),
        33 => style.fg(Color::Yellow),
        34 => style.fg(Color::Blue),
        35 => style.fg(Color::Magenta),
        36 => style.fg(Color::Cyan),
        37 => style.fg(Color::White),
        39 => style.fg(Color::Reset),
        40 => style.bg(Color::Black),
        41 => style.bg(Color::Red),
        42 => style.bg(Color::Green),
        43 => style.bg(Color::Yellow),
        44 => style.bg(Color::Blue),
        45 => style.bg(Color::Magenta),
        46 => style.bg(Color::Cyan),
        47 => style.bg(Color::White),
        49 => style.bg(Color::Reset),
        90 => style.fg(Color::DarkGray),
        91 => style.fg(Color::LightRed),
        92 => style.fg(Color::LightGreen),
        93 => style.fg(Color::LightYellow),
        94 => style.fg(Color::LightBlue),
        95 => style.fg(Color::LightMagenta),
        96 => style.fg(Color::LightCyan),
        97 => style.fg(Color::White),
        100 => style.bg(Color::DarkGray),
        101 => style.bg(Color::LightRed),
        102 => style.bg(Color::LightGreen),
        103 => style.bg(Color::LightYellow),
        104 => style.bg(Color::LightBlue),
        105 => style.bg(Color::LightMagenta),
        106 => style.bg(Color::LightCyan),
        107 => style.bg(Color::White),
        _ => style,
    }
}

/// Converts an xterm 256-color palette `index` to a ratatui [`Color`].
///
/// - `0..=15` are the standard and bright colors, resolved through the corresponding basic SGR
///   foreground codes (falling back to [`Color::White`]);
/// - `16..=231` form a 6x6x6 RGB cube whose channel levels are `0` or `55 + 40 * level`;
/// - `232..=255` are the 24-step grayscale ramp, `8 + 10 * step`.
fn ansi256_to_color(index: u8) -> Color {
    match index {
        0..=15 => {
            let code = match index {
                0 => 30,
                1 => 31,
                2 => 32,
                3 => 33,
                4 => 34,
                5 => 35,
                6 => 36,
                7 => 37,
                8 => 90,
                9 => 91,
                10 => 92,
                11 => 93,
                12 => 94,
                13 => 95,
                14 => 96,
                _ => 97,
            };
            apply_basic_sgr(Style::default(), code)
                .fg
                .unwrap_or(Color::White)
        }
        232..=255 => {
            let gray = 8 + (index - 232) * 10;
            Color::Rgb(gray, gray, gray)
        }
        _ => {
            let index = index - 16;
            let r = index / 36;
            let g = (index / 6) % 6;
            let b = index % 6;
            let channel = |value: u8| {
                if value == 0 { 0 } else { 55 + value * 40 }
            };
            Color::Rgb(channel(r), channel(g), channel(b))
        }
    }
}
