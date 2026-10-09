// Copyright (C) AbdAlMoniem AlHifnawy <hifnawy_moniem@hotmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

#![deny(clippy::unwrap_used)]

//! Renders TOML configuration and theme files with comment blocks describing each key.

use toml::Value;

/// Maximum character width for a single-line `key = value` assignment before arrays are broken out.
const MAX_INLINE_WIDTH: usize = 80usize;
/// Width of the dashed rule drawn in comments between documentation sections.
const SECTION_RULE_WIDTH: usize = 78usize;

/// One documented TOML key with comment lines and an optional commented-out example value.
pub struct DocItem {
    /// TOML key name (kebab-case as in the file).
    pub key: String,
    /// Comment lines printed immediately above the key.
    pub doc: &'static [&'static str],
    /// Value to emit as an active assignment or as a commented example.
    pub value: Value,
    /// When true, the assignment is rendered as a commented-out example line so the key stays unset.
    pub commented: bool,
}

/// A group of related [`DocItem`] entries, optionally under a TOML table header.
pub struct DocSection {
    /// Section title printed in the comment rule header.
    pub title: &'static str,
    /// `None` renders items as top-level keys; `Some("table")` emits `[table]` first.
    pub table: Option<&'static str>,
    /// Introductory comment lines for the whole section.
    pub doc: &'static [&'static str],
    /// Keys documented in this section, in emission order.
    pub items: Vec<DocItem>,
}

impl DocItem {
    /// Builds an active (non-commented) item with the given key, comments, and TOML value.
    pub fn set(key: &str, doc: &'static [&'static str], value: impl Into<Value>) -> Self {
        Self {
            key: key.to_string(),
            doc,
            value: value.into(),
            commented: false,
        }
    }

    /// Uses `value` when set; otherwise emits a commented example using `example`.
    pub fn optional<T: Into<Value>>(
        key: &str,
        doc: &'static [&'static str],
        value: Option<T>,
        example: impl Into<Value>,
    ) -> Self {
        match value {
            Some(value) => Self::set(key, doc, value),
            None => Self {
                commented: true,
                ..Self::set(key, doc, example)
            },
        }
    }
}

/// Appends one `#` comment line to `out`, using a bare `#` for empty lines.
fn push_comment(out: &mut String, line: &str) {
    match line.is_empty() {
        true => out.push_str("#\n"),
        false => {
            out.push_str("# ");
            out.push_str(line);
            out.push('\n');
        }
    }
}

/// Formats a TOML key as bare identifier or quoted string when required.
fn render_key(key: &str) -> String {
    let is_bare = !key.is_empty()
        && key
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || char == '-' || char == '_');

    match is_bare {
        true => key.to_string(),
        false => Value::String(key.to_string()).to_string(),
    }
}

/// Formats `key = value`, expanding long arrays across multiple lines.
fn render_value(key: &str, value: &Value) -> String {
    let key = render_key(key);
    let inline = format!("{key} = {value}");

    match value {
        Value::Array(items) if inline.chars().count() > MAX_INLINE_WIDTH => {
            let mut out = format!("{key} = [\n");
            for item in items {
                out.push_str(&format!("    {item},\n"));
            }
            out.push(']');
            out
        }
        _ => inline,
    }
}

/// Renders a full documented TOML document from a header comment block and [`DocSection`] list.
pub fn render(header: &[String], sections: &[DocSection]) -> String {
    let mut out = String::default();
    let rule = "-".repeat(SECTION_RULE_WIDTH);

    for line in header {
        push_comment(&mut out, line);
    }

    for section in sections {
        out.push('\n');
        push_comment(&mut out, &rule);
        push_comment(&mut out, section.title);
        push_comment(&mut out, &rule);
        for line in section.doc {
            push_comment(&mut out, line);
        }

        if let Some(table) = section.table {
            out.push_str(&format!("[{table}]\n"));
        }

        for (index, item) in section.items.iter().enumerate() {
            if index == 0usize || !item.doc.is_empty() {
                out.push('\n');
            }
            for line in item.doc {
                push_comment(&mut out, line);
            }

            match item.commented {
                true => push_comment(
                    &mut out,
                    &format!(
                        "{key} = {value}",
                        key = render_key(&item.key),
                        value = item.value
                    ),
                ),
                false => {
                    out.push_str(&render_value(&item.key, &item.value));
                    out.push('\n');
                }
            }
        }
    }

    out
}
