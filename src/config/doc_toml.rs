#![deny(clippy::unwrap_used)]

use toml::Value;

const MAX_INLINE_WIDTH: usize = 80usize;
const SECTION_RULE_WIDTH: usize = 78usize;

pub struct DocItem {
    pub key: String,
    pub doc: &'static [&'static str],
    pub value: Value,
    /// Rendered as a commented-out example line, so the key is documented but stays unset.
    pub commented: bool,
}

pub struct DocSection {
    pub title: &'static str,
    /// `None` renders the items as top-level keys; top-level sections must come before tables.
    pub table: Option<&'static str>,
    pub doc: &'static [&'static str],
    pub items: Vec<DocItem>,
}

impl DocItem {
    pub fn set(key: &str, doc: &'static [&'static str], value: impl Into<Value>) -> Self {
        Self {
            key: key.to_string(),
            doc,
            value: value.into(),
            commented: false,
        }
    }

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
