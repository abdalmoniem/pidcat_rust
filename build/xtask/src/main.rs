use anyhow::Context;
use anyhow::Error;
use anyhow::Result;

use clap::error::DefaultFormatter as ClapFormatter;
use clap::error::Error as ClapError;
use clap::error::ErrorKind as ClapErrorKind;

use scope_functions::Run;

use std::env::var_os;
use std::path::PathBuf;
use std::time::Instant;

#[cfg(target_os = "windows")]
use std::{fs::Metadata, fs::read_dir, io::Error as IoError, io::ErrorKind as IoErrorKind};

use which::which;

use xshell::Shell;
use xshell::cmd;
use xtask::CliArgs;
use xtask::Command;
use xtask::Profile;

/// Set terminal title to [msg]
fn status(msg: &str) {
    print!("\u{1b}]0;{msg}\u{07}");
    println!();
    println!("{msg}");
}

/// Get the cargo executable from the CARGO
/// system environment variables or look for
/// it in the system PATH variable
fn cargo() -> Result<PathBuf> {
    var_os("CARGO")
        .map_or(which("cargo"), |cargo_exe| Ok(PathBuf::from(cargo_exe)))
        .context("Couldn't find 'cargo' executable")
}

/// Clean the build artifacts for the pidcat package
fn clean(shell: &Shell, profile: &Profile) -> Result<()> {
    let clean_cmd = |cargo| {
        let dev = matches!(profile, Profile::Development | Profile::Both);
        let release = matches!(profile, Profile::Release | Profile::Both);

        if dev {
            status(">> Cleaning...");

            cmd!(shell, "{cargo} clean --package pidcat")
                .quiet()
                .run()
                .map_err(Error::new)?;
        }

        if release {
            status(">> Cleaning Release...");

            cmd!(shell, "{cargo} clean --release --package pidcat")
                .quiet()
                .run()
                .map_err(Error::new)?;
        }

        Ok(())
    };

    cargo().and_then(clean_cmd).context("failed to clean!")
}

/// Build PidCat
fn build(shell: &Shell, profile: &Profile) -> Result<()> {
    let build_cmd = |cargo| {
        let dev = matches!(profile, Profile::Development | Profile::Both);
        let release = matches!(profile, Profile::Release | Profile::Both);

        if dev {
            status(">> Building...");

            cmd!(shell, "{cargo} build")
                .quiet()
                .run()
                .map_err(Error::new)?;
        }

        if release {
            status(">> Building Release...");

            cmd!(shell, "{cargo} build --release")
                .quiet()
                .run()
                .map_err(Error::new)?;
        }

        Ok(())
    };

    cargo().and_then(build_cmd).context("failed to build!")
}

/// Build the Inno Setup Installer for PidCat
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

/// Run PidCat
fn run(shell: &Shell, profile: &Profile, args: &[String]) -> Result<()> {
    let cmd = |cargo| {
        let run_profile = match profile {
            Profile::Development => "",
            Profile::Release => "--release",
            Profile::Both => {
                unreachable!("can not run both 'dev' and 'release' profiles! how did we get here!")
            }
        };
        cmd!(shell, "{cargo} run {run_profile} -- {args...}")
            .quiet()
            .run()
            .map_err(Error::new)
    };

    let clap_err = ClapError::<ClapFormatter>::raw(
        ClapErrorKind::InvalidValue,
        "either 'dev' or 'release' is allowed",
    );
    let both_prof_err = Err(clap_err).context("can not run both 'dev' and 'release' profiles!");

    match profile {
        Profile::Development => status(">> Running..."),
        Profile::Release => status(">> Running Release..."),
        Profile::Both => return both_prof_err,
    }

    cargo().and_then(cmd).context("failed to run!")
}

/// Install PidCat using the Inno Setup Installer
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

/// Install PidCat using cargo install
#[cfg(not(target_os = "windows"))]
fn install(shell: &Shell) -> Result<()> {
    let cmd = |cargo| {
        status(">> Installing pidcat...");

        cmd!(shell, "{cargo} install --locked --path .")
            .quiet()
            .run()
            .map_err(Error::new)
    };

    cargo().and_then(cmd).context("failed to install!")
}

/// Main entry point for the xtask
fn main() -> Result<()> {
    let command = CliArgs::parse_args().command;
    let shell = Shell::new()?;

    let instant = Instant::now();

    match command {
        Command::Clean { profile } => clean(&shell, &profile),

        Command::Build { profile } => build(&shell, &profile),

        Command::Rebuild { profile } => clean(&shell, &profile)?.run(|_| build(&shell, &profile)),

        #[cfg(target_os = "windows")]
        Command::BuildInstaller { iscc_path } => build_installer(&shell, iscc_path),

        #[cfg(target_os = "windows")]
        Command::BuildAll { profile, iscc_path } => {
            build(&shell, &profile)?.run(|_| build_installer(&shell, iscc_path))
        }

        Command::Run { profile, args } => run(&shell, &profile, &args),

        #[cfg(target_os = "windows")]
        Command::Install { silent } => install(&shell, silent),

        #[cfg(not(target_os = "windows"))]
        Command::Install => install(&shell),

        #[cfg(target_os = "windows")]
        Command::Reinstall { iscc_path, silent } => clean(&shell, &Profile::Release)?
            .run(|_| build(&shell, &Profile::Release))?
            .run(|_| build_installer(&shell, iscc_path))?
            .run(|_| install(&shell, silent)),

        #[cfg(not(target_os = "windows"))]
        Command::Reinstall => clean(&shell, &Profile::Release)?
            .run(|_| build(&shell, &Profile::Release))?
            .run(|_| install(&shell)),
    }?
    .run(|_| {
        println!();
        println!(">> Command took: {elapsed:?}", elapsed = instant.elapsed());

        Ok(())
    })
}
