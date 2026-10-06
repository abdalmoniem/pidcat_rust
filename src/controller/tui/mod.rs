#![deny(clippy::unwrap_used)]

mod app;
mod border;
mod copy;
mod device_picker;
mod display_cache;
mod file_source;
mod help;
mod log_ingest;
mod palette;
mod theme;
mod ui;

pub use app::run_tui;
