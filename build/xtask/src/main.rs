use anyhow::Context;
use anyhow::Error;
use anyhow::Result;

use std::path::PathBuf;
use std::time::Instant;

#[cfg(target_os = "windows")]
use std::{fs::Metadata, fs::read_dir, io::Error as IoError, io::ErrorKind as IoErrorKind};

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
    let cmd = |cargo| {
        status(">> Running...");

        cmd!(shell, "{cargo} run -- {args...}")
            .quiet()
            .run()
            .map_err(Error::new)
    };

    cargo().and_then(cmd).context("failed to run release!")
}

fn run_release(shell: &Shell, args: &[String]) -> Result<()> {
    let cmd = |cargo| {
        status(">> Running Release...");

        cmd!(shell, "{cargo} run --release -- {args...}")
            .quiet()
            .run()
            .map_err(Error::new)
    };

    cargo().and_then(cmd).context("failed to run release!")
}

fn clean(shell: &Shell) -> Result<()> {
    let cmd = |(cargo, release)| {
        match release {
            true => status(">> Cleaning Release..."),
            false => status(">> Cleaning..."),
        }

        let release_flag = if release { "--release" } else { "" };
        cmd!(shell, "{cargo} clean {release_flag} --package pidcat")
            .quiet()
            .run()
            .map_err(Error::new)
    };

    cargo()
        .map(|res| (res, false))
        .and_then(cmd)
        .context("failed to clean!")?;

    cargo()
        .map(|res| (res, true))
        .and_then(cmd)
        .context("failed to clean release!")
}

fn build(shell: &Shell) -> Result<()> {
    let cmd = |cargo| {
        status(">> Building...");

        cmd!(shell, "{cargo} build")
            .quiet()
            .run()
            .map_err(Error::new)
    };

    cargo().and_then(cmd).context("failed to build release!")
}

fn build_release(shell: &Shell) -> Result<()> {
    let cmd = |cargo| {
        status(">> Building Release...");

        cmd!(shell, "{cargo} build --release")
            .quiet()
            .run()
            .map_err(Error::new)
    };

    cargo().and_then(cmd).context("failed to build release!")
}

#[cfg(target_os = "windows")]
fn build_installer(shell: &Shell, iscc_path: Option<PathBuf>) -> Result<()> {
    let cmd = |iscc| {
        cmd!(shell, "{iscc} build/setup/setup.iss")
            .quiet()
            .run()
            .map_err(Error::new)
    };
    status(">> Building Installer...");

    iscc_path
        .map_or(which("iscc"), Ok)
        .context("Couldn't find 'iscc' executable")
        .and_then(cmd)
        .context("failed to build installer!")
}

#[cfg(not(target_os = "windows"))]
fn install(shell: &Shell) -> Result<()> {
    let cmd = |cargo| {
        status(">> Installing pidcat...");

        cmd!(shell, "{cargo} install --path .")
            .quiet()
            .run()
            .map_err(Error::new)
    };

    cargo().and_then(cmd).context("failed to install!")
}

#[cfg(target_os = "windows")]
fn install(shell: &Shell, silent: bool) -> Result<()> {
    let cmd = |installer_exe| {
        status(&format!(">> Installing {installer_exe:?}..."));

        let silent_arg = if silent { "/verysilent" } else { "" };
        cmd!(shell, "{installer_exe} {silent_arg}")
            .quiet()
            .run()
            .map_err(Error::new)
    };

    let not_found_err = IoError::new(IoErrorKind::NotFound, "no setup files found!");
    let max_pred = |metadata: Metadata| metadata.modified().ok();

    read_dir("build/setup/output")
        .context("setup output dir not found!")?
        .flatten()
        .filter(|entry| entry.path().is_file())
        .max_by_key(|entry| entry.metadata().ok().and_then(max_pred))
        .ok_or(not_found_err)
        .context("failed to locate installation file!")
        .map(|entry| entry.path())
        .and_then(cmd)
        .context("failed to install!")
}

fn main() -> Result<()> {
    let args = CliArgs::parse_args();
    let shell = Shell::new()?;

    let instant = Instant::now();

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
    }?;

    let elapsed = instant.elapsed();
    println!(">> Command took: {elapsed:?}");

    Ok(())
}
