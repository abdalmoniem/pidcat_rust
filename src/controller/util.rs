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

//! Small shared helpers used across the controllers.
//!
//! This module groups stateless utilities that do not belong to a larger subsystem:
//!
//! - colored message formatting and fatal-error exit (`colored`, `exit_with_error`);
//! - multi-column listing and pager output (`format_columns`, `print_paged`);
//! - log-line trimming and number formatting (`trim_log_line_bytes`, `trim_log_line`,
//!   `format_usize_separated`);
//! - comma-separated value splitting (`split_csv_values`, `split_csv_args`);
//! - opening the `--output` file as a `Writer` (`open_output_writer`).

#![deny(clippy::unwrap_used)]

use std::env;
use std::fs::File;
use std::io::Write;
use std::process;
use std::process::Stdio;

use colored::Color;
use colored::Colorize;

use is_terminal::IsTerminal;

use scope_functions::Run;

use crate::ValueOrPanic;
use crate::Writer;
use crate::controller::terminal::restore_tui_terminal;

/// Formats `msg` in `color` and bold when `show_colors` is `true`, otherwise returns it
/// unchanged as an owned string.
///
/// The result contains ANSI escape sequences only when `show_colors` is `true`.
pub fn colored(msg: &str, show_colors: bool, color: Color) -> String {
    msg.run(|msg| match show_colors {
        true => msg.color(color).bold().to_string(),
        false => msg.to_string(),
    })
}

/// Prints `ERROR: {msg}` to standard error and terminates the process with exit code `1`.
///
/// Any active TUI terminal state is restored first (see [`restore_tui_terminal`]) so the message
/// is readable on the normal screen. The message is shown in bold bright red when `show_colors`
/// is `true`. Standard error is flushed before exiting.
pub fn exit_with_error(msg: &str, show_colors: bool) -> ! {
    restore_tui_terminal();

    let err_msg = format!("ERROR: {msg}").run(|msg| colored(msg, show_colors, Color::BrightRed));

    eprintln!("{err_msg}");
    let _ = std::io::stderr().flush();
    process::exit(1i32);
}

/// Pager program used when `$PAGER` is unset or blank: `more` on Windows, `less` elsewhere.
const DEFAULT_PAGER: &str = match cfg!(windows) {
    true => "more",
    false => "less",
};
/// Value given to `$LESS` when it is not already set: quit immediately if the output fits on one
/// screen (`F`), pass ANSI colors through (`R`), and do not clear the screen on exit (`X`).
const DEFAULT_LESS_FLAGS: &str = "FRX";
/// Number of blank columns separating adjacent columns in [`format_columns`].
const COLUMN_GAP: usize = 2;
/// Terminal size, as `(width, height)`, assumed when the real size cannot be determined.
const FALLBACK_TERMINAL_SIZE: (usize, usize) = (80, 24);

/// Lays `items` out in as many columns as fit the terminal width, each as wide as its longest
/// item, filled top to bottom within blocks one screen high (minus the pager prompt line), so
/// each pager page reads down its columns.
///
/// The terminal size is queried from the OS and falls back to 80x24 when unavailable. Columns
/// are separated by two spaces, trailing whitespace is trimmed from every line, and each line
/// ends with `\n`. When even a single column does not fit the width, one column is used anyway.
/// An empty `items` slice yields an empty string.
pub fn format_columns(items: &[String]) -> String {
    let (width, height) = terminal_size::terminal_size()
        .map(
            |(terminal_size::Width(width), terminal_size::Height(height))| {
                (width as usize, height as usize)
            },
        )
        .unwrap_or(FALLBACK_TERMINAL_SIZE);

    let page_rows = height.saturating_sub(1).max(1);

    let fits = |widths: &[usize]| {
        let used = widths.iter().filter(|width| **width > 0).count();
        widths.iter().sum::<usize>() + COLUMN_GAP * used.saturating_sub(1) <= width
    };

    let (columns, widths) = (1..=items.len().max(1))
        .rev()
        .map(|columns| (columns, column_widths(items, columns, page_rows)))
        .find(|(_, widths)| fits(widths))
        .unwrap_or_else(|| (1, column_widths(items, 1, page_rows)));

    items
        .chunks(columns * page_rows)
        .flat_map(|page| {
            let rows = page.len().div_ceil(columns);
            let widths = &widths;

            (0..rows).map(move |row| {
                let line = (0..columns)
                    .filter_map(|column| {
                        page.get(column * rows + row).map(|item| {
                            format!("{item:<width$}", width = widths[column] + COLUMN_GAP)
                        })
                    })
                    .collect::<String>();

                format!("{}\n", line.trim_end())
            })
        })
        .collect()
}

/// The width of each column when `items` are laid out in `columns` columns within page blocks
/// of up to `page_rows` rows, taken over all page blocks.
///
/// Widths are measured in characters (not bytes). The returned vector always has `columns`
/// entries; columns that receive no items have width `0`.
fn column_widths(items: &[String], columns: usize, page_rows: usize) -> Vec<usize> {
    let mut widths = vec![0usize; columns];

    for page in items.chunks(columns * page_rows) {
        let rows = page.len().div_ceil(columns);

        for (index, item) in page.iter().enumerate() {
            let width = &mut widths[index / rows];
            *width = (*width).max(item.chars().count());
        }
    }

    widths
}

/// Shows `text` through `$PAGER` (or `less`) when stdout is a terminal, and prints it as is
/// otherwise or when the pager cannot be started, so piped output stays plain.
///
/// The pager command may include arguments (it is split on whitespace). `$LESS` is passed to the
/// pager, defaulting to `FRX` when unset. A pager that exits before consuming all input (for
/// example when the user quits early) is not treated as an error.
pub fn print_paged(text: &str) {
    if !std::io::stdout().is_terminal() {
        print!("{text}");
        return;
    }

    let pager = env::var("PAGER")
        .ok()
        .filter(|pager| !pager.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_PAGER.to_string());

    let mut pager_args = pager.split_whitespace();

    let Some(program) = pager_args.next() else {
        print!("{text}");
        return;
    };

    let less_flags = env::var("LESS").unwrap_or_else(|_| DEFAULT_LESS_FLAGS.to_string());

    let spawned = process::Command::new(program)
        .args(pager_args)
        .env("LESS", less_flags)
        .stdin(Stdio::piped())
        .spawn();

    match spawned {
        Ok(mut child) => {
            if let Some(mut stdin) = child.stdin.take() {
                // The pager closes its input when quit early, which is not an error here.
                let _ = stdin.write_all(text.as_bytes());
            }

            let _ = child.wait();
        }
        Err(_) => print!("{text}"),
    }
}

/// Decodes a raw log line as UTF-8 (lossily) and removes any trailing `\r` and `\n` characters.
///
/// Invalid UTF-8 sequences are replaced with `U+FFFD`.
pub fn trim_log_line_bytes(raw: &[u8]) -> String {
    String::from_utf8_lossy(raw)
        .trim_end_matches(['\r', '\n'])
        .to_string()
}

/// Removes any trailing `\r` and `\n` characters from a log line.
pub fn trim_log_line(raw: &str) -> String {
    raw.trim_end_matches(['\r', '\n']).to_string()
}

/// Formats `value` in decimal with a comma between every group of three digits, for example
/// `1234567` becomes `"1,234,567"`.
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

/// Splits a comma-separated string into its parts.
///
/// Each part is trimmed of surrounding whitespace and empty parts are dropped, so
/// `"a, b,,c "` yields `["a", "b", "c"]`.
pub fn split_csv_values(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

/// Splits every string in `values` with [`split_csv_values`] and concatenates the results,
/// preserving order.
///
/// This lets repeated CLI arguments and comma-separated lists be used interchangeably.
pub fn split_csv_args(values: &[String]) -> Vec<String> {
    values
        .iter()
        .flat_map(|value| split_csv_values(value))
        .collect()
}

/// Creates (or truncates) the file at `path` and wraps it in a file-backed [`Writer`].
///
/// # Panics
///
/// Panics with `"Failed to create output file"` if the file cannot be created.
pub fn open_output_writer(path: &str) -> Writer {
    Writer::new_file(File::create(path).unwrap_or_panic("Failed to create output file"))
}
