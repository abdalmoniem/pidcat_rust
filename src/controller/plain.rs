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

//! Plain (non-interactive) streaming mode.
//!
//! `run_plain` reads logcat output line by line, either from a freshly spawned `adb logcat`
//! process or from piped standard input, and renders each entry to the console and, optionally,
//! to an output file. A process-wide running flag (`set_running`, `is_running`) lets another
//! thread (for example a Ctrl-C handler) ask the loop to stop.

#![deny(clippy::unwrap_used)]

use std::io::BufRead;
use std::io::Read;
use std::io::stdin;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;

use colored::Color;
use is_terminal::IsTerminal;
use lazy_static::lazy_static;
use scope_functions::Run;

use crate::CliArgs;
use crate::ELLIPSIS;
use crate::LogSource;
use crate::ValueOrPanic;
use crate::Writer;
use crate::build_logcat_command;
use crate::colored;
use crate::controller::setup::bootstrap_adb_plain;
use crate::controller::setup::build_state;
use crate::controller::setup::normalize_cli_args;
use crate::controller::setup::resolve_packages;
use crate::flush_long_log_entry;
use crate::open_output_writer;
use crate::spawn_logcat;
use crate::trim_log_line_bytes;
use crate::write_log_line;

lazy_static! {
    /// Whether the plain-mode read loop should keep running.
    ///
    /// Set to `true` when the loop starts and cleared (through [`set_running`]) to request a
    /// graceful stop.
    static ref IS_RUNNING: AtomicBool = AtomicBool::new(false);
}

/// Returns the current terminal width in columns, or `80` when it cannot be determined (for
/// example when output is not a terminal).
fn get_console_width() -> i16 {
    terminal_size::terminal_size()
        .map(|(terminal_size::Width(width), _)| width as i16)
        .unwrap_or(80i16)
}

/// Runs plain mode: streams, filters, and prints logcat until the input ends or a stop is
/// requested.
///
/// The steps are:
///
/// 1. Normalize `args` and resolve the packages to follow (this may set `args.all`).
/// 2. Create the output writers: the console, plus a file when `args.output_path` is set.
/// 3. When standard input is a terminal, bootstrap `adb` and spawn `adb logcat`; otherwise read
///    log lines piped through standard input.
/// 4. Build the session state, print a "Capturing ..." status line, and loop reading lines,
///    rendering each through [`write_log_line`]. Console writers pick up the current terminal
///    width before every line, so resizing is honored.
/// 5. When the loop ends, flush any pending multi-line entry, and kill and reap the `adb`
///    child process if one was spawned.
///
/// The loop ends when the input reaches end of file, the child process exits, waiting on the
/// child fails, or [`set_running`] is called with `false`. In the last case a
/// "stopped by user." message is printed.
///
/// # Panics
///
/// Panics if the output file cannot be created, `adb logcat` cannot be started, reading the
/// input fails, or the child process cannot be killed or waited for.
pub fn run_plain(args: &mut CliArgs) {
    let mut adb_child = None;
    let show_colors = !args.no_color;

    normalize_cli_args(args);

    let stdin = stdin();
    let adb_command = build_logcat_command(args, None);
    let console_width = get_console_width();
    let stdout_writer = Writer::new_console(console_width, !args.no_color);
    let writers = &mut vec![stdout_writer];

    let (packages, catchall_packages, named_processes) = resolve_packages(args, None);

    if let Some(path) = args.output_path.clone() {
        writers.push(open_output_writer(&path));
    }

    if stdin.is_terminal() {
        bootstrap_adb_plain(args, show_colors);
    }

    let mut state = build_state(args, &catchall_packages, named_processes, None);

    if stdin.is_terminal() {
        adb_child =
            Some(spawn_logcat(&adb_command).unwrap_or_panic("Failed to start adb logcat process"));
    }

    let mut log_source = match adb_child {
        Some(adb_child) => LogSource::Process(adb_child),
        None => LogSource::Stdin,
    };

    let (stdout_source, stderr_source) = match log_source {
        LogSource::Process(ref mut child) => {
            let stdout = child
                .stdout
                .take()
                .map(|stdout| Box::new(stdout) as Box<dyn Read>)
                .unwrap_or_panic("Failed to capture stdout");

            let stderr = child
                .stderr
                .take()
                .map(|stderr| Box::new(stderr) as Box<dyn Read>);

            (stdout, stderr)
        }

        LogSource::Stdin => (Box::new(stdin) as Box<dyn Read>, None),
    };

    let mut stdout = std::io::BufReader::new(stdout_source);
    let mut stderr = stderr_source.map(std::io::BufReader::new);

    let msg = match !packages.is_empty() {
        true => packages
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
            .run(|packages_str| {
                format!(
                    "Capturing logcat messages from packages: [{packages_str}]{ellipsis}",
                    ellipsis = *ELLIPSIS
                )
            }),
        false => format!(
            "Capturing all logcat messages{ellipsis}",
            ellipsis = *ELLIPSIS
        ),
    }
    .run(|msg| colored(msg, show_colors, Color::BrightCyan));

    println!("{msg}");

    IS_RUNNING.store(true, Relaxed);
    while IS_RUNNING.load(Relaxed) {
        if let LogSource::Process(ref mut adb_child) = log_source {
            let exit_status = adb_child.try_wait();

            match exit_status {
                Ok(exit_status) => {
                    if let Some(status) = exit_status {
                        let msg = format!(
                            "Child process {pid} exited with status: {status}",
                            pid = adb_child.id()
                        )
                        .run(|msg| colored(msg, show_colors, Color::BrightCyan));

                        println!("{msg}");
                        break;
                    }
                }

                Err(err) => {
                    let err_msg = format!(
                        "Failed to wait for child process {pid}: {err}",
                        pid = adb_child.id()
                    )
                    .run(|msg| colored(msg, show_colors, Color::BrightRed));

                    eprintln!("{err_msg}");
                    break;
                }
            }
        }

        let stdout_buffer = &mut vec![];
        let stderr_buffer = &mut vec![];

        let stdout_bytes_read = stdout
            .read_until(b'\n', stdout_buffer)
            .unwrap_or_panic("Error reading stream");

        if stdout_bytes_read == 0usize {
            if let Some(ref mut stderr) = stderr
                && let Ok(stderr_bytes_read) = stderr.read_to_end(stderr_buffer)
                && stderr_bytes_read > 0usize
            {
                let err = trim_log_line_bytes(stderr_buffer)
                    .run(|msg| colored(msg, show_colors, Color::BrightRed));

                let err_msg = format!("Error reading stream:\n{err}")
                    .run(|msg| colored(msg, show_colors, Color::BrightRed));

                eprintln!("{err_msg}");
            }

            break;
        }

        let line = trim_log_line_bytes(stdout_buffer);

        writers
            .iter_mut()
            .filter(|writer| writer.width.is_some())
            .for_each(|writer| writer.width = Some(get_console_width()));
        write_log_line(&line, &mut state, args, writers);
    }

    flush_long_log_entry(&mut state, args, writers);

    if let LogSource::Process(mut adb_child) = log_source {
        let kill_fail_msg = format!("Failed to kill child process {pid}", pid = adb_child.id())
            .run(|msg| colored(msg, show_colors, Color::BrightRed));
        let wait_fail_msg = format!(
            "Failed to wait for child process {pid}",
            pid = adb_child.id()
        )
        .run(|msg| colored(msg, show_colors, Color::BrightRed));

        adb_child.kill().unwrap_or_panic(&kill_fail_msg);
        adb_child.wait().unwrap_or_panic(&wait_fail_msg);
    }

    if !IS_RUNNING.load(Relaxed) {
        let bin_name = option_env!("CARGO_BIN_NAME").unwrap_or("pidcatrs");
        let msg = format!("{bin_name} stopped by user.")
            .run(|msg| colored(msg, show_colors, Color::BrightCyan));

        println!("{msg}");
    }
}

/// Sets the plain-mode running flag.
///
/// Passing `false` asks a running [`run_plain`] loop to stop; the flag is checked before each
/// read, so the loop exits once the read currently in progress returns. Passing `true` marks it
/// as running. Safe to call from any thread, such as a Ctrl-C handler.
pub fn set_running(running: bool) {
    IS_RUNNING.store(running, Relaxed);
}

/// Returns the plain-mode running flag: `true` while [`run_plain`] is looping and no stop has
/// been requested.
pub fn is_running() -> bool {
    IS_RUNNING.load(Relaxed)
}
