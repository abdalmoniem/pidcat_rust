#![deny(clippy::unwrap_used)]

use std::fs::File;
use std::path::Path;
use std::path::PathBuf;

pub fn default_browse_directory() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home);
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Working directory the binary was launched from.
pub fn default_export_directory() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| default_browse_directory())
}

pub fn expand_path(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return format!("{}/{stripped}", home.to_string_lossy());
    }

    if path == "~"
        && let Some(home) = std::env::var_os("HOME")
    {
        return home.to_string_lossy().to_string();
    }

    path.to_string()
}

fn is_path_separator(ch: char) -> bool {
    ch == '/' || ch == std::path::MAIN_SEPARATOR
}

/// Directory path rendered with a trailing separator so typing continues inside it.
pub fn directory_input(path: &Path) -> String {
    let mut input = path.to_string_lossy().to_string();
    if !input.ends_with(is_path_separator) {
        input.push(std::path::MAIN_SEPARATOR);
    }
    input
}

/// `/home/ab` → (`/home/`, `ab`); input without a separator has no directory part.
pub fn split_path_input(input: &str) -> (Option<PathBuf>, String) {
    let expanded = expand_path(input);
    match expanded.rfind(is_path_separator) {
        Some(index) => (
            Some(PathBuf::from(&expanded[..=index])),
            expanded[index + 1..].to_string(),
        ),
        None => (None, expanded),
    }
}

pub fn validate_log_file(path: &str) -> Result<String, String> {
    let expanded = expand_path(path.trim());

    if expanded.is_empty() {
        return Err("path is empty".to_string());
    }

    let file_path = Path::new(&expanded);

    if !file_path.exists() {
        return Err(format!("file not found: {expanded}"));
    }

    if !file_path.is_file() {
        return Err(format!("not a file: {expanded}"));
    }

    File::open(file_path).map_err(|err| format!("cannot read file: {err}"))?;

    Ok(expanded)
}
