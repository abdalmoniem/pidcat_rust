use anyhow::Context;
use anyhow::Error;
use anyhow::Result;

#[cfg(target_os = "windows")]
use std::io::Error as IoError;

#[cfg(target_os = "windows")]
use std::io::ErrorKind as IoErrorKind;

use std::path::PathBuf;

use which::which;

use xshell::Shell;
use xshell::cmd;
use xtask::CliArgs;
use xtask::Command;

/// Set terminal title to [msg]
fn status(msg: &str) {
    print!("\u{1b}]0;{msg}\u{07}");
    println!();
    println!("{msg}");
}

fn cargo() -> Result<PathBuf> {
    std::env::var_os("CARGO")
        .map_or(which("cargo"), |exe| Ok(PathBuf::from(exe)))
        .context("Couldn't find 'cargo' executable")
}

fn run(shell: &Shell, args: &[String]) -> Result<()> {
    status(">> Running...");

    cargo()
        .and_then(|cargo| {
            cmd!(shell, "{cargo} run -- {args...}")
                .quiet()
                .run()
                .map_err(Error::new)
        })
        .with_context(|| "failed to run!")
}

fn run_release(shell: &Shell, args: &[String]) -> Result<()> {
    status(">> Running Release...");

    cargo()
        .and_then(|cargo| {
            cmd!(shell, "{cargo} run --release -- {args...}")
                .quiet()
                .run()
                .map_err(Error::new)
        })
        .with_context(|| "failed to run release!")
}

fn clean(shell: &Shell) -> Result<()> {
    status(">> Cleaning...");

    cargo()
        .and_then(|cargo| {
            cmd!(shell, "{cargo} clean --package pidcat")
                .quiet()
                .run()
                .map_err(Error::new)
        })
        .with_context(|| "failed to run clean!")?;

    cargo()
        .and_then(|cargo| {
            cmd!(shell, "{cargo} clean --release --package pidcat")
                .quiet()
                .run()
                .map_err(Error::new)
        })
        .with_context(|| "failed to clean!")
}

fn build(shell: &Shell) -> Result<()> {
    status(">> Building...");

    cargo()
        .and_then(|cargo| {
            cmd!(shell, "{cargo} build")
                .quiet()
                .run()
                .map_err(Error::new)
        })
        .with_context(|| "failed to build!")
}

fn build_release(shell: &Shell) -> Result<()> {
    status(">> Building Release...");

    cargo()
        .and_then(|cargo| {
            cmd!(shell, "{cargo} build --release")
                .quiet()
                .run()
                .map_err(Error::new)
        })
        .with_context(|| "failed to build release!")
}

#[cfg(target_os = "windows")]
fn build_installer(shell: &Shell, iscc_path: Option<PathBuf>) -> Result<()> {
    status(">> Building Installer...");

    iscc_path
        .map_or(which("iscc"), Ok)
        .context("Couldn't find 'iscc' executable")
        .and_then(|iscc| {
            cmd!(shell, "{iscc} build/setup/setup.iss")
                .quiet()
                .run()
                .map_err(Error::new)
        })
        .with_context(|| "failed to build installer!")
}

#[cfg(not(target_os = "windows"))]
fn install(shell: &Shell) -> Result<()> {
    status(">> Installing pidcat...");

    cargo()
        .and_then(|cargo| {
            cmd!(shell, "{cargo} install --path .")
                .quiet()
                .run()
                .map_err(Error::new)
        })
        .with_context(|| "failed to install!")
}

#[cfg(target_os = "windows")]
fn install(shell: &Shell, silent: bool) -> Result<()> {
    std::fs::read_dir("build/setup/output")?
        .flatten()
        .filter(|entry| entry.path().is_file())
        .max_by_key(|entry| {
            entry
                .metadata()
                .ok()
                .and_then(|metadata| metadata.modified().ok())
        })
        .ok_or(IoError::new(
            IoErrorKind::NotFound,
            "no installation files found!",
        ))
        .context("failed to locate installation file!")
        .map(|entry| entry.path())
        .and_then(|installer_exe| {
            status(&format!(">> Installing {installer_exe:?}..."));

            let silent_arg = if silent { "/verysilent" } else { "" };
            cmd!(shell, "{installer_exe} {silent_arg}")
                .quiet()
                .run()
                .map_err(Error::new)
        })
        .with_context(|| "failed to install!")
}

fn main() -> Result<()> {
    let args = CliArgs::parse_args();
    let shell = Shell::new()?;

    match args.command {
        Command::Run { args } => run(&shell, &args),
        Command::RunRelease { args } => run_release(&shell, &args),
        Command::Clean => clean(&shell),
        Command::Build => build(&shell),
        Command::BuildRelease => build_release(&shell),
        Command::Rebuild { dev, release } => clean(&shell)
            .and_then(|_| match dev || !release {
                true => build(&shell),
                false => Ok(()),
            })
            .and_then(|_| match release {
                true => build_release(&shell),
                false => Ok(()),
            }),

        #[cfg(target_os = "windows")]
        Command::BuildInstaller { iscc_path } => build_installer(&shell, iscc_path),

        #[cfg(target_os = "windows")]
        Command::BuildAll {
            iscc_path,
            dev,
            release,
        } => match dev || !release {
            true => build(&shell),
            false => Ok(()),
        }
        .and_then(|_| match release {
            true => build_release(&shell),
            false => Ok(()),
        })
        .and_then(|_| build_installer(&shell, iscc_path)),

        #[cfg(target_os = "windows")]
        Command::Install { silent } => install(&shell, silent),

        #[cfg(not(target_os = "windows"))]
        Command::Install => install(&shell),

        #[cfg(target_os = "windows")]
        Command::Reinstall { iscc_path, silent } => clean(&shell)
            .and_then(|_| build_release(&shell))
            .and_then(|_| build_installer(&shell, iscc_path))
            .and_then(|_| install(&shell, silent)),

        #[cfg(not(target_os = "windows"))]
        Command::Reinstall => clean(&shell)
            .and_then(|_| build_release(&shell))
            .and_then(|_| install(&shell)),
    }
}
