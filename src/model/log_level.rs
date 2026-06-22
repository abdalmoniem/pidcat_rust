use clap::ValueEnum;

use clap::builder::PossibleValue;

use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum LogLevel {
    #[default]
    VERBOSE = 0isize,
    DEBUG = 1isize,
    INFO = 2isize,
    WARN = 3isize,
    ERROR = 4isize,
    FATAL = 5isize,
}

impl From<&str> for LogLevel {
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
    fn from(str: String) -> Self {
        Self::from(str.as_str())
    }
}

impl Display for LogLevel {
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

impl ValueEnum for LogLevel {
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
