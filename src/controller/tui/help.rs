#![deny(clippy::unwrap_used)]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HelpAction {
    Quit,
    PauseResume,
    RestartLive,
    OpenDevicePicker,
    OpenFileDialog,
    ExportEntries,
    FocusFilter,
}

#[derive(Clone, Copy, Debug)]
pub struct HelpEntry {
    pub section: &'static str,
    pub keys: &'static str,
    pub description: &'static str,
    pub action: Option<HelpAction>,
}

pub const HELP_CATALOG: &[HelpEntry] = &[
    HelpEntry {
        section: "general",
        keys: "q",
        description: "quit",
        action: Some(HelpAction::Quit),
    },
    HelpEntry {
        section: "general",
        keys: "ctrl+c",
        description: "quit",
        action: Some(HelpAction::Quit),
    },
    HelpEntry {
        section: "general",
        keys: "?",
        description: "open command palette",
        action: None,
    },
    HelpEntry {
        section: "general",
        keys: "esc",
        description: "close dialog / stop editing filter",
        action: None,
    },
    HelpEntry {
        section: "log view",
        keys: "j / k / ↑ / ↓",
        description: "scroll log view",
        action: None,
    },
    HelpEntry {
        section: "log view",
        keys: "pgup / pgdn",
        description: "scroll log view by page",
        action: None,
    },
    HelpEntry {
        section: "log view",
        keys: "v",
        description: "enter select mode",
        action: None,
    },
    HelpEntry {
        section: "log view",
        keys: "mouse wheel",
        description: "scroll log view",
        action: None,
    },
    HelpEntry {
        section: "select mode",
        keys: "j / k / ↑ / ↓",
        description: "move selection between log entries",
        action: None,
    },
    HelpEntry {
        section: "select mode",
        keys: "pgup / pgdn",
        description: "move selection by 10 entries",
        action: None,
    },
    HelpEntry {
        section: "select mode",
        keys: "y / enter",
        description: "open copy menu for selected entry",
        action: None,
    },
    HelpEntry {
        section: "select mode",
        keys: "v / esc",
        description: "exit select mode",
        action: None,
    },
    HelpEntry {
        section: "log view",
        keys: "g / home",
        description: "jump to top",
        action: None,
    },
    HelpEntry {
        section: "log view",
        keys: "G / end",
        description: "jump to bottom (resume live tail)",
        action: None,
    },
    HelpEntry {
        section: "log capture",
        keys: "p / space",
        description: "pause / resume incoming logs",
        action: Some(HelpAction::PauseResume),
    },
    HelpEntry {
        section: "log capture",
        keys: "l",
        description: "clear buffer and restart live logcat",
        action: Some(HelpAction::RestartLive),
    },
    HelpEntry {
        section: "log capture",
        keys: "d",
        description: "select adb device",
        action: Some(HelpAction::OpenDevicePicker),
    },
    HelpEntry {
        section: "log capture",
        keys: "o",
        description: "open log file",
        action: Some(HelpAction::OpenFileDialog),
    },
    HelpEntry {
        section: "log capture",
        keys: "ctrl+s",
        description: "export entries matching the current filter to a file",
        action: Some(HelpAction::ExportEntries),
    },
    HelpEntry {
        section: "filter bar",
        keys: "/",
        description: "focus filter input",
        action: Some(HelpAction::FocusFilter),
    },
    HelpEntry {
        section: "filter bar",
        keys: "enter",
        description: "apply filter",
        action: None,
    },
    HelpEntry {
        section: "filter bar",
        keys: "left / right",
        description: "move cursor in filter text",
        action: None,
    },
    HelpEntry {
        section: "filter bar",
        keys: "home / end",
        description: "jump to start/end of filter text",
        action: None,
    },
    HelpEntry {
        section: "filter bar",
        keys: "backspace / delete",
        description: "edit filter text",
        action: None,
    },
    HelpEntry {
        section: "device picker",
        keys: "enter",
        description: "select highlighted device",
        action: None,
    },
    HelpEntry {
        section: "device picker",
        keys: "ctrl+r",
        description: "refresh device list",
        action: None,
    },
    HelpEntry {
        section: "device picker",
        keys: "j / k / up / down",
        description: "move selection",
        action: None,
    },
    HelpEntry {
        section: "copy menu",
        keys: "m / t / p / u / e",
        description: "copy message, tag, pid, uid, or entire entry",
        action: None,
    },
    HelpEntry {
        section: "copy menu",
        keys: "enter",
        description: "copy highlighted option to clipboard",
        action: None,
    },
    HelpEntry {
        section: "copy menu",
        keys: "j / k / up / down",
        description: "move selection",
        action: None,
    },
    HelpEntry {
        section: "export format",
        keys: "p / a",
        description: "export as rendered output or raw adb logcat lines",
        action: None,
    },
    HelpEntry {
        section: "export format",
        keys: "enter",
        description: "choose highlighted format",
        action: None,
    },
    HelpEntry {
        section: "export format",
        keys: "j / k / up / down",
        description: "move selection",
        action: None,
    },
    HelpEntry {
        section: "file explorer",
        keys: "type a path",
        description: "list matching entries of the typed directory",
        action: None,
    },
    HelpEntry {
        section: "file explorer",
        keys: "up / down / pgup / pgdn",
        description: "move selection",
        action: None,
    },
    HelpEntry {
        section: "file explorer",
        keys: "tab",
        description: "complete path with selected entry",
        action: None,
    },
    HelpEntry {
        section: "file explorer",
        keys: "enter",
        description: "open file / save export (enter twice to overwrite)",
        action: None,
    },
    HelpEntry {
        section: "filter syntax",
        keys: "package:name",
        description: "filter by package (or with multiple)",
        action: None,
    },
    HelpEntry {
        section: "filter syntax",
        keys: "tag:name",
        description: "filter by tag (or with multiple)",
        action: None,
    },
    HelpEntry {
        section: "filter syntax",
        keys: "level:debug",
        description: "minimum log level (verbose/debug/info/...)",
        action: None,
    },
    HelpEntry {
        section: "filter syntax",
        keys: "pid:1234",
        description: "filter by pid",
        action: None,
    },
    HelpEntry {
        section: "filter syntax",
        keys: "uid:10001",
        description: "filter by uid",
        action: None,
    },
    HelpEntry {
        section: "filter syntax",
        keys: "word",
        description: "match message text (and with other keywords)",
        action: None,
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HelpRow {
    Section(&'static str),
    Entry(usize),
}

pub fn build_help_rows(query: &str) -> Vec<HelpRow> {
    let query = query.trim();
    if query.is_empty() {
        let mut rows = Vec::default();
        let mut current_section = "";

        for (index, entry) in HELP_CATALOG.iter().enumerate() {
            if entry.section != current_section {
                current_section = entry.section;
                rows.push(HelpRow::Section(current_section));
            }
            rows.push(HelpRow::Entry(index));
        }

        return rows;
    }

    HELP_CATALOG
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            let haystack = format!("{} {} {}", entry.section, entry.keys, entry.description);
            super::palette::matches_query(query, &haystack)
        })
        .map(|(index, _)| HelpRow::Entry(index))
        .collect()
}

pub fn row_action(row: &HelpRow) -> Option<HelpAction> {
    match row {
        HelpRow::Entry(index) => HELP_CATALOG[*index].action,
        HelpRow::Section(_) => None,
    }
}
