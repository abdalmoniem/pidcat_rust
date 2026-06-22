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

/*
LOG_LINE native adb formats:
Single format verbs:

brief      Show priority, tag, and PID of the process issuing the message.
long       Show all metadata fields and separate messages with blank lines.
process    Show PID only.
raw        Show the raw log message with no other metadata fields.
tag        Show the priority and tag only.
thread     Show priority, PID, and TID of the thread issuing the message.
threadtime Show the date, invocation time, priority, tag, PID, and TID of the thread issuing the message. (This is the default.)
time       Show the date, invocation time, priority, tag, and PID of the process issuing the message.
*/

#[derive(Clone, Debug)]
pub struct LogFormatMatchConfig {
    pub regex: Regex,
    pub date_index: Option<usize>,
    pub time_index: Option<usize>,
    pub level_index: Option<usize>,
    pub tag_index: Option<usize>,
    pub pid_index: Option<usize>,
    pub tid_index: Option<usize>,
    pub msg_index: Option<usize>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LogFormatKind {
    Brief = 0isize,
    Long = 1isize,
    Process = 2isize,
    Raw = 3isize,
    Tag = 4isize,
    Thread = 5isize,
    ThreadTime = 6isize,
    Time = 7isize,
}

#[derive(Clone, Debug)]
pub struct LogFormat {
    pub kind: LogFormatKind,
    match_cfg: LogFormatMatchConfig,
}

#[derive(Clone)]
pub struct LogFormatParser;

type LogFormatParserResult = Result<LogFormat, Error>;

impl Display for LogFormatKind {
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
    pub fn new(kind: LogFormatKind) -> Self {
        let match_cfg = match kind {
            LogFormatKind::Brief => LogFormatMatchConfig {
                regex: Regex::new(r"^([A-Z])/(.+?)\( *(\d+)\): (.*?)$").unwrap(),
                date_index: None,
                time_index: None,
                level_index: Some(1),
                tag_index: Some(2),
                pid_index: Some(3),
                tid_index: None,
                msg_index: Some(4),
            },
            LogFormatKind::Long => unimplemented!("Long LogFormatKind not implemented yet!"),
            LogFormatKind::Process => unimplemented!("Process LogFormatKind not implemented yet!"),
            LogFormatKind::Raw => unimplemented!("Raw LogFormatKind not implemented yet!"),
            LogFormatKind::Tag => unimplemented!("Tag LogFormatKind not implemented yet!"),
            LogFormatKind::Thread => unimplemented!("Thread LogFormatKind not implemented yet!"),
            LogFormatKind::ThreadTime => LogFormatMatchConfig {
                regex: Regex::new(
                    r"^(\d+-\d+)\s+((?:\d+:?)+(?:\.\d+)?)\s+(\d+)\s+(\d+)\s+([A-Z])\s+(.*?):\s+(.*?)$"
                ).unwrap(),
                date_index: Some(1),
                time_index: Some(2),
                level_index: Some(5),
                tag_index: Some(6),
                pid_index: Some(3),
                tid_index: Some(4),
                msg_index: Some(7),
            },
            LogFormatKind::Time => unimplemented!("Time LogFormatKind not implemented yet!"),
        };

        Self { kind, match_cfg }
    }

    pub fn regex(&self) -> &Regex {
        &self.match_cfg.regex
    }

    pub fn date_index(&self) -> &Option<usize> {
        &self.match_cfg.date_index
    }

    pub fn time_index(&self) -> &Option<usize> {
        &self.match_cfg.time_index
    }

    pub fn level_index(&self) -> &Option<usize> {
        &self.match_cfg.level_index
    }

    pub fn tag_index(&self) -> &Option<usize> {
        &self.match_cfg.tag_index
    }

    pub fn pid_index(&self) -> &Option<usize> {
        &self.match_cfg.pid_index
    }

    pub fn tid_index(&self) -> &Option<usize> {
        &self.match_cfg.tid_index
    }

    pub fn msg_index(&self) -> &Option<usize> {
        &self.match_cfg.msg_index
    }
}

impl From<&str> for LogFormatKind {
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
    fn from(str: &str) -> Self {
        let kind = LogFormatKind::from(str);

        Self::new(kind)
    }
}

impl From<String> for LogFormat {
    fn from(str: String) -> Self {
        Self::from(str.as_str())
    }
}

impl Display for LogFormat {
    fn fmt(&self, formatter: &mut Formatter) -> fmtResult {
        write!(formatter, "{kind}", kind = self.kind)
    }
}

impl ValueEnum for LogFormatKind {
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
    type Value = LogFormat;

    fn parse_ref(&self, cmd: &Command, arg: Option<&Arg>, value: &OsStr) -> LogFormatParserResult {
        let enum_value_parser = EnumValueParser::<LogFormatKind>::new();
        let kind = enum_value_parser.parse_ref(cmd, arg, value)?;

        Ok(LogFormat::new(kind))
    }

    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue>>> {
        let value_variants = LogFormatKind::value_variants()
            .iter()
            .filter_map(|kind| kind.to_possible_value());

        Some(Box::new(value_variants))
    }
}
