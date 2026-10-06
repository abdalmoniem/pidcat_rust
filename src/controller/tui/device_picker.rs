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

const DEVICE_SEARCH_PLACEHOLDER: &str = "search devices by serial or state...";

const DEVICE_PICKER_HINTS: &[(&str, &str)] = &[
    ("↑↓", " navigate"),
    ("enter", " select"),
    ("esc", " cancel"),
    ("ctrl+r", " refresh"),
    ("o", " open file"),
];

pub fn is_selectable(device: &AdbDevice) -> bool {
    matches!(device.device_state, AdbState::Device | AdbState::Emulator)
}

pub fn device_state_label(devices: &[AdbDevice], serial: &str) -> Option<&'static str> {
    devices
        .iter()
        .find(|device| device.device_id == serial)
        .map(|device| device.device_state.label())
}

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
                        .fg(theme::BG)
                        .bg(theme::YELLOW)
                        .add_modifier(Modifier::BOLD)
                } else if selectable {
                    Style::default().fg(theme::TEXT)
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
