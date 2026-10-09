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

//! Start-up helpers shared by plain mode and the TUI.
//!
//! These functions prepare a run before any log line is processed:
//!
//! - normalizing parsed CLI arguments (`normalize_cli_args`);
//! - resolving which packages and processes to follow (`current_app_packages`,
//!   `resolve_packages`);
//! - creating and refreshing the session `State` (`build_state`, `refresh_process_maps`);
//! - producing the initial TUI filter text (`seed_filter_input`);
//! - bringing up `adb` and the device list (`bootstrap_adb_plain`, `bootstrap_adb_tui`,
//!   `maybe_clear_logcat`).

#![deny(clippy::unwrap_used)]

use std::collections::HashSet;
use std::io::Error;
use std::io::ErrorKind;
use std::process;

use colored::Color;
use itertools::Itertools;
use scope_functions::Run;

use crate::AdbDevice;
use crate::CliArgs;
use crate::ELLIPSIS;
use crate::LogLevel;
use crate::SYSTEM_TAGS;
use crate::State;
use crate::ValueOrPanic;
use crate::active_theme;
use crate::build_adb_command;
use crate::clear_logcat;
use crate::get_adb_devices;
use crate::get_current_app_package;
use crate::get_processes;
use crate::start_adb_server;

use super::adb::NO_ADB_DEVICES_ERROR_HEADER;
use super::adb::NO_ADB_DEVICES_ERROR_MESSAGE;
use super::util::colored;
use super::util::split_csv_args;

/// Normalizes tag-related CLI arguments in place.
///
/// - When `args.ignore_system_tags` is set, every entry of [`SYSTEM_TAGS`] is added to
///   `args.ignore_tag` as an anchored pattern (`^tag$`), after any tags already present.
/// - The `ignore_tag` and `tag` lists are then expanded so that comma-separated values inside a
///   single argument become separate entries (see
///   [`split_csv_args`]).
pub fn normalize_cli_args(args: &mut CliArgs) {
    if args.ignore_system_tags {
        let mut system_tags: Vec<String> =
            SYSTEM_TAGS.iter().map(|tag| format!("^{tag}$")).collect();
        args.ignore_tag = match args.ignore_tag.as_mut() {
            Some(existing) => {
                existing.append(&mut system_tags);
                Some(existing.to_vec())
            }
            None => Some(system_tags),
        };
    }

    if let Some(ignore_tags) = args.ignore_tag.clone() {
        args.ignore_tag = Some(split_csv_args(&ignore_tags));
    }

    if let Some(tags) = args.tag.clone() {
        args.tag = Some(split_csv_args(&tags));
    }
}

/// Returns the packages of the foreground (visible) apps when `--current-app` is enabled.
///
/// Queries the device selected by `device_serial` (or the one configured in `args`). Returns an
/// empty list when the option is disabled, or when no visible package could be determined.
pub fn current_app_packages(args: &CliArgs, device_serial: Option<&str>) -> Vec<String> {
    if !args.current_app {
        return Vec::default();
    }

    get_current_app_package(&build_adb_command(args, device_serial)).unwrap_or_default()
}

/// Determines which packages and processes to follow.
///
/// The set of packages is the configured `args.packages` plus the foreground apps from
/// [`current_app_packages`]. Entries are then classified into:
///
/// - *catch-all packages*: entries without a `:`; they match the package and all of its
///   `package:process` children;
/// - *named processes*: entries containing a `:`, with a single trailing `:` removed.
///
/// If no package ended up selected, `args.all` is switched on so every process is captured.
///
/// Returns `(packages, catchall_packages, named_processes)`.
pub fn resolve_packages(
    args: &mut CliArgs,
    device_serial: Option<&str>,
) -> (HashSet<String>, Vec<String>, Vec<String>) {
    let mut packages: HashSet<String> = args
        .packages
        .iter()
        .map(|package| package.to_string())
        .collect();

    packages.extend(current_app_packages(args, device_serial));

    let catchall_packages = packages
        .iter()
        .filter(|package| !package.contains(':'))
        .cloned()
        .collect::<Vec<_>>();

    let named_processes = packages
        .iter()
        .filter(|package| package.contains(':'))
        .map(|package| package.strip_suffix(':').unwrap_or(package).to_string())
        .collect::<Vec<_>>();

    if packages.is_empty() {
        args.all = true;
    }

    (packages, catchall_packages, named_processes)
}

/// Creates the initial session [`State`].
///
/// Reads the device's current PID and UID maps through [`get_processes`] for the device
/// selected by `device_serial`, takes the rotating token color palette from the active theme,
/// and starts with no known tokens, no last tag, no app PID, and no pending multi-line entry.
/// The minimum log level comes from `args.log_level`.
pub fn build_state(
    args: &CliArgs,
    catchall_packages: &[String],
    named_processes: Vec<String>,
    device_serial: Option<&str>,
) -> State {
    let base_adb_command = build_adb_command(args, device_serial);
    let (pids_map, uids_map) = get_processes(&base_adb_command, catchall_packages, args);

    let token_colors = active_theme()
        .log
        .tokens
        .iter()
        .map(|&token| Color::from(token))
        .collect();

    State {
        pids_map,
        uids_map,
        last_tag: None,
        app_pid: None,
        log_level: args.log_level,
        named_processes,
        catchall_packages: catchall_packages.to_vec(),
        token_colors,
        known_tokens: std::collections::HashMap::default(),
        long_pending: None,
    }
}

/// Builds the text the TUI filter input starts with, mirroring the CLI selection.
///
/// The parts, separated by single spaces, are in order:
///
/// 1. `package:<name>` for each catch-all package (entries without `:`), sorted alphabetically;
/// 2. `tag:<tag>` for each configured tag;
/// 3. the configured filter regex, verbatim;
/// 4. `level:<name>` when the log level is anything other than verbose.
pub fn seed_filter_input(args: &CliArgs, packages: &HashSet<String>) -> String {
    let mut parts = Vec::default();

    let mut packages: Vec<&String> = packages
        .iter()
        .filter(|package| !package.contains(':'))
        .collect();
    packages.sort();
    for package in packages {
        parts.push(format!("package:{package}"));
    }

    if let Some(tags) = &args.tag {
        for tag in tags {
            parts.push(format!("tag:{tag}"));
        }
    }

    if let Some(regex) = &args.regex {
        parts.push(regex.clone());
    }

    if args.log_level != LogLevel::VERBOSE {
        parts.push(format!("level:{}", args.log_level.filter_name()));
    }

    parts.join(" ")
}

/// Re-reads the device's PID and UID maps and stores them in `state`.
///
/// Used to pick up processes that started before capture began or while capture was running. The
/// maps are replaced wholesale; other parts of `state` are left untouched.
pub fn refresh_process_maps(
    state: &mut State,
    args: &CliArgs,
    catchall_packages: &[String],
    device_serial: Option<&str>,
) {
    let base = build_adb_command(args, device_serial);
    let (pids_map, uids_map) = get_processes(&base, catchall_packages, args);
    state.pids_map = pids_map;
    state.uids_map = uids_map;
}

/// Prepares `adb` for plain mode, reporting progress on the console.
///
/// In order, this starts the ADB server, lists the attached devices, and, unless
/// `args.keep_logcat` is set, clears the logcat buffers. Status lines are printed in bright cyan
/// (when `show_colors` is `true`).
///
/// # Exits
///
/// Terminates the process with an error message when the ADB server cannot be started (using the
/// OS error code, or `1` if there is none) or when no device is attached (see
/// [`NO_ADB_DEVICES_ERROR_HEADER`] and [`NO_ADB_DEVICES_ERROR_MESSAGE`]).
///
/// # Panics
///
/// Panics if the logcat buffers cannot be cleared.
pub fn bootstrap_adb_plain(args: &CliArgs, show_colors: bool) {
    let base_adb_command = build_adb_command(args, None);

    let msg = "Starting ADB server...".run(|msg| colored(msg, show_colors, Color::BrightCyan));
    println!("{msg}");

    match start_adb_server(&base_adb_command) {
        Ok(output) => match !output.stdout.is_empty() {
            true => output.stdout,
            false => output.stderr,
        }
        .split(|&byte| byte == b'\n')
        .map(|line| String::from_utf8_lossy(line).trim().to_string())
        .take_while(|line| !line.is_empty())
        .join("\n")
        .run(|msg| colored(msg, show_colors, Color::BrightCyan))
        .run(|output| {
            if !output.is_empty() {
                println!("{output}");
            }
        }),

        Err(err) => err.run(|err| {
            let err_code = err.raw_os_error().unwrap_or(1i32);
            let err_hdr =
                format!("ERROR: {err}").run(|msg| colored(msg, show_colors, Color::BrightRed));
            let err_msg =
                "Could not start ADB server, check that ADB is added to env PATH and try again!"
                    .run(|msg| colored(msg, show_colors, Color::BrightRed));

            eprintln!("{err_hdr}");
            eprintln!("{err_msg}");
            process::exit(err_code);
        }),
    }

    match get_adb_devices(&base_adb_command, false) {
        Some(devices) => {
            for (index, device) in devices.iter().enumerate() {
                let msg = format!("Found Device #{index}: {device:?}")
                    .run(|msg| colored(msg, show_colors, Color::BrightCyan));

                println!("{msg}");
            }
        }

        None => {
            let err = Error::from(ErrorKind::NotConnected);
            let err_code = err.raw_os_error().unwrap_or(1i32);
            let err_hdr = colored(NO_ADB_DEVICES_ERROR_HEADER, show_colors, Color::BrightRed);
            let err_msg = colored(NO_ADB_DEVICES_ERROR_MESSAGE, show_colors, Color::BrightRed);
            eprintln!("{err_hdr}");
            eprintln!("{err_msg}");
            process::exit(err_code);
        }
    }

    if !args.keep_logcat {
        let msg = format!("Clearing logcat{ellipsis}", ellipsis = *ELLIPSIS)
            .run(|msg| colored(msg, show_colors, Color::BrightCyan));

        println!("{msg}");

        clear_logcat(&base_adb_command).unwrap_or_panic("Could not clear logcat");
    }
}

/// Prepares `adb` for the TUI and returns the attached devices.
///
/// Starts the ADB server (ignoring any failure) and lists the devices without printing
/// anything. Returns an empty list when no devices are found, leaving the decision of how to
/// present that to the TUI.
pub fn bootstrap_adb_tui(args: &CliArgs) -> Vec<AdbDevice> {
    let base = build_adb_command(args, None);
    let _ = start_adb_server(&base);

    get_adb_devices(&base, true).unwrap_or_default()
}

/// Clears the logcat buffers of `device_serial`, unless `args.keep_logcat` is set.
///
/// Does nothing when no device is selected. Failures are ignored.
pub fn maybe_clear_logcat(args: &CliArgs, device_serial: Option<&str>) {
    if !args.keep_logcat && device_serial.is_some() {
        let _ = clear_logcat(&build_adb_command(args, device_serial));
    }
}
