use clap::ArgAction;
use clap::ColorChoice;
use clap::Parser;

use clap::builder::styling::AnsiColor;
use clap::builder::styling::Styles;

use clap_complete::Shell;

use colored::Colorize;

use crate::LogFormat;
use crate::LogFormatKind;
use crate::LogFormatParser;
use crate::LogLevel;
use crate::ValueOrPanic;

const POSITIONAL_ARGUMENTS: &str = "Positional Arguments";
const ABOUT_OPTIONS: &str = "Options";
const DEVICE_OPTIONS: &str = "Device Options";
const FILTERING_OPTIONS: &str = "Filtering Options";
const FORMATTING_OPTIONS: &str = "Formatting Options";
const COLORING_OPTIONS: &str = "Color Options";
const OUTPUT_OPTIONS: &str = "Output Options";

#[derive(Debug, Parser)]
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
    #[arg(required = false)]
    #[arg(value_name = "PACKAGE")]
    #[arg(help_heading = POSITIONAL_ARGUMENTS)]
    #[arg(help = "Application package name(s)\nThis can be specified multiple times")]
    pub packages: Vec<String>,

    #[arg(short = 'h')]
    #[arg(long = "help")]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(action = ArgAction::Help)]
    #[arg(help_heading = ABOUT_OPTIONS)]
    #[arg(help = "Show this help message and exit")]
    pub help: Option<bool>,

    #[arg(short = 'v')]
    #[arg(long = "version")]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(action = ArgAction::Version)]
    #[arg(help_heading = ABOUT_OPTIONS)]
    #[arg(help = "Print the version number and exit")]
    pub version: Option<bool>,

    #[arg(required = false)]
    #[arg(long = "completions")]
    #[arg(value_name = "SHELL")]
    #[arg(help_heading = ABOUT_OPTIONS)]
    #[arg(help = format!("Generate shell completions for {metavar}", metavar = "[SHELL]".cyan().bold()))]
    pub completions: Option<Shell>,

    #[arg(short = 'A')]
    #[arg(long = "adb")]
    #[arg(required = false)]
    #[arg(default_value = None)]
    #[arg(value_name = "ADB_PATH")]
    #[arg(help_heading = ABOUT_OPTIONS)]
    #[arg(help = "Path to adb executable (if not in PATH)")]
    pub adb_path: Option<String>,

    #[arg(short = 'd')]
    #[arg(long = "device")]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(default_value_t = false)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = DEVICE_OPTIONS)]
    #[arg(help = "Use first device for log input")]
    pub use_device: bool,

    #[arg(short = 'e')]
    #[arg(required = false)]
    #[arg(long = "emulator")]
    #[arg(value_name = None)]
    #[arg(default_value_t = false)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = DEVICE_OPTIONS)]
    #[arg(help = "Use first emulator for log input")]
    pub use_emulator: bool,

    #[arg(short = 's')]
    #[arg(long = "serial")]
    #[arg(required = false)]
    #[arg(default_value = None)]
    #[arg(value_name = "DEVICE_SERIAL")]
    #[arg(help_heading = DEVICE_OPTIONS)]
    #[arg(help = format!("Use {metavar} for log input", metavar = "[DEVICE_SERIAL]".cyan().bold()))]
    pub device_serial: Option<String>,

    #[arg(short = 'a')]
    #[arg(long = "all")]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(default_value_t = false)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = FILTERING_OPTIONS)]
    #[arg(help = "Print log messages from all packages")]
    pub all: bool,

    #[arg(short = 'k')]
    #[arg(long = "keep")]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(default_value_t = false)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = FILTERING_OPTIONS)]
    #[arg(help = "Keep the entire log before running")]
    pub keep_logcat: bool,

    #[arg(short = 'c')]
    #[arg(long = "current")]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(default_value_t = false)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = FILTERING_OPTIONS)]
    #[arg(help = "Filter logcat by current running app(s)")]
    pub current_app: bool,

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

    #[arg(short = 'l')]
    #[arg(long = "log-level")]
    #[arg(ignore_case = true)]
    #[arg(value_name = "LEVEL")]
    #[arg(help_heading = FILTERING_OPTIONS)]
    #[arg(default_value_t = LogLevel::VERBOSE)]
    #[arg(help = "Filter messages lower than minimum log level")]
    pub log_level: LogLevel,

    #[arg(short = 'r')]
    #[arg(long = "regex")]
    #[arg(required = false)]
    #[arg(value_name = "REGEX")]
    #[arg(default_value = None)]
    #[arg(help_heading = FILTERING_OPTIONS)]
    #[arg(help = format!("Filter output messages using the specified {metavar}", metavar = "[REGEX]".cyan().bold()))]
    pub regex: Option<String>,

    #[arg(short = 'f')]
    #[arg(long = "log-format")]
    #[arg(ignore_case = true)]
    #[arg(value_name = "FORMAT")]
    #[arg(help_heading = FORMATTING_OPTIONS)]
    #[arg(default_value_t = LogFormat::new(LogFormatKind::Brief))]
    #[arg(help = "Input log format from adb")]
    #[arg(value_parser = LogFormatParser)]
    pub log_format: LogFormat,

    #[arg(short = 'P')]
    #[arg(required = false)]
    #[arg(long = "show-pid")]
    #[arg(value_name = None)]
    #[arg(default_value_t = false)]
    #[arg(help = "Show PID in output")]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = FORMATTING_OPTIONS)]
    pub show_pid: bool,

    #[arg(short = 'p')]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(long = "show-package")]
    #[arg(default_value_t = false)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = FORMATTING_OPTIONS)]
    #[arg(help = "Show package name in output")]
    pub show_package: bool,

    #[arg(short = 'S')]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(default_value_t = false)]
    #[arg(long = "always-show-tags")]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help = "Always show the tag name")]
    #[arg(help_heading = FORMATTING_OPTIONS)]
    pub always_show_tags: bool,

    #[arg(short = 'x')]
    #[arg(required = false)]
    #[arg(long = "pid-width")]
    #[arg(default_value_t = 5u8)]
    #[arg(value_name = "WIDTH")]
    #[arg(help = "Width of PID column")]
    #[arg(help_heading = FORMATTING_OPTIONS)]
    pub pid_width: u8,

    #[arg(short = 'n')]
    #[arg(required = false)]
    #[arg(value_name = "WIDTH")]
    #[arg(default_value_t = 20u8)]
    #[arg(long = "package-width")]
    #[arg(help_heading = FORMATTING_OPTIONS)]
    #[arg(help = "Width of package/process name column")]
    pub package_width: u8,

    #[arg(short = 'm')]
    #[arg(required = false)]
    #[arg(value_name = "WIDTH")]
    #[arg(long = "tag-width")]
    #[arg(default_value_t = 20u8)]
    #[arg(help = "Width of tag column")]
    #[arg(help_heading = FORMATTING_OPTIONS)]
    pub tag_width: u8,

    #[arg(short = 'g')]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(long = "gc-color")]
    #[arg(default_value_t = false)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help_heading = COLORING_OPTIONS)]
    #[arg(help = "Enable garbage collector messages colors")]
    pub gc_color: bool,

    #[arg(short = 'N')]
    #[arg(required = false)]
    #[arg(value_name = None)]
    #[arg(long = "no-color")]
    #[arg(default_value_t = false)]
    #[arg(action = ArgAction::SetTrue)]
    #[arg(help = "Disable message colors")]
    #[arg(help_heading = COLORING_OPTIONS)]
    pub no_color: bool,

    #[arg(short = 'o')]
    #[arg(long = "output")]
    #[arg(required = false)]
    #[arg(default_value = None)]
    #[arg(value_name = "FILE_PATH")]
    #[arg(help_heading = OUTPUT_OPTIONS)]
    #[arg(help = format!("Save output to {metavar}", metavar = "[FILE_PATH]".cyan().bold()))]
    pub output_path: Option<String>,
}

impl CliArgs {
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

    fn get_about() -> String {
        let bin_name = Self::get_name();
        let version = Self::get_version();
        let description = env!("CARGO_PKG_DESCRIPTION");

        format!("{bin_name} {version}\n{description}")
    }

    fn get_name() -> &'static str {
        let bin_name = std::env::current_exe()
            .unwrap_or_panic("Failed to get current executable path")
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or(env!("CARGO_PKG_NAME").to_string());

        bin_name.leak()
    }

    fn get_version() -> &'static str {
        let version = env!("CARGO_PKG_VERSION");

        format!("v{version}").leak()
    }

    fn get_long_version() -> &'static str {
        let version = Self::get_version();
        let author = env!("CARGO_PKG_AUTHORS");
        let description = env!("CARGO_PKG_DESCRIPTION");

        format!("{version}\n{description}\nAuthor: {author}").leak()
    }

    pub fn parse_args() -> Self {
        Self::parse()
    }
}
