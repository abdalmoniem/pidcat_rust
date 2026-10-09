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

//! The "select device" dialog.
//!
//! Lists the devices reported by `adb devices`, lets the user narrow them
//! down by typing, and renders the dialog. Devices that are offline or
//! unauthorized are shown dimmed and cannot be selected.

#![deny(clippy::unwrap_used)]

use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::Paragraph;

use crate::AdbDevice;
use crate::AdbState;
use crate::controller::adb::NO_ADB_DEVICES_ERROR_HEADER;
use crate::controller::adb::NO_ADB_DEVICES_ERROR_MESSAGE;

use super::border::render_dialog;
use super::palette::PaletteSearch;
use super::palette::matches_query;
use super::palette::render_search_field;
use super::theme;

/// Placeholder shown in the empty device search field.
const DEVICE_SEARCH_PLACEHOLDER: &str = "search devices by serial or state...";

/// Shortcut hints shown in the bottom border of the device dialog.
const DEVICE_PICKER_HINTS: &[(&str, &str)] = &[
    ("↑↓", " navigate"),
    ("enter", " select"),
    ("esc", " cancel"),
    ("ctrl+r", " refresh"),
    ("o", " open file"),
];

/// Tests whether a device can be chosen as the log source.
///
/// # Arguments
///
/// * `device` - The device to check.
///
/// # Returns
///
/// `true` for ready physical devices and emulators; `false` for devices that
/// are offline, unauthorized, etc.
pub fn is_selectable(device: &AdbDevice) -> bool {
    matches!(device.device_state, AdbState::Device | AdbState::Emulator)
}

/// Looks up the human-readable state of a device by serial number.
///
/// # Arguments
///
/// * `devices` - The known devices.
/// * `serial` - The serial number to look for.
///
/// # Returns
///
/// The state label (e.g. `"device"`), or `None` if no device has that serial.
pub fn device_state_label(devices: &[AdbDevice], serial: &str) -> Option<&'static str> {
    devices
        .iter()
        .find(|device| device.device_id == serial)
        .map(|device| device.device_state.label())
}

/// Filters devices by a search query.
///
/// The query is matched against `"<serial> <state label>"` using
/// [`matches_query`], so every whitespace-separated token must match.
///
/// # Arguments
///
/// * `devices` - The devices to filter.
/// * `query` - The user's search text.
///
/// # Returns
///
/// Indices into `devices` of the matching devices, in their original order.
pub fn filter_device_indices(devices: &[AdbDevice], query: &str) -> Vec<usize> {
    devices
        .iter()
        .enumerate()
        .filter(|(_, device)| {
            let haystack = format!("{} {}", device.device_id, device.device_state.label());
            matches_query(query, &haystack)
        })
        .map(|(index, _)| index)
        .collect()
}

/// Renders the device picker dialog.
///
/// Shows a search field above the device list. Depending on the state it
/// displays an explanatory "no devices" message, a "no matching devices"
/// notice, or the filtered list with the highlighted row. The selection and
/// scroll offset in `search` are updated to keep the highlight visible.
///
/// # Arguments
///
/// * `frame` - Frame to draw on.
/// * `devices` - All known devices.
/// * `search` - Search/selection state (mutated to clamp and scroll).
/// * `area` - Outer rectangle of the dialog.
pub fn render_device_picker(
    frame: &mut Frame,
    devices: &[AdbDevice],
    search: &mut PaletteSearch,
    area: Rect,
) {
    let inner = render_dialog(frame, area, "select device", DEVICE_PICKER_HINTS, false);

    let chunks = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(inner);

    render_search_field(frame, chunks[0usize], search, DEVICE_SEARCH_PLACEHOLDER);

    let filtered = filter_device_indices(devices, &search.query);
    let list_height = chunks[1usize].height as usize;

    if devices.is_empty() {
        let empty = List::new(vec![
            ListItem::new(Span::styled(
                NO_ADB_DEVICES_ERROR_HEADER,
                theme::error_style(),
            )),
            ListItem::new(Span::styled(
                NO_ADB_DEVICES_ERROR_MESSAGE,
                theme::error_style(),
            )),
            ListItem::new(""),
            ListItem::new(Line::from(vec![
                Span::styled("connect a device, then press ", theme::hint_style()),
                Span::styled("ctrl+r", theme::hint_key_style()),
                Span::styled(" to refresh", theme::hint_style()),
            ])),
        ])
        .style(theme::app_background_style());
        frame.render_widget(empty, chunks[1usize]);
    } else if filtered.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "no matching devices",
                theme::dim_style(),
            )))
            .style(theme::app_background_style()),
            chunks[1usize],
        );
    } else {
        search.clamp_selection(filtered.len());
        search.ensure_list_top_visible(list_height);

        let items = filtered
            .iter()
            .enumerate()
            .skip(search.list_top)
            .take(list_height)
            .map(|(visible_index, device_index)| {
                let device = &devices[*device_index];
                let list_index = search.list_top + visible_index;
                let selectable = is_selectable(device);
                let prefix = if list_index == search.selected {
                    "▸ "
                } else {
                    "  "
                };
                let state_label = device.device_state.label();
                let line = format!("{prefix}{}  {state_label}", device.device_id);
                let style = if list_index == search.selected {
                    Style::default()
                        .fg(theme::background())
                        .bg(theme::accent())
                        .add_modifier(Modifier::BOLD)
                } else if selectable {
                    Style::default().fg(theme::text())
                } else {
                    theme::dim_style()
                };
                ListItem::new(Line::from(Span::styled(line, style)))
            })
            .collect::<Vec<_>>();

        let list = List::new(items).style(theme::app_background_style());
        frame.render_widget(list, chunks[1usize]);
    }
}
