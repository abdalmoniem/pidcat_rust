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

//! Lexing log text into literal runs and CSI escape sequences.

#![deny(clippy::unwrap_used)]

use crate::ValueOrPanic;

/// One contiguous run of plain text or a single escape sequence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnsiToken {
    /// Visible characters with no leading ESC.
    Text(String),
    /// A complete CSI sequence starting with `\x1b[` through its final letter.
    Escape(String),
}

/// Splits `text` into alternating [`AnsiToken::Text`] and [`AnsiToken::Escape`] pieces.
///
/// Only CSI sequences introduced by `\x1b[` are recognized; other ESC forms remain in text.
pub fn tokenize_ansi(text: &str) -> Vec<AnsiToken> {
    let mut tokens = Vec::default();
    let mut current_text = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            if !current_text.is_empty() {
                tokens.push(AnsiToken::Text(std::mem::take(&mut current_text)));
            }
            tokens.push(AnsiToken::Escape(read_csi_sequence(&mut chars)));
        } else {
            current_text.push(ch);
        }
    }

    if !current_text.is_empty() {
        tokens.push(AnsiToken::Text(current_text));
    }

    tokens
}

/// Consumes `[` through the terminating alphabetic CSI parameter byte.
///
/// `chars` must already have consumed ESC and have `[` as the next character.
fn read_csi_sequence(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut code = String::from("\x1b");
    code.push(
        chars
            .next()
            .unwrap_or_panic("CSI sequence must follow ESC when peeked"),
    );

    while let Some(&next_ch) = chars.peek() {
        code.push(
            chars
                .next()
                .unwrap_or_panic("CSI sequence must terminate with a letter"),
        );
        if next_ch.is_ascii_alphabetic() {
            break;
        }
    }

    code
}
