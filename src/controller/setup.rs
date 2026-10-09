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

pub fn current_app_packages(args: &CliArgs, device_serial: Option<&str>) -> Vec<String> {
    if !args.current_app {
        return Vec::default();
    }

    get_current_app_package(&build_adb_command(args, device_serial)).unwrap_or_default()
}

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

pub fn bootstrap_adb_tui(args: &CliArgs) -> Vec<AdbDevice> {
    let base = build_adb_command(args, None);
    let _ = start_adb_server(&base);

    get_adb_devices(&base, true).unwrap_or_default()
}

pub fn maybe_clear_logcat(args: &CliArgs, device_serial: Option<&str>) {
    if !args.keep_logcat && device_serial.is_some() {
        let _ = clear_logcat(&build_adb_command(args, device_serial));
    }
}
