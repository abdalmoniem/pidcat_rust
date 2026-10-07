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

pub fn colored(msg: &str, show_colors: bool, color: Color) -> String {
    msg.run(|msg| match show_colors {
        true => msg.color(color).bold().to_string(),
        false => msg.to_string(),
    })
}

pub fn exit_with_error(msg: &str, show_colors: bool) -> ! {
    let err_msg = format!("ERROR: {msg}").run(|msg| colored(msg, show_colors, Color::BrightRed));

    eprintln!("{err_msg}");
    process::exit(1i32);
}

const DEFAULT_PAGER: &str = match cfg!(windows) {
    true => "more",
    false => "less",
};
const DEFAULT_LESS_FLAGS: &str = "FRX";
const COLUMN_GAP: usize = 2;
const FALLBACK_TERMINAL_SIZE: (usize, usize) = (80, 24);

/// Lays `items` out in as many columns as fit the terminal width, each as wide as its longest
/// item, filled top to bottom within blocks one screen high (minus the pager prompt line), so
/// each pager page reads down its columns.
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
