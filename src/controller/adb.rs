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

//! Running and parsing `adb` commands.
//!
//! Every function here builds on a *base command*: the `adb` executable plus the global options
//! that select a device (`-d`, `-e`, or `-s <serial>`), produced by `build_adb_command`. The
//! helpers cover:
//!
//! - assembling commands (`get_adb_command`, `build_adb_command`, `build_logcat_command`);
//! - managing the server and devices (`start_adb_server`, `get_adb_devices`,
//!   `resolve_initial_device`);
//! - inspecting the device (`get_current_app_package`, `get_processes`);
//! - controlling `logcat` (`clear_logcat`, `spawn_logcat`).
//!
//! All functions that take a base command index into it directly, so it must contain at least the
//! executable name.

#![deny(clippy::unwrap_used)]

use std::collections::HashMap;
use std::collections::HashSet;
use std::io::BufRead;
use std::io::Error;
use std::process::Child;
use std::process::Command;
use std::process::Output;
use std::process::Stdio;

use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;

use crate::AdbDevice;
use crate::AdbState;
use crate::CliArgs;
use crate::ValueOrPanic;

lazy_static! {
    /// Matches one row of `adb shell ps` output.
    ///
    /// Capture 1 is the user, capture 2 the PID, and capture 3 the process name (the last
    /// column). The columns in between (PPID, VSZ, RSS, WCHAN, ADDR, and state) are skipped.
    static ref PID_LINE: Regex =
        Regex::new(r"^(\S+)\s+(\d+)\s+\S+\s+\S+\s+\S+\s+\S+\s+\S+\s+\S+\s+(.*?)$")
            .unwrap_or_panic("Invalid Regex for PID_LINE");
    /// Matches the `userId=<uid>` line of `dumpsys package <name>` output; capture 1 is the UID.
    static ref UID_LINE: Regex =
        Regex::new(r"^\s*userId=(\d+)\s*$").unwrap_or_panic("Invalid Regex for UID_LINE");
    /// Matches one row of `cmd package list packages -U` output (`package:<name> uid:<uid>`);
    /// capture 1 is the package name and capture 2 the UID.
    static ref PACKAGE_UID_LINE: Regex = Regex::new(r"^package:(\S+)\s+uid:(\d+)\s*$")
        .unwrap_or_panic("Invalid Regex for PACKAGE_UID_LINE");
    /// Matches the `VisibleActivityProcess:[ ... ]` entry of `dumpsys activity activities` output
    /// that lists the process records of the currently visible activities.
    static ref VISIBLE_ACTIVITIES: Regex = Regex::new(
        r"VisibleActivityProcess:\[\s*(?:(?:ProcessRecord\{\w+\s*\d+:(?:[a-zA-Z.]+)/\w+\})\s*)+\]"
    )
    .unwrap_or_panic("Invalid Regex for VISIBLE_ACTIVITIES");
    /// Matches a single `ProcessRecord{<hash> <pid>:<package>/<user>}` item inside
    /// [`struct@VISIBLE_ACTIVITIES`]; capture 1 is the package name.
    static ref VISIBLE_PACKAGES: Regex = Regex::new(r"ProcessRecord\{\w+\s*\d+:([a-zA-Z.]+)/\w+\}")
        .unwrap_or_panic("Invalid Regex for VISIBLE_PACKAGES");
}

/// Plain-mode no-device error lines (preserve exact casing in TUI and CLI).
///
/// This is the header line, printed before [`NO_ADB_DEVICES_ERROR_MESSAGE`].
pub const NO_ADB_DEVICES_ERROR_HEADER: &str = "ERROR: not connected";
/// Explanatory line shown after [`NO_ADB_DEVICES_ERROR_HEADER`] when `adb` reports no attached
/// devices.
pub const NO_ADB_DEVICES_ERROR_MESSAGE: &str =
    "ADB cannot find any attached devices, attach a device and try again!";

/// Builds the base `adb` command for `args` without an explicit device serial override.
///
/// Equivalent to [`build_adb_command`] with `device_serial` set to `None`.
pub fn get_adb_command(args: &CliArgs) -> Vec<String> {
    build_adb_command(args, None)
}

/// Builds the full `adb logcat` command line for `args`.
///
/// Starts from [`build_adb_command`] (honoring `device_serial`), then appends `logcat`, `-v` with
/// the configured log format, and, when a filter regex is configured, `-e <regex>`.
pub fn build_logcat_command(args: &CliArgs, device_serial: Option<&str>) -> Vec<String> {
    let mut adb_command = build_adb_command(args, device_serial);
    adb_command.extend([
        "logcat".to_string(),
        "-v".to_string(),
        args.log_format.adb_verb(),
    ]);

    if let Some(regex) = &args.regex {
        adb_command.extend(["-e".to_string(), regex.clone()]);
    }

    adb_command
}

/// Builds the base `adb` command: the executable plus the device-selection option.
///
/// The executable is `args.adb_path` or `adb` when unset. Device selection is, in priority
/// order: `-d` when `args.use_device` is set, `-e` when `args.use_emulator` is set, otherwise
/// `-s <serial>` using `device_serial` (preferred) or `args.device_serial`. If none apply, no
/// selection option is added.
pub fn build_adb_command(args: &CliArgs, device_serial: Option<&str>) -> Vec<String> {
    let adb_path = args.adb_path.clone().unwrap_or_else(|| "adb".to_string());
    let mut base_adb_command = vec![adb_path];

    if args.use_device {
        base_adb_command.push("-d".to_string());
    } else if args.use_emulator {
        base_adb_command.push("-e".to_string());
    } else if let Some(serial) = device_serial.or(args.device_serial.as_deref()) {
        base_adb_command.push("-s".to_string());
        base_adb_command.push(serial.to_string());
    }

    base_adb_command
}

/// Runs `adb start-server` and returns its captured output.
///
/// # Errors
///
/// Returns an I/O error if the `adb` executable cannot be started (for example when it is not on
/// `PATH`). A non-zero exit status of `adb` itself is not an error; inspect the returned
/// [`Output`].
pub fn start_adb_server(base_adb_command: &[String]) -> Result<Output, Error> {
    Command::new(&base_adb_command[0usize])
        .args(&base_adb_command[1usize..])
        .arg("start-server")
        .output()
}

/// Lists the devices known to `adb` by running `adb devices`.
///
/// The header line of the output is skipped and each remaining non-empty line is parsed as
/// `<serial> <state>`. Unless `quiet` is `true`, every device line is also printed to standard
/// output as it is read.
///
/// Returns `None` when the command cannot be run or when no devices are listed.
///
/// # Panics
///
/// Panics if a device line does not consist of exactly two whitespace-separated fields.
pub fn get_adb_devices(base_adb_command: &[String], quiet: bool) -> Option<Vec<AdbDevice>> {
    let output = Command::new(&base_adb_command[0usize])
        .args(&base_adb_command[1usize..])
        .arg("devices")
        .output();

    match output {
        Ok(output) => {
            let re = Regex::new(r"\s+").unwrap_or_panic("Invalid Regex");
            let devices = output
                .stdout
                .split(|&byte| byte == b'\n')
                .skip(1usize)
                .map(|line| String::from_utf8_lossy(line).trim().to_string())
                .filter(|line| !line.is_empty())
                .map(|device| {
                    if !quiet {
                        println!("{device}");
                    }

                    let (device_id_str, device_state_str) = re
                        .split(&device)
                        .map(|str| str.to_string())
                        .collect_tuple::<(String, String)>()
                        .unwrap_or_panic("Failed to get device id and type");

                    AdbDevice {
                        device_id: device_id_str,
                        device_state: AdbState::from(device_state_str),
                    }
                })
                .collect::<Vec<_>>();

            if !devices.is_empty() {
                Some(devices)
            } else {
                None
            }
        }

        Err(_) => None,
    }
}

/// Picks the device serial to use at start-up, or `None` when the choice is ambiguous.
///
/// Only devices in the ready states (`Device` or `Emulator`) are candidates. The result is, in
/// priority order:
///
/// 1. the explicit `args.device_serial`, returned as is;
/// 2. with `args.use_device`, the first ready device that is not an emulator;
/// 3. with `args.use_emulator`, the first ready device whose id starts with `emulator-`;
/// 4. the only ready device, when exactly one exists;
/// 5. otherwise `None`, meaning the user has to choose.
pub fn resolve_initial_device(args: &CliArgs, devices: &[AdbDevice]) -> Option<String> {
    if let Some(serial) = &args.device_serial {
        return Some(serial.clone());
    }

    let ready = devices
        .iter()
        .filter(|device| matches!(device.device_state, AdbState::Device | AdbState::Emulator))
        .collect::<Vec<_>>();

    if args.use_device {
        return ready
            .iter()
            .find(|device| !device.device_id.starts_with("emulator-"))
            .map(|device| device.device_id.clone());
    }

    if args.use_emulator {
        return ready
            .iter()
            .find(|device| device.device_id.starts_with("emulator-"))
            .map(|device| device.device_id.clone());
    }

    if ready.len() == 1usize {
        return Some(ready[0usize].device_id.clone());
    }

    None
}

/// Detects the packages of the activities currently visible on the device.
///
/// Runs `adb shell dumpsys activity activities` and extracts the packages from its
/// `VisibleActivityProcess` entry.
///
/// Returns `None` when the command fails, the entry is not present, or it contains no packages.
pub fn get_current_app_package(base_adb_command: &[String]) -> Option<Vec<String>> {
    let mut cmd = Command::new(&base_adb_command[0usize]);
    if base_adb_command.len() > 1usize {
        cmd.args(&base_adb_command[1usize..]);
    }

    let output = cmd
        .args(["shell", "dumpsys", "activity", "activities"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .ok()?;

    let system_dump = String::from_utf8_lossy(&output.stdout);

    let visible_activities = VISIBLE_ACTIVITIES.find(&system_dump)?.as_str();

    let packages: Vec<String> = VISIBLE_PACKAGES
        .captures_iter(visible_activities)
        .filter_map(|cap| cap.get(1usize).map(|mat| mat.as_str().to_string()))
        .collect();

    if packages.is_empty() {
        None
    } else {
        Some(packages)
    }
}

/// Reads the device's running processes and package UIDs.
///
/// Returns `(pids_map, uids_map)`:
///
/// - `pids_map` maps a PID to its process name. It contains every running process when
///   `args.all` is set, otherwise only processes whose name is listed in `catchall_package`.
/// - `uids_map` maps a UID to a process or package name. For each matching process (looked up
///   once per distinct name) the UID is read from `dumpsys package <package>`, where the package
///   is the process name without any `:suffix`. When `args.all` is set, the UIDs of all packages
///   of user 0 from `cmd package list packages -U` are added as well, overriding earlier entries.
///
/// Commands that fail to run are ignored and simply contribute no entries.
pub fn get_processes(
    base_adb_command: &[String],
    catchall_package: &[String],
    args: &CliArgs,
) -> (HashMap<String, String>, HashMap<String, String>) {
    let mut pids_map = HashMap::default();
    let mut uids_map = HashMap::default();
    let mut packages: HashSet<String> = HashSet::default();
    let mut cmd = Command::new(&base_adb_command[0usize]);

    if base_adb_command.len() > 1usize {
        cmd.args(&base_adb_command[1usize..]);
    }

    let output = cmd.args(["shell", "ps"]).stdout(Stdio::piped()).output();

    if let Ok(out) = output {
        let stdout = std::io::BufReader::new(&out.stdout[..]);
        for line in stdout.lines().map_while(Result::ok) {
            if let Some(caps) = PID_LINE.captures(&line) {
                let pid = caps
                    .get(2usize)
                    .map_or(String::default(), |mat| mat.as_str().to_string());
                let process = caps
                    .get(3usize)
                    .map_or(String::default(), |mat| mat.as_str().to_string());

                let is_target_package = catchall_package.contains(&process);

                if args.all || is_target_package {
                    pids_map.insert(pid, process.clone());
                }

                if is_target_package && packages.insert(process.clone()) {
                    let package = process.split(':').next().unwrap_or(&process);
                    let mut cmd = Command::new(&base_adb_command[0usize]);

                    if base_adb_command.len() > 1usize {
                        cmd.args(&base_adb_command[1usize..]);
                    }

                    let output = cmd
                        .args(["shell", "dumpsys", "package", package])
                        .stdout(Stdio::piped())
                        .output();

                    if let Ok(out) = output {
                        let stdout = std::io::BufReader::new(&out.stdout[..]);
                        for line in stdout.lines().map_while(Result::ok) {
                            if let Some(caps) = UID_LINE.captures(&line) {
                                let uid = caps
                                    .get(1usize)
                                    .map_or(String::default(), |mat| mat.as_str().to_string());
                                uids_map.insert(uid, process.clone());
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    if args.all {
        let mut cmd = Command::new(&base_adb_command[0usize]);

        if base_adb_command.len() > 1usize {
            cmd.args(&base_adb_command[1usize..]);
        }

        let output = cmd
            .args([
                "shell", "cmd", "package", "list", "packages", "--user", "0", "-U",
            ])
            .stdout(Stdio::piped())
            .output();

        if let Ok(out) = output {
            let stdout = std::io::BufReader::new(&out.stdout[..]);
            for line in stdout.lines().map_while(Result::ok) {
                if let Some(caps) = PACKAGE_UID_LINE.captures(&line) {
                    let package = caps
                        .get(1usize)
                        .map_or(String::default(), |mat| mat.as_str().to_string());
                    let uid = caps
                        .get(2usize)
                        .map_or(String::default(), |mat| mat.as_str().to_string());
                    uids_map.insert(uid, package);
                }
            }
        }
    }

    (pids_map, uids_map)
}

/// Clears the device's logcat buffers by running `adb logcat -c`.
///
/// # Errors
///
/// Returns an I/O error if the `adb` executable cannot be started. A non-zero exit status of
/// `adb` itself is not an error; inspect the returned [`Output`].
pub fn clear_logcat(base_adb_command: &[String]) -> Result<Output, Error> {
    let mut clear_cmd = base_adb_command.to_vec();
    clear_cmd.push("logcat".to_string());
    clear_cmd.push("-c".to_string());

    Command::new(&clear_cmd[0usize])
        .args(&clear_cmd[1usize..])
        .output()
}

/// Spawns `adb_command` (typically from [`build_logcat_command`]) as a child process with piped
/// standard output and standard error.
///
/// The caller owns the returned [`Child`] and is responsible for reading its pipes and for
/// waiting on or killing it.
///
/// # Errors
///
/// Returns an I/O error if the process cannot be started.
pub fn spawn_logcat(adb_command: &[String]) -> Result<Child, Error> {
    Command::new(&adb_command[0usize])
        .args(&adb_command[1usize..])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}
