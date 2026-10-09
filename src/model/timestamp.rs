use chrono::DateTime;
use chrono::Datelike;
use chrono::Local;
use chrono::NaiveDateTime;
use chrono::TimeZone;

pub const DEFAULT_TIMESTAMP_FORMAT: &str = "%I:%M:%S%.3f%P";

/// Validates that `format` renders a fixed width and returns that width in characters.
pub fn timestamp_column_width(format: &str) -> Result<usize, String> {
    let samples = [
        Local
            .with_ymd_and_hms(2024, 1, 2, 3, 4, 5)
            .single()
            .expect("sample timestamp"),
        Local
            .with_ymd_and_hms(2024, 12, 31, 11, 59, 59)
            .single()
            .expect("sample timestamp"),
    ];

    let mut widths = Vec::default();
    for sample in samples {
        let formatted = sample.format(format).to_string();
        widths.push(formatted.chars().count());
    }

    let width = *widths.first().unwrap_or(&0usize);
    if widths.iter().all(|value| *value == width) {
        Ok(width)
    } else {
        Err(format!(
            "timestamp format '{format}' must produce a fixed width; got {widths:?} characters"
        ))
    }
}

pub fn format_log_timestamp(
    time: DateTime<Local>,
    format: &str,
    width: usize,
) -> Result<String, String> {
    let formatted = time.format(format).to_string();
    let char_count = formatted.chars().count();
    if char_count != width {
        return Err(format!(
            "timestamp format '{format}' produced {char_count} characters, expected {width}"
        ));
    }

    Ok(formatted)
}

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
