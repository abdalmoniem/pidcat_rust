use clap::ColorChoice;
use clap::CommandFactory;
use clap::Parser;
use clap::Subcommand;
use clap::builder::styling::AnsiColor;
use clap::builder::styling::Styles;
use clap::error::ErrorKind;

use scope_functions::Run;

#[cfg(target_os = "windows")]
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(color = ColorChoice::Auto)]
#[command(name = CliArgs::get_name())]
#[command(about = CliArgs::get_about())]
#[command(arg_required_else_help = false)]
#[command(version = CliArgs::get_version())]
#[command(styles = CliArgs::get_cli_styles())]
#[command(long_version = CliArgs::get_long_version())]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the pidcat binary
    Run {
        /// Arguments to pass to the binary
        #[arg(last = true)]
        args: Vec<String>,
    },

    /// Run the pidcat release binary
    RunRelease {
        /// Arguments to pass to the binary
        #[arg(last = true)]
        args: Vec<String>,
    },

    /// Clean generated files
    Clean,

    /// Build the pidcat binary
    Build,

    /// Build the pidcat release binary
    BuildRelease,

    /// Rebuild the executable package
    Rebuild {
        /// Build dev binary
        #[arg(short = 'd', long = "dev", default_value_t = false)]
        dev: bool,

        /// Build release binary
        #[arg(short = 'r', long = "release", default_value_t = false)]
        release: bool,
    },

    #[cfg(target_os = "windows")]
    /// Build the installer using Inno Setup Compiler
    BuildInstaller {
        /// Path to Inno Setup Compiler (ISCC) executable
        #[arg(short = 'p', long = "iscc-path", value_name = "ISCC_PATH")]
        iscc_path: Option<PathBuf>,
    },

    #[cfg(target_os = "windows")]
    /// Build both the executable and installer packages
    BuildAll {
        /// Build dev binary
        #[arg(short = 'd', long = "dev", default_value_t = false)]
        dev: bool,

        /// Build release binary
        #[arg(short = 'r', long = "release", default_value_t = false)]
        release: bool,

        /// Path to Inno Setup Compiler (ISCC) executable
        #[arg(short = 'p', long = "iscc-path", value_name = "ISCC_PATH")]
        iscc_path: Option<PathBuf>,
    },

    #[cfg(target_os = "windows")]
    /// Install the application by running the generated installer
    Install {
        /// Perform a silent install
        #[arg(short = 's', long = "silent", default_value_t = false)]
        silent: bool,
    },

    #[cfg(not(target_os = "windows"))]
    /// Install the application by running the generated installer
    Install,

    #[cfg(target_os = "windows")]
    /// Perform a full rebuild, create the installer, and install the application
    Reinstall {
        /// Path to Inno Setup Compiler (ISCC) executable
        #[arg(short = 'p', long = "iscc-path", value_name = "ISCC_PATH")]
        iscc_path: Option<PathBuf>,

        /// Perform a silent install
        #[arg(short = 's', long = "silent", default_value_t = false)]
        silent: bool,
    },

    #[cfg(not(target_os = "windows"))]
    /// Perform a full rebuild, create the installer, and install the application
    Reinstall,
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
            .expect("Failed to get current executable path")
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
        match Self::try_parse() {
            Ok(args) => args,
            Err(err) => {
                if err.kind() == ErrorKind::MissingSubcommand {
                    Self::command()
                        .render_long_help()
                        .run(|help| println!("{ansi_help}", ansi_help = help.ansi()));
                } else {
                    err.exit();
                }

                std::process::exit(0);
            }
        }
    }
}
