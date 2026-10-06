#![deny(clippy::unwrap_used)]

use std::fs::File;

use colored::Color;
use colored::Colorize;

use scope_functions::Run;

use crate::ValueOrPanic;
use crate::Writer;

pub fn colored(msg: &str, show_colors: bool, color: Color) -> String {
    msg.run(|msg| match show_colors {
        true => msg.color(color).bold().to_string(),
        false => msg.to_string(),
    })
}

pub fn trim_log_line_bytes(raw: &[u8]) -> String {
    String::from_utf8_lossy(raw)
        .trim_end_matches(['\r', '\n'])
        .to_string()
}

pub fn trim_log_line(raw: &str) -> String {
    raw.trim_end_matches(['\r', '\n']).to_string()
}

pub fn format_usize_separated(value: usize) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

pub fn split_csv_values(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn split_csv_args(values: &[String]) -> Vec<String> {
    values
        .iter()
        .flat_map(|value| split_csv_values(value))
        .collect()
}

pub fn open_output_writer(path: &str) -> Writer {
    Writer::new_file(File::create(path).unwrap_or_panic("Failed to create output file"))
}
