use clap::{Parser, ValueEnum, builder::styling};
use std::{env, fmt};

const POSITIONAL_ARGUMENTS: &str = "Positional Arguments";
const ABOUT_OPTIONS: &str = "Options";
const DEVICE_OPTIONS: &str = "Device Options";
const FILTERING_OPTIONS: &str = "Filtering Options";
const FORMATTING_OPTIONS: &str = "Formatting Options";
const COLORING_OPTIONS: &str = "Color Options";
const OUTPUT_OPTIONS: &str = "Output Options";

#[derive(Eq, Ord, Copy, Debug, Clone, ValueEnum, PartialEq, PartialOrd)]
pub enum LogLevel {
    #[value(alias = "v")]
    VERBOSE = 0,

    #[value(alias = "d")]
    DEBUG = 1,

    #[value(alias = "i")]
    INFO = 2,

    #[value(alias = "w")]
    WARN = 3,

    #[value(alias = "e")]
    ERROR = 4,

    #[value(alias = "f")]
    FATAL = 5,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let letter = match self {
            LogLevel::VERBOSE => "V",
            LogLevel::DEBUG => "D",
            LogLevel::INFO => "I",
            LogLevel::WARN => "W",
            LogLevel::ERROR => "E",
            LogLevel::FATAL => "F",
        };
        write!(formatter, "{}", letter)
    }
}

#[derive(Debug, Parser)]
#[command(
    disable_help_flag = true,
    disable_version_flag = true,
    arg_required_else_help = false,
    color = clap::ColorChoice::Auto,
    styles = CliArgs::get_cli_styles(),
    version = concat!("v", env!("CARGO_PKG_VERSION")),
    long_version = concat!("v", env!("CARGO_PKG_VERSION")),
)]
pub struct CliArgs {
    #[arg(
        required = false,
        value_name = "PACKAGE",
        help_heading = POSITIONAL_ARGUMENTS,
        help = concat!(
            "Application package name(s)",
            "\nThis can be specified multiple times"
        ),
    )]
    pub packages: Vec<String>,

    #[arg(
        short = 'h',
        long = "help",
        required = false,
        value_name = None,
        help_heading = ABOUT_OPTIONS,
        action = clap::ArgAction::Help,
        help = "Show this help message and exit",
    )]
    pub help: Option<bool>,

    #[arg(
        short = 'v',
        long = "version",
        required = false,
        value_name = None,
        help_heading = ABOUT_OPTIONS,
        action = clap::ArgAction::Version,
        help = "Print the version number and exit",
    )]
    pub version: Option<bool>,

    #[arg(
        short = 'd',
        long = "device",
        required = false,
        value_name = None,
        default_value_t = false,
        help_heading = DEVICE_OPTIONS,
        action = clap::ArgAction::SetTrue,
        help = "Use first device for log input",
    )]
    pub use_device: bool,

    #[arg(
        short = 'e',
        required = false,
        long = "emulator",
        value_name = None,
        default_value_t = false,
        help_heading = DEVICE_OPTIONS,
        action = clap::ArgAction::SetTrue,
        help = "Use first emulator for log input",
    )]
    pub use_emulator: bool,

    #[arg(
        short = 's',
        long = "serial",
        required = false,
        default_value = None,
        value_name = "DEVICE_SERIAL",
        help_heading = DEVICE_OPTIONS,
        help = "Use first emulator for log input",
    )]
    pub device_serial: Option<String>,

    #[arg(
        short = 'a',
        long = "all",
        required = false,
        value_name = None,
        default_value_t = false,
        help_heading = FILTERING_OPTIONS,
        action = clap::ArgAction::SetTrue,
        help = "Print log messages from all packages",
    )]
    pub all: bool,

    #[arg(
        short = 'k',
        long = "keep",
        required = false,
        value_name = None,
        default_value_t = false,
        help_heading = FILTERING_OPTIONS,
        action = clap::ArgAction::SetTrue,
        help = "Keep the entire log before running",
    )]
    pub keep_logcat: bool,

    #[arg(
        short = 'c',
        long = "current",
        required = false,
        value_name = None,
        default_value_t = false,
        help_heading = FILTERING_OPTIONS,
        action = clap::ArgAction::SetTrue,
        help = "Filter logcat by current running app(s)",
    )]
    pub current_app: bool,

    #[arg(
        short = 'I',
        long = "ignore-system-tags",
        required = false,
        value_name = None,
        default_value_t = false,
        help_heading = FILTERING_OPTIONS,
        action = clap::ArgAction::SetTrue,
        help = concat!(
            "Filter output by ignoring known system tags",
            "\nUse --ignore-tag to ignore additional tags if needed"
        ),
    )]
    pub ignore_system_tags: bool,

    #[arg(
        short = 't',
        long = "tag",
        required = false,
        value_name = "TAG",
        default_value = None,
        help_heading = FILTERING_OPTIONS,
        help = concat!(
            "Filter output by specified tag(s)",
            "\nThis can be specified multiple times, or as a comma separated list"
        ),
    )]
    pub tag: Option<Vec<String>>,

    #[arg(
        short = 'i',
        required = false,
        long = "ignore-tag",
        default_value = None,
        value_name = "IGNORED_TAG",
        help_heading = FILTERING_OPTIONS,
        help = concat!(
                "Filter output by ignoring specified tag(s)",
                "\nThis can be specified multiple times, or as a comma separated list"
            ),
    )]
    pub ignore_tag: Option<Vec<String>>,

    #[arg(
        short = 'l',
        long = "log-level",
        ignore_case = true,
        default_value = "v",
        value_name = "LEVEL",
        help_heading = FILTERING_OPTIONS,
        help = "Filter messages lower than minimum log level",
    )]
    pub log_level: LogLevel,

    #[arg(
        short = 'r',
        long = "regex",
        required = false,
        value_name = "REGEX",
        default_value = None,
        help_heading = FILTERING_OPTIONS,
        help = "Filter output messages using the specified [REGEX]",
    )]
    pub regex: Option<String>,

    #[arg(
        short = 'P',
        required = false,
        long = "show-pid",
        value_name = None,
        default_value_t = false,
        help = "Show PID in output",
        help_heading = FORMATTING_OPTIONS,
        action = clap::ArgAction::SetTrue,
    )]
    pub show_pid: bool,

    #[arg(
        short = 'p',
        required = false,
        value_name = None,
        long = "show-package",
        default_value_t = false,
        help_heading = FORMATTING_OPTIONS,
        action = clap::ArgAction::SetTrue,
        help = "Show package name in output",
    )]
    pub show_package: bool,

    #[arg(
        short = 'S',
        required = false,
        value_name = None,
        long = "always-show-tags",
        default_value_t = false,
        help_heading = FORMATTING_OPTIONS,
        action = clap::ArgAction::SetTrue,
        help = "Always show the tag name",
    )]
    pub always_show_tags: bool,

    #[arg(
        short = 'x',
        required = false,
        value_name = "X",
        long = "pid-width",
        default_value_t = 5,
        help = "Width of PID column",
        help_heading = FORMATTING_OPTIONS,
    )]
    pub pid_width: u8,

    #[arg(
        short = 'n',
        required = false,
        value_name = "N",
        default_value_t = 20,
        long = "package-width",
        help_heading = FORMATTING_OPTIONS,
        help = "Width of package/process name column",
    )]
    pub package_width: u8,

    #[arg(
        short = 'm',
        required = false,
        value_name = "M",
        long = "tag-width",
        default_value_t = 20,
        help = "Width of tag column",
        help_heading = FORMATTING_OPTIONS,
    )]
    pub tag_width: u8,

    #[arg(
        short = 'g',
        required = false,
        value_name = None,
        long = "gc-color",
        default_value_t = false,
        help_heading = COLORING_OPTIONS,
        action = clap::ArgAction::SetTrue,
        help = "Enable garbage collector messages colors",
    )]
    pub gc_color: bool,

    #[arg(
        short = 'N',
        required = false,
        value_name = None,
        long = "no-color",
        default_value_t = false,
        help_heading = COLORING_OPTIONS,
        help = "Disable message colors",
        action = clap::ArgAction::SetTrue,
    )]
    pub no_color: bool,

    #[arg(
        short = 'o',
        long = "output",
        required = false,
        value_name = "FILE_PATH",
        default_value = None,
        help_heading = OUTPUT_OPTIONS,
        help = "Save output to [FILE_PATH]",
    )]
    pub output_path: Option<String>,
}

impl CliArgs {
    fn get_cli_styles() -> styling::Styles {
        styling::Styles::styled()
            .valid(styling::AnsiColor::Green.on_default())
            .invalid(styling::AnsiColor::Yellow.on_default())
            .error(styling::AnsiColor::Red.on_default().bold())
            .placeholder(styling::AnsiColor::Yellow.on_default())
            .context(styling::AnsiColor::Cyan.on_default().bold())
            .literal(styling::AnsiColor::Green.on_default().bold())
            .context_value(styling::AnsiColor::Cyan.on_default().bold())
            .usage(styling::AnsiColor::Blue.on_default().underline().bold())
            .header(styling::AnsiColor::Blue.on_default().underline().bold())
    }

    pub fn parse_args() -> Self {
        Self::parse()
    }
}
