mod controller;
mod model;

pub use controller::adb::build_adb_command;
pub use controller::adb::build_logcat_command;
pub use controller::adb::clear_logcat;
pub use controller::adb::get_adb_command;
pub use controller::adb::get_adb_devices;
pub use controller::adb::get_current_app_package;
pub use controller::adb::get_processes;
pub use controller::adb::resolve_initial_device;
pub use controller::adb::spawn_logcat;
pub use controller::adb::start_adb_server;

pub use controller::util::colored;
pub use controller::util::open_output_writer;
pub use controller::util::split_csv_values;
pub use controller::util::trim_log_line;
pub use controller::util::trim_log_line_bytes;

pub use controller::log_processor::ELLIPSIS;
pub use controller::log_processor::PLAIN_LEVEL_DISPLAY_WIDTH;
pub use controller::log_processor::PLAIN_LEVEL_HEADER_WIDTH;
pub use controller::log_processor::PLAIN_MESSAGE_SEPARATOR_WIDTH;
pub use controller::log_processor::SYSTEM_TAGS;
pub use controller::log_processor::TUI_MESSAGE_WRAP_HEADER_WIDTH;
pub use controller::log_processor::compute_header_width;
pub use controller::log_processor::format_log_level_field;
pub use controller::log_processor::format_log_message;
pub use controller::log_processor::format_process_death_message;
pub use controller::log_processor::format_process_start_messages;
pub use controller::log_processor::get_active_codes_at_pos;
pub use controller::log_processor::get_ansi_segments;
pub use controller::log_processor::get_dead_process;
pub use controller::log_processor::get_started_process;
pub use controller::log_processor::get_token_color;
pub use controller::log_processor::get_wrapped_indent;
pub use controller::log_processor::insert_ansi_codes_in_range;
pub use controller::log_processor::is_matching_package;
pub use controller::log_processor::is_matching_tag;
pub use controller::log_processor::level_color;
pub use controller::log_processor::plain_banner_prefix_width;
pub use controller::log_processor::plain_prefix_column_widths;
pub use controller::log_processor::process_line;
pub use controller::log_processor::render_entry;
pub use controller::log_processor::render_entry_lines;
pub use controller::log_processor::wrap_message_for_tui;
pub use controller::log_processor::wrap_text_for_tui;
pub use controller::log_processor::write_log_line;

pub use controller::plain::is_running;
pub use controller::plain::run_plain;
pub use controller::plain::set_running;

pub use controller::tui::run_tui;

pub use model::adb_device::AdbDevice;
pub use model::adb_state::AdbState;
pub use model::ansi_segment::AnsiSegment;
pub use model::cli_args::CliArgs;
pub use model::filter::is_ignored_tag;
pub use model::filter::package_name_contains;
pub use model::filter::passes_log_level;
pub use model::filter::passes_package_ownership;
pub use model::filter::passes_tag_filter;
pub use model::filter::resolve_entry_package;
pub use model::log_entry::LogEntry;
pub use model::log_entry::LogEntryKind;
pub use model::log_format::LogFormat;
pub use model::log_format::LogFormatKind;
pub use model::log_format::LogFormatParser;
pub use model::log_level::LogLevel;
pub use model::log_source::LogSource;
pub use model::state::State;
pub use model::tui_filter::TuiFilter;
pub use model::tui_filter::TuiFilterSet;
pub use model::value_unwrap::ValueOrPanic;

pub use controller::writer::Writer;
