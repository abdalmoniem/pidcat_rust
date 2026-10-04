#![deny(clippy::unwrap_used)]

use crate::ValueOrPanic;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnsiToken {
    Text(String),
    Escape(String),
}

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
