// Copyright (C) 2026 AbdAlMoniem AlHifnawy
//
// This file is part of pidcatrs.
//
// pidcatrs is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// pidcatrs is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with pidcatrs.  If not, see <https://www.gnu.org/licenses/>.
//
// Author: AbdAlMoniem AlHifnawy

//! The command palette / help catalog.
//!
//! [`HELP_CATALOG`] is the single source of truth for every documented key
//! binding and filter keyword. Entries that have a [`HelpAction`] can be
//! executed straight from the palette; the rest are purely informational.
//! [`build_help_rows`] turns the catalog into the (optionally searched) list
//! of rows that the UI displays.

#![deny(clippy::unwrap_used)]

/// A command that can be triggered directly from the command palette.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HelpAction {
    /// Quit the application.
    Quit,
    /// Toggle pausing of incoming logs.
    PauseResume,
    /// Clear the buffer and restart live logcat capture.
    RestartLive,
    /// Open the device selection dialog.
    OpenDevicePicker,
    /// Open the "open log file" dialog.
    OpenFileDialog,
    /// Export the currently filtered entries to a file.
    ExportEntries,
    /// Move keyboard focus to the filter input.
    FocusFilter,
}

/// One documented key binding or filter keyword.
#[derive(Clone, Copy, Debug)]
pub struct HelpEntry {
    /// Name of the group this entry is listed under (e.g. `"log view"`).
    pub section: &'static str,
    /// The key(s) or syntax, as displayed (e.g. `"j / k / ↑ / ↓"`).
    pub keys: &'static str,
    /// Short explanation of what the binding does.
    pub description: &'static str,
    /// The command to run when the entry is chosen in the palette, or `None`
    /// for entries that are informational only.
    pub action: Option<HelpAction>,
}

/// Every entry shown in the command palette, grouped by consecutive `section`.
///
/// Entries of the same section must be adjacent: [`build_help_rows`] emits a
/// section header whenever the section name changes.
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
        keys: "← / →",
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
        keys: "j / k / ↑ / ↓",
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
        keys: "j / k / ↑ / ↓",
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
        keys: "j / k / ↑ / ↓",
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
        keys: "↑ / ↓ / pgup / pgdn",
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

/// One displayed row of the command palette list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HelpRow {
    /// A non-selectable section heading with the section's name.
    Section(&'static str),
    /// A selectable entry, identified by its index into [`HELP_CATALOG`].
    Entry(usize),
}

/// Builds the list of rows to display for a search query.
///
/// With an empty (or whitespace-only) query, all entries are listed, each
/// section introduced by a [`HelpRow::Section`] heading. Otherwise only the
/// entries whose `"<section> <keys> <description>"` text matches the query
/// (see [`matches_query`](super::palette::matches_query)) are returned, as a
/// flat list without headings.
///
/// # Arguments
///
/// * `query` - The text typed into the palette search field.
///
/// # Returns
///
/// The rows in display order.
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

/// Returns the command bound to a row, if any.
///
/// # Arguments
///
/// * `row` - A row produced by [`build_help_rows`].
///
/// # Returns
///
/// The entry's [`HelpAction`], or `None` for section headings and
/// informational entries.
///
/// # Panics
///
/// Panics if `row` is a [`HelpRow::Entry`] whose index is outside
/// [`HELP_CATALOG`]; rows from [`build_help_rows`] are always in range.
pub fn row_action(row: &HelpRow) -> Option<HelpAction> {
    match row {
        HelpRow::Entry(index) => HELP_CATALOG[*index].action,
        HelpRow::Section(_) => None,
    }
}
