// Copyright (c) AbdAlMoniem AlHifnawy <hifnawy_moniem@hotmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Command-line interface definition for pidcatrs ([`CliArgs`]).
//!
//! Built with clap: flags mirror [`crate::Config`] fields, merge with an optional config file,
//! and drive device selection, filtering, column layout, colors, and output mode.

use clap::ArgAction;
use clap::ColorChoice;
use clap::CommandFactory;
use clap::FromArgMatches;
use clap::Parser;
use clap::ValueHint;

use clap::builder::styling::AnsiColor;
use clap::builder::styling::Styles;

use clap_complete::Shell;
use colored::Colorize;

use crate::BUNDLED_THEMES;
use crate::Config;
use crate::DEFAULT_THEME_NAME;
use crate::LogFormat;
use crate::LogFormatKind;
use crate::LogFormatParser;
use crate::LogLevel;
use crate::ValueOrPanic;
use crate::default_config_file;
use crate::exit_with_error;
use crate::model::timestamp::DEFAULT_TIMESTAMP_FORMAT;
use crate::model::timestamp::timestamp_column_width;
use crate::themes_dir;

/// clap help heading for positional arguments.
const POSITIONAL_ARGUMENTS: &str = "Positional Arguments";
/// clap help heading for general options (help, version, completions).
const ABOUT_OPTIONS: &str = "Options";
/// clap help heading for ADB device selection flags.
const DEVICE_OPTIONS: &str = "Device Options";
/// clap help heading for log filtering flags.
const FILTERING_OPTIONS: &str = "Filtering Options";
/// clap help heading for output column and timestamp flags.
const FORMATTING_OPTIONS: &str = "Formatting Options";
/// clap help heading for ANSI and GC coloring flags.
const COLORING_OPTIONS: &str = "Color Options";
/// clap help heading for file output and plain mode.
const OUTPUT_OPTIONS: &str = "Output Options";
/// clap help heading for config file, theme, and introspection flags.
const CONFIG_OPTIONS: &str = "Config Options";

/// Full set of pidcatrs command-line flags and positional package names.
#[derive(Clone, Debug, Parser)]
#[command(disable_help_flag = true)]
#[command(color = ColorChoice::Auto)]
#[command(name = CliArgs::get_name())]
#[command(disable_version_flag = true)]
#[command(about = CliArgs::get_about())]
#[command(arg_required_else_help = false)]
#[command(version = CliArgs::get_version())]
#[command(styles = CliArgs::get_cli_styles())]
#[command(long_version = CliArgs::get_long_version())]
pub struct CliArgs {
    /// Application package name(s); may be repeated on the command line.
    #[arg(required = false)]
    #[arg(value_name = "PACKAGE")]
    #[arg(help_heading = POSITIONAL_ARGUMENTS)]
    #[arg(help = "Application package name(s)\nThis can be specified multiple times")]
    pub packages: Vec<String>,

    /// Short help (`-h` / `--help`).
    #[arg(short = 'h')]
    #[arg(long = "help")]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(action = ArgAction::HelpShort)]
    #[arg(help_heading = ABOUT_OPTIONS)]
    #[arg(help = "Show this help message and exit")]
    pub help: Option<bool>,

    /// Version (`-v` / `--version`).
    #[arg(short = 'v')]
    #[arg(long = "version")]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(action = ArgAction::Version)]
    #[arg(help_heading = ABOUT_OPTIONS)]
    #[arg(help = "Print the version number and exit")]
    pub version: Option<bool>,

    /// When set, generate shell completions for the given shell and exit.
    #[arg(required = false)]
    #[arg(long = "completions")]
    #[arg(value_name = "SHELL")]
    #[arg(help_heading = ABOUT_OPTIONS)]
    #[arg(help = format!("Generate shell completions for {metavar}", metavar = "[SHELL]".cyan().bold()))]
    pub completions: Option<Shell>,

    /// Path to the `adb` executable when it is not on `PATH`.
    #[arg(short = 'A')]
    #[arg(long = "adb")]
    #[arg(required = false)]
    #[arg(default_value = None)]
    #[arg(value_name = "ADB_PATH")]
    #[arg(value_hint = ValueHint::FilePath)]
    #[arg(help_heading = ABOUT_OPTIONS)]
    #[arg(help = "Path to adb executable (if not in PATH)")]
    pub adb_path: Option<String>,

    /// Use the first connected physical device.
    #[arg(short = 'd')]
    #[arg(long = "device")]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(default_value_t = false)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = DEVICE_OPTIONS)]
    #[arg(help = "Use first device for log input")]
    pub use_device: bool,

    /// Use the first connected emulator.
    #[arg(short = 'e')]
    #[arg(required = false)]
    #[arg(long = "emulator")]
    #[arg(value_name = None)]
    #[arg(default_value_t = false)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = DEVICE_OPTIONS)]
    #[arg(help = "Use first emulator for log input")]
    pub use_emulator: bool,

    /// Use the device with this serial number.
    #[arg(short = 's')]
    #[arg(long = "serial")]
    #[arg(required = false)]
    #[arg(default_value = None)]
    #[arg(value_name = "DEVICE_SERIAL")]
    #[arg(help_heading = DEVICE_OPTIONS)]
    #[arg(help = format!("Use {metavar} for log input", metavar = "[DEVICE_SERIAL]".cyan().bold()))]
    pub device_serial: Option<String>,

    /// Do not filter by package; show logs from all applications.
    #[arg(short = 'a')]
    #[arg(long = "all")]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(default_value_t = false)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = FILTERING_OPTIONS)]
    #[arg(help = "Print log messages from all packages")]
    pub all: bool,

    /// Do not clear logcat before streaming (keep existing buffer).
    #[arg(short = 'k')]
    #[arg(long = "keep")]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(default_value_t = false)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = FILTERING_OPTIONS)]
    #[arg(help = "Keep the entire log before running")]
    pub keep_logcat: bool,

    /// Restrict output to the foreground application on the device.
    #[arg(short = 'c')]
    #[arg(long = "current")]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(default_value_t = false)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = FILTERING_OPTIONS)]
    #[arg(help = "Filter logcat by current running app(s)")]
    pub current_app: bool,

    /// Drop lines tagged with known noisy system tags.
    #[arg(short = 'I')]
    #[arg(long = "ignore-system-tags")]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(default_value_t = false)]
    #[arg(help_heading = FILTERING_OPTIONS)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help = concat!(
            "Filter output by ignoring known system tags",
            "\nUse --ignore-tag to ignore additional tags if needed"
        ),
    )]
    pub ignore_system_tags: bool,

    /// Include only lines whose tag matches one of these values (comma-separated or repeated).
    #[arg(short = 't')]
    #[arg(long = "tag")]
    #[arg(required = false)]
    #[arg(value_name = "TAG")]
    #[arg(default_value = None)]
    #[arg(help_heading = FILTERING_OPTIONS)]
    #[arg(help = concat!(
            "Filter output by specified tag(s)",
            "\nThis can be specified multiple times, or as a comma separated list"
        ),
    )]
    pub tag: Option<Vec<String>>,

    /// Exclude lines whose tag matches one of these values.
    #[arg(short = 'i')]
    #[arg(required = false)]
    #[arg(long = "ignore-tag")]
    #[arg(default_value = None)]
    #[arg(value_name = "IGNORED_TAG")]
    #[arg(help_heading = FILTERING_OPTIONS)]
    #[arg(help = concat!(
            "Filter output by ignoring specified tag(s)",
            "\nThis can be specified multiple times, or as a comma separated list"
        ),
    )]
    pub ignore_tag: Option<Vec<String>>,

    /// Minimum log priority to display (verbose is lowest).
    #[arg(short = 'l')]
    #[arg(long = "log-level")]
    #[arg(ignore_case = true)]
    #[arg(value_name = "LEVEL")]
    #[arg(help_heading = FILTERING_OPTIONS)]
    #[arg(default_value_t = LogLevel::VERBOSE)]
    #[arg(help = "Filter messages lower than minimum log level")]
    pub log_level: LogLevel,

    /// Keep only lines whose message matches this regular expression.
    #[arg(short = 'r')]
    #[arg(long = "regex")]
    #[arg(required = false)]
    #[arg(value_name = "REGEX")]
    #[arg(default_value = None)]
    #[arg(help_heading = FILTERING_OPTIONS)]
    #[arg(help = format!("Filter output messages using the specified {metavar}", metavar = "[REGEX]".cyan().bold()))]
    pub regex: Option<String>,

    /// Prepend a timestamp column to each rendered line.
    #[arg(short = 'T')]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(long = "timestamps")]
    #[arg(default_value_t = false)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = FORMATTING_OPTIONS)]
    #[arg(help = "Show a timestamp column before log fields")]
    pub show_timestamps: bool,

    /// `chrono` strftime pattern for [`Self::show_timestamps`] (see docs.rs/chrono strftime).
    #[arg(short = 'Z')]
    #[arg(required = false)]
    #[arg(long = "timestamp-format")]
    #[arg(value_name = "FORMAT")]
    #[arg(default_value = DEFAULT_TIMESTAMP_FORMAT)]
    #[arg(help_heading = FORMATTING_OPTIONS)]
    #[arg(help = "chrono strftime format for the timestamp column (see docs.rs/chrono strftime)")]
    pub timestamp_format: String,

    /// Native `adb logcat -v` format used to parse incoming lines.
    #[arg(short = 'f')]
    #[arg(long = "log-format")]
    #[arg(ignore_case = true)]
    #[arg(value_name = "FORMAT")]
    #[arg(help_heading = FORMATTING_OPTIONS)]
    #[arg(default_value_t = LogFormat::new(LogFormatKind::Brief))]
    #[arg(help = "Input log format from adb")]
    #[arg(value_parser = LogFormatParser)]
    pub log_format: LogFormat,

    /// Show process ID in the output columns.
    #[arg(short = 'P')]
    #[arg(required = false)]
    #[arg(long = "show-pid")]
    #[arg(value_name = None)]
    #[arg(default_value_t = false)]
    #[arg(help = "Show PID in output")]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = FORMATTING_OPTIONS)]
    pub show_pid: bool,

    /// Show user ID in the output columns.
    #[arg(short = 'U')]
    #[arg(required = false)]
    #[arg(long = "show-uid")]
    #[arg(value_name = None)]
    #[arg(default_value_t = false)]
    #[arg(help = "Show UID in output")]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = FORMATTING_OPTIONS)]
    pub show_uid: bool,

    /// Show owning package name in the output columns.
    #[arg(short = 'p')]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(long = "show-package")]
    #[arg(default_value_t = false)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = FORMATTING_OPTIONS)]
    #[arg(help = "Show package name in output")]
    pub show_package: bool,

    /// Print the log tag even when it would normally be omitted for brevity.
    #[arg(short = 'S')]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(default_value_t = false)]
    #[arg(long = "always-show-tags")]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help = "Always show the tag name")]
    #[arg(help_heading = FORMATTING_OPTIONS)]
    pub always_show_tags: bool,

    /// Fixed width of the PID/UID column in plain and TUI layout.
    #[arg(short = 'x')]
    #[arg(required = false)]
    #[arg(long = "puid-width")]
    #[arg(default_value_t = 5u8)]
    #[arg(value_name = "WIDTH")]
    #[arg(help = "Width of PID/UID column")]
    #[arg(help_heading = FORMATTING_OPTIONS)]
    pub puid_width: u8,

    /// Fixed width of the package/process name column.
    #[arg(short = 'm')]
    #[arg(required = false)]
    #[arg(value_name = "WIDTH")]
    #[arg(default_value_t = 20u8)]
    #[arg(long = "package-width")]
    #[arg(help_heading = FORMATTING_OPTIONS)]
    #[arg(help = "Width of package/process name column")]
    pub package_width: u8,

    /// Fixed width of the log tag column.
    #[arg(short = 'n')]
    #[arg(required = false)]
    #[arg(value_name = "WIDTH")]
    #[arg(long = "tag-width")]
    #[arg(default_value_t = 20u8)]
    #[arg(help = "Width of tag column")]
    #[arg(help_heading = FORMATTING_OPTIONS)]
    pub tag_width: u8,

    /// Apply dedicated colors to garbage-collector log lines.
    #[arg(short = 'g')]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(long = "gc-color")]
    #[arg(default_value_t = false)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = COLORING_OPTIONS)]
    #[arg(help = "Enable garbage collector messages colors")]
    pub gc_color: bool,

    /// Disable ANSI colors in messages and errors.
    #[arg(short = 'N')]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(long = "no-color")]
    #[arg(default_value_t = false)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help = "Disable message colors")]
    #[arg(help_heading = COLORING_OPTIONS)]
    pub no_color: bool,

    /// Optional path to tee rendered output to a file.
    #[arg(short = 'o')]
    #[arg(long = "output")]
    #[arg(required = false)]
    #[arg(default_value = None)]
    #[arg(value_name = "FILE_PATH")]
    #[arg(value_hint = ValueHint::FilePath)]
    #[arg(help_heading = OUTPUT_OPTIONS)]
    #[arg(help = format!("Save output to {metavar}", metavar = "[FILE_PATH]".cyan().bold()))]
    pub output_path: Option<String>,

    /// Force line-oriented stdout instead of the interactive TUI.
    #[arg(long = "plain")]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(default_value_t = false)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = OUTPUT_OPTIONS)]
    #[arg(help = "Use plain text output instead of TUI")]
    pub plain: bool,

    /// Override path to the TOML config file (see [`default_config_file`]).
    #[arg(long = "config")]
    #[arg(required = false)]
    #[arg(default_value = None)]
    #[arg(value_name = "CONFIG_PATH")]
    #[arg(value_hint = ValueHint::FilePath)]
    #[arg(help_heading = CONFIG_OPTIONS)]
    #[arg(help = CliArgs::get_config_help())]
    pub config_path: Option<String>,

    /// Theme name under the themes directory or path to a `.toml` theme file.
    #[arg(long = "theme")]
    #[arg(required = false)]
    #[arg(value_name = "THEME")]
    #[arg(help_heading = CONFIG_OPTIONS)]
    #[arg(default_value = DEFAULT_THEME_NAME)]
    #[arg(help = CliArgs::get_theme_help())]
    pub theme: String,

    /// List bundled and user themes, then exit.
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(long = "list-themes")]
    #[arg(default_value_t = false)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = CONFIG_OPTIONS)]
    #[arg(help = "List the bundled and custom themes in the themes directory and exit")]
    pub list_themes: bool,

    /// Print merged effective config as documented TOML and exit.
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(long = "print-config")]
    #[arg(default_value_t = false)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = CONFIG_OPTIONS)]
    #[arg(help = "Print the effective configuration as a documented config file and exit")]
    pub print_config: bool,

    /// Print the loaded theme as documented TOML and exit.
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(long = "print-theme")]
    #[arg(default_value_t = false)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = CONFIG_OPTIONS)]
    #[arg(help = "Print the active theme as a documented theme file and exit")]
    pub print_theme: bool,

    /// Set by the TUI runner so downstream code knows interactive mode is active.
    #[arg(skip)]
    pub tui_mode: bool,

    /// Plain character width of the timestamp column; zero when [`Self::show_timestamps`] is false.
    #[arg(skip)]
    pub timestamp_width: usize,
}

impl CliArgs {
    /// clap color styles for help, errors, and literals.
    fn get_cli_styles() -> Styles {
        Styles::styled()
            .error(AnsiColor::Red.on_default().bold())
            .valid(AnsiColor::Green.on_default().bold())
            .context(AnsiColor::Cyan.on_default().bold())
            .usage(AnsiColor::Yellow.on_default().bold())
            .header(AnsiColor::Yellow.on_default().bold())
            .literal(AnsiColor::Green.on_default().bold())
            .invalid(AnsiColor::Yellow.on_default().bold())
            .placeholder(AnsiColor::Cyan.on_default().bold())
            .context_value(AnsiColor::Cyan.on_default().bold())
    }

    /// Short `--about` text: executable name, version, and crate description.
    fn get_about() -> String {
        let bin_name = Self::get_name();
        let version = Self::get_version();
        let description = env!("CARGO_PKG_DESCRIPTION");

        format!("{bin_name} {version}\n{description}")
    }

    /// Executable file stem (leaked `'static` str) used as the clap command name.
    fn get_name() -> &'static str {
        let bin_name = std::env::current_exe()
            .unwrap_or_panic("Failed to get current executable path")
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or(env!("CARGO_PKG_NAME").to_string());

        bin_name.leak()
    }

    /// Short version string (`v` + [`CARGO_PKG_VERSION`]), leaked for clap.
    fn get_version() -> &'static str {
        let version = env!("CARGO_PKG_VERSION");

        format!("v{version}").leak()
    }

    /// Multi-line `--version` output including author and description.
    fn get_long_version() -> &'static str {
        let version = Self::get_version();
        let author = env!("CARGO_PKG_AUTHORS");
        let description = env!("CARGO_PKG_DESCRIPTION");

        format!("{version}\n{description}\nAuthor: {author}").leak()
    }

    /// Help text for `--config`, including the default config file path when known.
    fn get_config_help() -> String {
        let metavar = "[CONFIG_PATH]".cyan().bold();
        let default_path = default_config_file()
            .map(|path| path.display().to_string())
            .unwrap_or_default();

        format!(
            "Load configuration from {metavar} instead of the default\nconfig file: {default_path}"
        )
    }

    /// Help text for `--theme`, including themes directory and bundled theme count.
    fn get_theme_help() -> String {
        let metavar = "[THEME]".cyan().bold();
        let themes_dir = themes_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        let bundled = BUNDLED_THEMES.len();

        format!(
            "Color {metavar} name from {themes_dir}\nor path to a .toml theme file\n{bundled} bundled themes are installed there"
        )
    }

    /// Built-in defaults as if no flags were passed (used to seed a first-run config file).
    fn defaults() -> Option<Self> {
        Self::command()
            .try_get_matches_from([env!("CARGO_PKG_NAME")])
            .ok()
            .and_then(|matches| Self::from_arg_matches(&matches).ok())
    }

    /// Parse argv, optionally install default config, merge file config, and compute [`Self::timestamp_width`].
    pub fn parse_args() -> Self {
        let matches = Self::command().get_matches();
        let mut args = Self::from_arg_matches(&matches).unwrap_or_else(|err| err.exit());
        let show_colors = !args.no_color;

        if args.config_path.is_none()
            && let Some(defaults) = Self::defaults()
        {
            Config::install_default_file(&defaults);
        }

        Config::load_effective(args.config_path.as_deref())
            .unwrap_or_else(|err| exit_with_error(&err, show_colors))
            .merge_into(&mut args, &matches);

        let timestamp_width =
            timestamp_column_width(&args.timestamp_format).unwrap_or_else(|err| {
                exit_with_error(&err, show_colors);
            });
        args.timestamp_width = if args.show_timestamps {
            timestamp_width
        } else {
            0usize
        };

        args
    }
}
