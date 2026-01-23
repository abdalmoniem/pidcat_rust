use std::collections::HashMap;

use crate::LogLevel;

#[derive(Debug)]
pub struct State {
    pub pids_map: HashMap<String, String>,
    pub last_tag: Option<String>,
    pub app_pid: Option<String>,
    pub log_level: LogLevel,
    pub named_processes: Vec<String>,
    pub catchall_package: Vec<String>,
    pub token_colors: Vec<colored::Color>,
    pub known_tokens: HashMap<String, colored::Color>,
}
