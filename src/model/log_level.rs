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

//! Android log priority levels and CLI integration.

use clap::ValueEnum;

use clap::builder::PossibleValue;

use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

/// Logcat priority ordered from most verbose to most severe.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum LogLevel {
    /// Verbose (`V`).
    #[default]
    VERBOSE = 0isize,
    /// Debug (`D`).
    DEBUG = 1isize,
    /// Info (`I`).
    INFO = 2isize,
    /// Warning (`W`).
    WARN = 3isize,
    /// Error (`E`).
    ERROR = 4isize,
    /// Fatal (`F`).
    FATAL = 5isize,
}

impl From<&str> for LogLevel {
    /// Parses a single-letter logcat priority (`V`, `D`, `I`, `W`, `E`, `F`).
    ///
    /// # Panics
    ///
    /// Panics if the letter is not recognized.
    fn from(str: &str) -> Self {
        match str {
            "V" => Self::VERBOSE,
            "D" => Self::DEBUG,
            "I" => Self::INFO,
            "W" => Self::WARN,
            "E" => Self::ERROR,
            "F" => Self::FATAL,
            _ => panic!("Invalid log level"),
        }
    }
}

impl From<String> for LogLevel {
    /// Parses owned priority text via [`From<&str>`].
    fn from(str: String) -> Self {
        Self::from(str.as_str())
    }
}

impl Display for LogLevel {
    /// Writes the single-letter priority (`V`…`F`).
    fn fmt(&self, formatter: &mut Formatter) -> Result {
        let letter = match self {
            Self::VERBOSE => "V",
            Self::DEBUG => "D",
            Self::INFO => "I",
            Self::WARN => "W",
            Self::ERROR => "E",
            Self::FATAL => "F",
        };
        write!(formatter, "{letter}")
    }
}

impl LogLevel {
    /// Parses CLI/TUI names such as `verbose`, `w`, or `ERROR` (ASCII case-insensitive).
    pub fn parse_name(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "v" | "verbose" => Some(Self::VERBOSE),
            "d" | "debug" => Some(Self::DEBUG),
            "i" | "info" => Some(Self::INFO),
            "w" | "warn" => Some(Self::WARN),
            "e" | "error" => Some(Self::ERROR),
            "f" | "fatal" => Some(Self::FATAL),
            _ => None,
        }
    }

    /// Lowercase filter keyword used in config and TUI (`verbose`, `debug`, …).
    pub fn filter_name(self) -> &'static str {
        match self {
            Self::VERBOSE => "verbose",
            Self::DEBUG => "debug",
            Self::INFO => "info",
            Self::WARN => "warn",
            Self::ERROR => "error",
            Self::FATAL => "fatal",
        }
    }
}

impl ValueEnum for LogLevel {
    /// All variants exposed on the command line.
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::VERBOSE,
            Self::DEBUG,
            Self::INFO,
            Self::WARN,
            Self::ERROR,
            Self::FATAL,
        ]
    }

    /// clap name, aliases, and help text for one variant.
    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            Self::VERBOSE => PossibleValue::new("V").alias("verbose").help("verbose"),
            Self::DEBUG => PossibleValue::new("D").alias("debug").help("debug"),
            Self::INFO => PossibleValue::new("I").alias("info").help("info"),
            Self::WARN => PossibleValue::new("W").alias("warn").help("warn"),
            Self::ERROR => PossibleValue::new("E").alias("error").help("error"),
            Self::FATAL => PossibleValue::new("F").alias("fatal").help("fatal"),
        })
    }
}
