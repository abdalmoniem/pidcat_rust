// Copyright (C) AbdAlMoniem AlHifnawy <hifnawy_moniem@hotmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! `xtask` binary: developer workflows for building, packaging, and validating `pidcatrs`.
//!
//! Invoked as `cargo xtask …` (or via project `just` recipes). Commands delegate to `cargo`,
//! Inno Setup on Windows, and library helpers from the main `pidcatrs` crate for JSON Schema and
//! bundled theme generation.
//!
//! # Commands
//!
//! See [`xtask::Command`] for the full clap subcommand tree. Highlights:
//!
//! - **Build / clean / rebuild / run** — wrap `cargo` with profile selection ([`Profile`]).
//! - **Schema** — write or check `schemas/*.json` against [`pidcatrs::config_schema`] and
//!   [`pidcatrs::theme_schema`].
//! - **Themes** — render documented bundled theme TOML under `src/config/themes`.
//! - **Windows-only** — build or run the Inno Setup installer, install from `build/setup/output`.
//! - **Non-Windows** — `cargo install --path .` for local installation.

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;

use clap::error::DefaultFormatter as ClapFormatter;
use clap::error::Error as ClapError;
use clap::error::ErrorKind as ClapErrorKind;

use pidcatrs::config_schema;
use pidcatrs::render_theme_source;
use pidcatrs::theme_schema;

use scope_functions::Run;

use std::env::var_os;
use std::fs::create_dir_all;
use std::fs::read_dir;
use std::fs::read_to_string;
use std::fs::write;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

#[cfg(target_os = "windows")]
use std::{fs::Metadata, io::Error as IoError, io::ErrorKind as IoErrorKind};

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

/// Clean the build artifacts for the pidcatrs package
fn clean(shell: &Shell, profile: &Profile) -> Result<()> {
    let clean_cmd = |cargo| {
        let dev = matches!(profile, Profile::Development | Profile::Both);
        let release = matches!(profile, Profile::Release | Profile::Both);

        if dev {
            status(">> Cleaning...");

            cmd!(shell, "{cargo} clean --package pidcatrs")
                .quiet()
                .run()
                .map_err(Error::new)?;
        }

        if release {
            status(">> Cleaning Release...");

            cmd!(shell, "{cargo} clean --release --package pidcatrs")
                .quiet()
                .run()
                .map_err(Error::new)?;
        }

        Ok(())
    };

    cargo().and_then(clean_cmd).context("failed to clean!")
}

/// Build pidcatrs
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

/// Build the Inno Setup Installer for pidcatrs
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

/// Run pidcatrs
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

fn schema_entries() -> [(&'static str, String); 2] {
    [
        ("config.schema.json", config_schema()),
        ("theme.schema.json", theme_schema()),
    ]
}

/// Write the JSON schemas for the config and theme files
fn schema_generate() -> Result<()> {
    status(">> Generating schemas...");

    let schemas_dir = PathBuf::from("schemas");
    create_dir_all(&schemas_dir).context("failed to create schemas dir!")?;

    schema_entries()
        .into_iter()
        .try_for_each(|(file_name, schema)| {
            let path = schemas_dir.join(file_name);

            write(&path, schema)
                .with_context(|| format!("failed to write {path:?}!"))
                .map(|_| println!("wrote {path:?}"))
        })
}

/// Check that the JSON schema files match the generated schemas
fn schema_check() -> Result<()> {
    status(">> Checking schemas...");

    let schemas_dir = PathBuf::from("schemas");
    let outdated = schema_entries()
        .into_iter()
        .filter(|(file_name, schema)| {
            read_to_string(schemas_dir.join(file_name)).ok().as_ref() != Some(schema)
        })
        .map(|(file_name, _)| schemas_dir.join(file_name))
        .collect::<Vec<_>>();

    match outdated.is_empty() {
        true => {
            println!("schemas are up to date");
            Ok(())
        }
        false => Err(Error::msg(
            outdated
                .iter()
                .map(|path| format!("  {path:?}"))
                .collect::<Vec<_>>()
                .join("\n"),
        ))
        .context("outdated schemas, run 'just generate-schema'!"),
    }
}

fn schema(check: bool) -> Result<()> {
    if check {
        schema_check()
    } else {
        schema_generate()
    }
}

/// The `.toml` theme files in [dir], sorted by name
fn theme_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = read_dir(dir)
        .with_context(|| format!("failed to read {dir:?}!"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("failed to read {dir:?}!"))?
        .into_iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect::<Vec<_>>();
    files.sort();

    Ok(files)
}

/// Write or check the documented bundled theme sources embedded in the binary, rendering
/// each from the bundled source itself or, when importing, from the imported theme file
fn themes(generate: bool, check: bool, import_dir: Option<PathBuf>) -> Result<()> {
    match (check, &import_dir) {
        (true, _) => status(">> Checking bundled themes..."),
        (false, Some(dir)) => status(&format!(">> Importing themes from {dir:?}...")),
        (false, None) if generate => status(">> Generating bundled themes..."),
        (false, None) => {
            return Err(Error::msg("specify --generate, --check, or --import"));
        }
    }

    let themes_dir = PathBuf::from("src/config/themes");
    let sources = theme_files(import_dir.as_deref().unwrap_or(&themes_dir))?;

    let outdated = sources
        .iter()
        .map(|source_path| {
            let file_name = source_path
                .file_name()
                .with_context(|| format!("invalid theme file name {source_path:?}!"))?;
            let path = themes_dir.join(file_name);
            let source = read_to_string(source_path)
                .with_context(|| format!("failed to read {source_path:?}!"))?;
            let rendered = render_theme_source(&source)
                .map_err(Error::msg)
                .with_context(|| format!("invalid theme file {source_path:?}!"))?;

            Ok((path, rendered))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|(path, rendered)| read_to_string(path).ok().as_ref() != Some(rendered))
        .collect::<Vec<_>>();

    match check {
        true => match outdated.is_empty() {
            true => {
                println!(
                    "{count} bundled themes are up to date",
                    count = sources.len()
                );
                Ok(())
            }
            false => Err(Error::msg(
                outdated
                    .iter()
                    .map(|(path, _)| format!("  {path:?}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ))
            .context("outdated bundled themes, run 'just generate-themes'!"),
        },

        false => outdated.iter().try_for_each(|(path, rendered)| {
            write(path, rendered)
                .with_context(|| format!("failed to write {path:?}!"))
                .map(|_| println!("wrote {path:?}"))
        }),
    }
}

/// Install pidcatrs using the Inno Setup Installer
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

/// Install pidcatrs using cargo install
#[cfg(not(target_os = "windows"))]
fn install(shell: &Shell) -> Result<()> {
    let cmd = |cargo| {
        status(">> Installing pidcatrs...");

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

        Command::Schema { check, .. } => schema(check),

        Command::Themes {
            generate,
            check,
            import_dir,
        } => themes(generate, check, import_dir),

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
