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
    static ref PID_LINE: Regex =
        Regex::new(r"^(\S+)\s+(\d+)\s+\S+\s+\S+\s+\S+\s+\S+\s+\S+\s+\S+\s+(.*?)$")
            .unwrap_or_panic("Invalid Regex for PID_LINE");
    static ref UID_LINE: Regex =
        Regex::new(r"^\s*userId=(\d+)\s*$").unwrap_or_panic("Invalid Regex for UID_LINE");
    static ref PACKAGE_UID_LINE: Regex = Regex::new(r"^package:(\S+)\s+uid:(\d+)\s*$")
        .unwrap_or_panic("Invalid Regex for PACKAGE_UID_LINE");
    static ref VISIBLE_ACTIVITIES: Regex = Regex::new(
        r"VisibleActivityProcess:\[\s*(?:(?:ProcessRecord\{\w+\s*\d+:(?:[a-zA-Z.]+)/\w+\})\s*)+\]"
    )
    .unwrap_or_panic("Invalid Regex for VISIBLE_ACTIVITIES");
    static ref VISIBLE_PACKAGES: Regex = Regex::new(r"ProcessRecord\{\w+\s*\d+:([a-zA-Z.]+)/\w+\}")
        .unwrap_or_panic("Invalid Regex for VISIBLE_PACKAGES");
}

/// Plain-mode no-device error lines (preserve exact casing in TUI and CLI).
pub const NO_ADB_DEVICES_ERROR_HEADER: &str = "ERROR: not connected";
pub const NO_ADB_DEVICES_ERROR_MESSAGE: &str =
    "ADB cannot find any attached devices, attach a device and try again!";

pub fn get_adb_command(args: &CliArgs) -> Vec<String> {
    build_adb_command(args, None)
}

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

pub fn start_adb_server(base_adb_command: &[String]) -> Result<Output, Error> {
    Command::new(&base_adb_command[0usize])
        .args(&base_adb_command[1usize..])
        .arg("start-server")
        .output()
}

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

pub fn clear_logcat(base_adb_command: &[String]) -> Result<Output, Error> {
    let mut clear_cmd = base_adb_command.to_vec();
    clear_cmd.push("logcat".to_string());
    clear_cmd.push("-c".to_string());

    Command::new(&clear_cmd[0usize])
        .args(&clear_cmd[1usize..])
        .output()
}

pub fn spawn_logcat(adb_command: &[String]) -> Result<Child, Error> {
    Command::new(&adb_command[0usize])
        .args(&adb_command[1usize..])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}
