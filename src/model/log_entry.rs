use crate::LogLevel;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LogEntryKind {
    Normal,
    ProcessStart,
    ProcessDeath,
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub kind: LogEntryKind,
    pub pid: String,
    pub uid: String,
    pub owner: String,
    pub package: String,
    pub tag: String,
    pub level: LogLevel,
    pub message: String,
    pub banner_text: String,
}
