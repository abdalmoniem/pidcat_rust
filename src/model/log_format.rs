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
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Regex layouts for native Android `adb logcat -v` line formats.
//!
//! Each [`LogFormatKind`] maps to one line shape. [`LogFormatMatchConfig`] stores the compiled
//! pattern and capture-group indices for date, time, level, tag, pid, uid, tid, and message.
//!
//! # ADB format reference
//!
//! Single format verbs (from `adb logcat -v`):
//!
//! - **brief** — priority, tag, and PID of the issuing process.
//! - **long** — all metadata fields; messages separated by blank lines.
//! - **process** — PID only (plus level and message in practice).
//! - **raw** — message body only.
//! - **tag** — priority and tag only.
//! - **thread** — priority, PID, and TID of the issuing thread.
//! - **threadtime** — date, time, priority, tag, PID, and TID (adb default).
//! - **time** — date, time, priority, tag, and PID.

use clap::Arg;
use clap::Command;
use clap::Error;
use clap::ValueEnum;
use clap::builder::EnumValueParser;
use clap::builder::PossibleValue;
use clap::builder::TypedValueParser;

use regex::Regex;

use std::ffi::OsStr;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as fmtResult;
use std::result::Result;

/// Compiled regex and 1-based capture indices for one logcat `-v` layout.
#[derive(Clone, Debug)]
pub struct LogFormatMatchConfig {
    /// Whole-line matcher for this format.
    pub regex: Regex,
    /// Capture group index for the date portion, if present.
    pub date_index: Option<usize>,
    /// Capture group index for the time portion, if present.
    pub time_index: Option<usize>,
    /// Capture group index for the single-letter priority (`V`…`F`).
    pub level_index: Option<usize>,
    /// Capture group index for the log tag.
    pub tag_index: Option<usize>,
    /// Capture group index for the process ID.
    pub pid_index: Option<usize>,
    /// Capture group index for the user/application ID.
    pub uid_index: Option<usize>,
    /// Capture group index for the thread ID.
    pub tid_index: Option<usize>,
    /// Capture group index for the log message body.
    pub msg_index: Option<usize>,
}

/// Native adb logcat verbosity format (maps to `logcat -v` names).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LogFormatKind {
    /// `brief` format.
    Brief = 0isize,
    /// `long` format.
    Long = 1isize,
    /// `process` format.
    Process = 2isize,
    /// `raw` format.
    Raw = 3isize,
    /// `tag` format.
    Tag = 4isize,
    /// `thread` format.
    Thread = 5isize,
    /// `threadtime` format (Android default).
    ThreadTime = 6isize,
    /// `time` format.
    Time = 7isize,
}

/// Active log line parser: kind plus private match configuration.
#[derive(Clone, Debug)]
pub struct LogFormat {
    /// Selected adb verbosity kind.
    pub kind: LogFormatKind,
    match_cfg: LogFormatMatchConfig,
}

/// clap [`TypedValueParser`] that accepts [`LogFormatKind`] names and builds a [`LogFormat`].
#[derive(Clone)]
pub struct LogFormatParser;

/// Result type for [`LogFormatParser::parse_ref`].
type LogFormatParserResult = Result<LogFormat, Error>;

impl Display for LogFormatKind {
    /// Writes the adb `-v` verb (`brief`, `long`, …).
    fn fmt(&self, formatter: &mut Formatter) -> fmtResult {
        let name = match self {
            Self::Brief => "brief",
            Self::Long => "long",
            Self::Process => "process",
            Self::Raw => "raw",
            Self::Tag => "tag",
            Self::Thread => "thread",
            Self::ThreadTime => "threadtime",
            Self::Time => "time",
        };

        write!(formatter, "{name}")
    }
}

impl LogFormat {
    /// Builds a parser for `kind`, compiling the appropriate line regex and capture indices.
    pub fn new(kind: LogFormatKind) -> Self {
        let match_cfg = match kind {
            LogFormatKind::Brief => LogFormatMatchConfig {
                regex: Regex::new(r"^([A-Z])/(.+?)\(\s*(?:(\S+):\s*)?(\d+)\): (.*?)$").unwrap(),
                date_index: None,
                time_index: None,
                level_index: Some(1),
                tag_index: Some(2),
                pid_index: Some(4),
                uid_index: Some(3),
                tid_index: None,
                msg_index: Some(5),
            },
            LogFormatKind::Long => LogFormatMatchConfig {
                regex: Regex::new(
                    r"^\[\s+(\d+-\d+)\s+((?:\d+:?)+(?:\.\d+)?)\s+(?:(\S+):\s+(\d+):\s*(\d+)|(\d+):\s*(\d+))\s+([A-Z])/(.+?)\s+\]\s*(.*)$",
                )
                .unwrap(),
                date_index: Some(1),
                time_index: Some(2),
                level_index: Some(8),
                tag_index: Some(9),
                pid_index: Some(4),
                uid_index: Some(3),
                tid_index: Some(5),
                msg_index: Some(10),
            },
            LogFormatKind::Process => LogFormatMatchConfig {
                regex: Regex::new(
                    r"^([A-Z])\(\s*(?:(\S+):\s*)?(\d+)\)\s+(.*?)(?:\s+\((.+?)\))?\s*$",
                )
                .unwrap(),
                date_index: None,
                time_index: None,
                level_index: Some(1),
                tag_index: Some(5),
                pid_index: Some(3),
                uid_index: Some(2),
                tid_index: None,
                msg_index: Some(4),
            },
            LogFormatKind::Raw => LogFormatMatchConfig {
                regex: Regex::new(r"^(.*)$").unwrap(),
                date_index: None,
                time_index: None,
                level_index: Some(9),
                tag_index: Some(10),
                pid_index: Some(11),
                uid_index: Some(12),
                tid_index: None,
                msg_index: Some(1),
            },
            LogFormatKind::Tag => LogFormatMatchConfig {
                regex: Regex::new(r"^([A-Z])/(.+?): (.*?)$").unwrap(),
                date_index: None,
                time_index: None,
                level_index: Some(1),
                tag_index: Some(2),
                pid_index: Some(9),
                uid_index: Some(10),
                tid_index: None,
                msg_index: Some(3),
            },
            LogFormatKind::Thread => LogFormatMatchConfig {
                regex: Regex::new(r"^([A-Z])\(\s*(?:(\S+):\s*)?(\d+):\s*(\d+)\)\s*(.*?)$").unwrap(),
                date_index: None,
                time_index: None,
                level_index: Some(1),
                tag_index: Some(10),
                pid_index: Some(3),
                uid_index: Some(2),
                tid_index: Some(4),
                msg_index: Some(5),
            },
            LogFormatKind::ThreadTime => LogFormatMatchConfig {
                regex: Regex::new(
                    r"^(\d+-\d+)\s+((?:\d+:?)+(?:\.\d+)?)\s+(?:(\S+)\s+)?(\d+)\s+(\d+)\s+([A-Z])\s+(.*?):\s+(.*?)$"
                ).unwrap(),
                date_index: Some(1),
                time_index: Some(2),
                level_index: Some(6),
                tag_index: Some(7),
                pid_index: Some(4),
                uid_index: Some(3),
                tid_index: Some(5),
                msg_index: Some(8),
            },
            LogFormatKind::Time => LogFormatMatchConfig {
                regex: Regex::new(
                    r"^(\d+-\d+)\s+((?:\d+:?)+(?:\.\d+)?)\s+([A-Z])/(.+?)\(\s*(?:(\S+):\s*)?(\d+)\): (.*?)$",
                )
                .unwrap(),
                date_index: Some(1),
                time_index: Some(2),
                level_index: Some(3),
                tag_index: Some(4),
                pid_index: Some(6),
                uid_index: Some(5),
                tid_index: None,
                msg_index: Some(7),
            },
        };

        Self { kind, match_cfg }
    }

    /// Line-matching regex for this format.
    pub fn regex(&self) -> &Regex {
        &self.match_cfg.regex
    }

    /// Capture index for the date field, if any.
    pub fn date_index(&self) -> &Option<usize> {
        &self.match_cfg.date_index
    }

    /// Capture index for the time field, if any.
    pub fn time_index(&self) -> &Option<usize> {
        &self.match_cfg.time_index
    }

    /// Capture index for the priority letter.
    pub fn level_index(&self) -> &Option<usize> {
        &self.match_cfg.level_index
    }

    /// Capture index for the tag name.
    pub fn tag_index(&self) -> &Option<usize> {
        &self.match_cfg.tag_index
    }

    /// Capture index for the process ID.
    pub fn pid_index(&self) -> &Option<usize> {
        &self.match_cfg.pid_index
    }

    /// Capture index for the user ID.
    pub fn uid_index(&self) -> &Option<usize> {
        &self.match_cfg.uid_index
    }

    /// Capture index for the thread ID.
    pub fn tid_index(&self) -> &Option<usize> {
        &self.match_cfg.tid_index
    }

    /// Capture index for the message body.
    pub fn msg_index(&self) -> &Option<usize> {
        &self.match_cfg.msg_index
    }

    /// Parses `(uid, pid)` from a `-v long` regex match (handles `uid:pid:tid` vs `pid:tid`).
    pub fn long_owner_ids(captures: &regex::Captures) -> (String, String) {
        if captures.get(3).is_some() {
            let uid = captures
                .get(3)
                .map_or(String::default(), |mat| mat.as_str().trim().to_string());
            let pid = captures
                .get(4)
                .map_or(String::default(), |mat| mat.as_str().trim().to_string());
            return (uid, pid);
        }

        let pid = captures
            .get(6)
            .map_or(String::default(), |mat| mat.as_str().trim().to_string());
        (String::default(), pid)
    }

    /// Argument to `adb logcat -v`, including `uid` when the device supports it.
    pub fn adb_verb(&self) -> String {
        match self.kind {
            LogFormatKind::Brief | LogFormatKind::ThreadTime => {
                format!("{kind},uid", kind = self.kind)
            }
            LogFormatKind::Raw => format!("{kind}", kind = self.kind),
            LogFormatKind::Long
            | LogFormatKind::Process
            | LogFormatKind::Tag
            | LogFormatKind::Thread
            | LogFormatKind::Time => format!("{kind},uid", kind = self.kind),
        }
    }
}

impl From<&str> for LogFormatKind {
    /// Parses compact CLI codes (`B`, `L`, `P`, …) into a [`LogFormatKind`].
    ///
    /// # Panics
    ///
    /// Panics if `str` is not a recognized code.
    fn from(str: &str) -> Self {
        match str {
            "B" => Self::Brief,
            "L" => Self::Long,
            "P" => Self::Process,
            "R" => Self::Raw,
            "T" => Self::Tag,
            "Th" => Self::Thread,
            "Tht" => Self::ThreadTime,
            "Ti" => Self::Time,
            _ => panic!("Invalid log format kind"),
        }
    }
}

impl From<&str> for LogFormat {
    /// Builds [`LogFormat`] from a compact kind code (see [`LogFormatKind::from`]).
    fn from(str: &str) -> Self {
        let kind = LogFormatKind::from(str);

        Self::new(kind)
    }
}

impl From<String> for LogFormat {
    /// Builds [`LogFormat`] from an owned kind code string.
    fn from(str: String) -> Self {
        Self::from(str.as_str())
    }
}

impl Display for LogFormat {
    /// Displays the adb verb name ([`LogFormatKind`]).
    fn fmt(&self, formatter: &mut Formatter) -> fmtResult {
        write!(formatter, "{kind}", kind = self.kind)
    }
}

impl ValueEnum for LogFormatKind {
    /// All variants offered on the command line.
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::Brief,
            Self::Long,
            Self::Process,
            Self::Raw,
            Self::Tag,
            Self::Thread,
            Self::ThreadTime,
            Self::Time,
        ]
    }

    /// clap name, aliases, and help for one variant.
    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            Self::Brief => PossibleValue::new("B").alias("brief").help("brief"),
            Self::Long => PossibleValue::new("L").alias("long").help("long"),
            Self::Process => PossibleValue::new("P").alias("process").help("process"),
            Self::Raw => PossibleValue::new("R").alias("raw").help("raw"),
            Self::Tag => PossibleValue::new("T").alias("tag").help("tag"),
            Self::Thread => PossibleValue::new("Th").alias("thread").help("thread"),
            Self::ThreadTime => PossibleValue::new("Tht")
                .alias("threadtime")
                .help("threadtime"),
            Self::Time => PossibleValue::new("Ti").alias("time").help("time"),
        })
    }
}

impl TypedValueParser for LogFormatParser {
    /// Parsed CLI value type.
    type Value = LogFormat;

    /// Parses a [`LogFormatKind`] then wraps it with [`LogFormat::new`].
    fn parse_ref(&self, cmd: &Command, arg: Option<&Arg>, value: &OsStr) -> LogFormatParserResult {
        let enum_value_parser = EnumValueParser::<LogFormatKind>::new();
        let kind = enum_value_parser.parse_ref(cmd, arg, value)?;

        Ok(LogFormat::new(kind))
    }

    /// Enumerates valid `--log-format` strings for shell completions.
    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue>>> {
        let value_variants = LogFormatKind::value_variants()
            .iter()
            .filter_map(|kind| kind.to_possible_value());

        Some(Box::new(value_variants))
    }
}
