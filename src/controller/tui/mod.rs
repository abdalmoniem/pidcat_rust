#![deny(clippy::unwrap_used)]

mod app;
mod copy;
mod device_picker;
mod display_cache;
mod file_source;
mod help;
mod palette;
mod theme;
mod ui;

pub use app::run_tui;
