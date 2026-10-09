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

//! Timestamp formatting for log columns and parsing from logcat lines.

use std::fmt::Write;

use chrono::DateTime;
use chrono::Datelike;
use chrono::Local;
use chrono::NaiveDateTime;
use chrono::TimeZone;

/// Default strftime pattern for the time column when none is configured.
pub const DEFAULT_TIMESTAMP_FORMAT: &str = "%I:%M:%S%.3f%P";

/// Link appended to format validation errors for user-facing help.
pub const CHRONO_STRFTIME_DOCS: &str =
    "https://docs.rs/chrono/latest/chrono/format/strftime/index.html";

/// Builds a user-facing validation error including [`CHRONO_STRFTIME_DOCS`].
fn timestamp_format_error(detail: impl std::fmt::Display) -> String {
    format!("{detail}\nSee {CHRONO_STRFTIME_DOCS}")
}

/// Renders `time` with chrono's `format` and maps write failures to validation errors.
fn format_datetime(format: &str, time: DateTime<Local>) -> Result<String, String> {
    let mut output = String::new();
    write!(output, "{}", time.format(format))
        .map_err(|_| timestamp_format_error(format!("invalid timestamp format '{format}'")))?;
    Ok(output)
}

/// Adds one fixed local datetime to `samples` when `with_ymd_and_hms` succeeds.
fn push_sample(samples: &mut Vec<DateTime<Local>>, year: i32, month: u32, day: u32, hour: u32) {
    if let Some(sample) = Local
        .with_ymd_and_hms(year, month, day, hour, 30, 45)
        .single()
    {
        samples.push(sample);
    }
}

/// Representative datetimes used to measure worst-case formatted width.
fn timestamp_validation_samples() -> Vec<DateTime<Local>> {
    let mut samples = vec![Local::now()];

    for year in [2024i32, 2025, 2026] {
        for month in 1u32..=12u32 {
            for day in [1u32, 10, 15, 20, 28] {
                for hour in [0u32, 6, 12, 18, 23] {
                    push_sample(&mut samples, year, month, day, hour);
                }
            }
        }
    }

    for date in ["01-02", "02-28", "06-15", "09-09", "12-31"] {
        for time in [
            "03:04:05.123",
            "11:59:59.000",
            "12:00:00.000",
            "23:59:59.999",
        ] {
            if let Some(parsed) = parse_android_log_timestamp(date, time) {
                samples.push(parsed);
            }
        }
    }

    samples
}

/// Validates chrono strftime `format` and returns the timestamp column width (maximum rendered
/// width over representative date/times). Shorter values are right-padded when displayed.
pub fn timestamp_column_width(format: &str) -> Result<usize, String> {
    let mut max_width = 0usize;

    for sample in timestamp_validation_samples() {
        let formatted = format_datetime(format, sample)?;
        max_width = max_width.max(formatted.chars().count());
    }

    if max_width == 0usize {
        return Err(timestamp_format_error(format!(
            "timestamp format '{format}' could not be validated"
        )));
    }

    Ok(max_width)
}

/// Formats `time` with `format`, enforces fixed `width`, and right-pads with spaces.
pub fn format_log_timestamp(
    time: DateTime<Local>,
    format: &str,
    width: usize,
) -> Result<String, String> {
    let mut formatted = format_datetime(format, time)?;
    let char_count = formatted.chars().count();
    if char_count > width {
        return Err(timestamp_format_error(format!(
            "timestamp format '{format}' produced {char_count} characters, wider than the \
             timestamp column ({width}); choose a narrower format or a format with less \
             variation"
        )));
    }

    if char_count < width {
        formatted.push_str(&" ".repeat(width - char_count));
    }

    Ok(formatted)
}

/// Parses Android log date (`MM-DD`) and time fields using the current local year.
pub fn parse_android_log_timestamp(date: &str, time: &str) -> Option<DateTime<Local>> {
    let year = Local::now().year();
    let trimmed_time = time.trim();
    let patterns = [
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M:%S%.3f",
    ];

    for pattern in patterns {
        let candidate = format!("{year}-{date} {trimmed_time}");
        if let Ok(naive) = NaiveDateTime::parse_from_str(&candidate, pattern) {
            return Local.from_local_datetime(&naive).single();
        }
    }

    None
}

/// Extracts timestamp from `line` using [`crate::CliArgs`] log format regex, or [`Local::now`].
pub fn timestamp_from_log_line(args: &crate::CliArgs, line: &str) -> DateTime<Local> {
    if let (Some(date_index), Some(time_index)) = (
        args.log_format.date_index().as_ref(),
        args.log_format.time_index().as_ref(),
    ) && let Some(captures) = args.log_format.regex().captures(line)
        && let (Some(date), Some(time)) = (captures.get(*date_index), captures.get(*time_index))
        && let Some(parsed) = parse_android_log_timestamp(date.as_str(), time.as_str())
    {
        return parsed;
    }

    Local::now()
}
