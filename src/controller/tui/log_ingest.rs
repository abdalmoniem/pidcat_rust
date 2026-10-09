// Copyright (C) 2026 AbdAlMoniem AlHifnawy
//
// This file is part of pidcatrs.
//
// pidcatrs is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// pidcatrs is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with pidcatrs.  If not, see <https://www.gnu.org/licenses/>.
//
// Author: AbdAlMoniem AlHifnawy

//! Background log ingestion for the TUI.
//!
//! A [`LogIngest`] owns a Tokio task that reads raw log lines from one of the
//! supported sources (a live `adb logcat` process, a file, or standard
//! input), parses them into [`LogEntry`] values, evaluates the current TUI
//! filter against each entry and forwards the result to the UI as an
//! [`IngestUpdate`] over an unbounded channel.
//!
//! The task can be paused and resumed without being torn down, and it can
//! optionally mirror the entries that pass the filter to an output file
//! (`--output`).

#![deny(clippy::unwrap_used)]

use std::path::Path;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;

use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Command;
use tokio::runtime::Handle;
use tokio::sync::Notify;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::task::JoinHandle as TokioJoinHandle;

use crate::CliArgs;
use crate::LogEntry;
use crate::State;
use crate::TuiFilterSet;
use crate::ValueOrPanic;
use crate::build_logcat_command;
use crate::open_output_writer;
use crate::process_log_input;
use crate::render_entry;
use crate::trim_log_line;
use crate::trim_log_line_bytes;

use super::app::SourceMode;

/// Handle to the task that mirrors filtered entries to the `--output` file.
struct FileOutputTask {
    /// Channel on which entries to be written are sent to the writer task.
    tx: tokio_mpsc::UnboundedSender<LogEntry>,
}

impl FileOutputTask {
    /// Spawns the writer task if an output path was configured.
    ///
    /// # Arguments
    ///
    /// * `handle` - Runtime on which the writer task is spawned.
    /// * `args` - CLI arguments; `output_path` selects the destination.
    /// * `initial_state` - Render state the writer starts from.
    ///
    /// # Returns
    ///
    /// `Some` with a sender for the new task, or `None` if `args.output_path`
    /// is not set.
    fn start(handle: &Handle, args: &CliArgs, initial_state: State) -> Option<Self> {
        let path = args.output_path.as_ref()?.clone();
        let (tx, rx) = tokio_mpsc::unbounded_channel();
        let args = args.clone();

        handle.spawn(run_file_output(rx, args, initial_state, path));

        Some(Self { tx })
    }
}

/// Writer task: renders every received entry into the output file.
///
/// Runs until the sending side is dropped, then flushes the writer.
///
/// # Arguments
///
/// * `rx` - Receives the entries to write.
/// * `args` - The active CLI arguments, used for rendering.
/// * `state` - Render state, advanced as entries are rendered.
/// * `path` - Path of the output file.
async fn run_file_output(
    mut rx: tokio_mpsc::UnboundedReceiver<LogEntry>,
    args: CliArgs,
    mut state: State,
    path: String,
) {
    let mut writer = open_output_writer(&path);

    while let Some(entry) = rx.recv().await {
        render_entry(&entry, &mut state, &args, std::slice::from_mut(&mut writer));
    }

    writer.flush();
}

/// A cheaply clonable pause switch shared between the UI and the ingest task.
///
/// While paused, the ingest task blocks (without busy-waiting) before reading
/// the next line, so no new data is consumed from the source.
struct PauseGate {
    /// `true` while ingestion is paused.
    paused: Arc<AtomicBool>,
    /// Notified when ingestion is resumed so waiting tasks wake up.
    resume: Arc<Notify>,
}

impl PauseGate {
    /// Creates a gate in the running (not paused) state.
    fn new() -> Self {
        Self {
            paused: Arc::new(AtomicBool::new(false)),
            resume: Arc::new(Notify::new()),
        }
    }

    /// Pauses or resumes ingestion.
    ///
    /// Resuming wakes every task blocked in [`wait_if_paused`](Self::wait_if_paused).
    ///
    /// # Arguments
    ///
    /// * `paused` - `true` to pause, `false` to resume.
    fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Relaxed);
        if !paused {
            self.resume.notify_waiters();
        }
    }

    /// Waits until ingestion is no longer paused.
    ///
    /// Returns immediately when not paused, or when `stop` is set.
    ///
    /// # Arguments
    ///
    /// * `stop` - Stop flag; once set, the wait ends regardless of pausing.
    async fn wait_if_paused(&self, stop: &AtomicBool) {
        while self.paused.load(Relaxed) && !stop.load(Relaxed) {
            self.resume.notified().await;
        }
    }
}

/// Mutable state shared by the per-source reader loops.
struct IngestContext {
    /// The active CLI arguments, used for parsing.
    args: CliArgs,
    /// Parser state (PID/UID maps, last tag, ...), advanced for every line.
    state: State,
    /// Filter shared with the UI; read to decide whether an entry matches.
    filters: Arc<RwLock<TuiFilterSet>>,
    /// Channel to the output-file writer, when `--output` is configured.
    file_tx: Option<tokio_mpsc::UnboundedSender<LogEntry>>,
    /// Channel delivering parsed entries to the UI.
    update_tx: tokio_mpsc::UnboundedSender<IngestUpdate>,
    /// Set to request that the reader loop terminates.
    stop: Arc<AtomicBool>,
    /// Pause switch consulted before reading each line.
    pause: PauseGate,
}

impl IngestContext {
    /// Parses one raw line and forwards the resulting entries.
    ///
    /// A single line can yield zero or more entries. For each entry the shared filter is evaluated;
    /// matching entries are also sent to the output file, and every entry is
    /// sent to the UI with a snapshot of the parser state.
    ///
    /// # Arguments
    ///
    /// * `line` - The trimmed log line.
    ///
    /// # Panics
    ///
    /// Panics if the shared filter lock is poisoned.
    fn ingest_line(&mut self, line: String) {
        for entry in process_log_input(&line, &mut self.state, &self.args) {
            let matches_filter = self
                .filters
                .read()
                .unwrap_or_panic("filter lock poisoned")
                .matches(&entry, &self.state);

            if matches_filter && let Some(file_tx) = &self.file_tx {
                let _ = file_tx.send(entry.clone());
            }

            let _ = self.update_tx.send(IngestUpdate {
                entry,
                matches_filter,
                state: self.state.clone(),
            });
        }
    }

    /// Blocks while ingestion is paused (see [`PauseGate::wait_if_paused`]).
    async fn wait_if_paused(&self) {
        self.pause.wait_if_paused(&self.stop).await;
    }
}

/// Reader loop for a log file source.
///
/// Reads the file line by line until end of file, an I/O error or a stop
/// request. If the file cannot be opened the function returns immediately.
///
/// # Arguments
///
/// * `path` - Path of the log file.
/// * `ctx` - Shared ingest state.
async fn ingest_from_file(path: String, ctx: &mut IngestContext) {
    let Ok(file) = tokio::fs::File::open(Path::new(&path)).await else {
        return;
    };

    let mut lines = BufReader::new(file).lines();

    while !ctx.stop.load(Relaxed) {
        ctx.wait_if_paused().await;
        if ctx.stop.load(Relaxed) {
            break;
        }

        match lines.next_line().await {
            Ok(Some(line)) => ctx.ingest_line(trim_log_line(&line)),
            Ok(None) | Err(_) => break,
        }
    }
}

/// Reader loop for standard input (piped logs).
///
/// Reads stdin line by line until end of input, an I/O error or a stop
/// request.
///
/// # Arguments
///
/// * `ctx` - Shared ingest state.
async fn ingest_from_pipe(ctx: &mut IngestContext) {
    let mut lines = BufReader::new(tokio::io::stdin()).lines();

    while !ctx.stop.load(Relaxed) {
        ctx.wait_if_paused().await;
        if ctx.stop.load(Relaxed) {
            break;
        }

        match lines.next_line().await {
            Ok(Some(line)) => ctx.ingest_line(trim_log_line(&line)),
            Ok(None) | Err(_) => break,
        }
    }
}

/// Reader loop for a live `adb logcat` process.
///
/// Spawns logcat for `device_serial` (or the default device), reads its
/// standard output line by line and kills the child process when the loop
/// ends. Returns immediately if the process cannot be spawned.
///
/// # Arguments
///
/// * `device_serial` - Serial of the device to read from, if one is selected.
/// * `ctx` - Shared ingest state.
async fn ingest_from_live(device_serial: Option<String>, ctx: &mut IngestContext) {
    let adb_command = build_logcat_command(&ctx.args, device_serial.as_deref());
    let mut child = match Command::new(&adb_command[0usize])
        .args(&adb_command[1usize..])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return,
    };

    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => return,
    };

    let mut reader = BufReader::new(stdout);
    let mut buffer = Vec::default();

    while !ctx.stop.load(Relaxed) {
        ctx.wait_if_paused().await;
        if ctx.stop.load(Relaxed) {
            break;
        }

        buffer.clear();
        match reader.read_until(b'\n', &mut buffer).await {
            Ok(0) => break,
            Ok(_) => ctx.ingest_line(trim_log_line_bytes(&buffer)),
            Err(_) => break,
        }
    }

    let _ = child.kill().await;
}

/// Everything the spawned ingest task needs, moved into it as one value.
struct IngestTask {
    /// Which kind of source to read from.
    source: SourceMode,
    /// The active CLI arguments.
    args: CliArgs,
    /// Initial parser state.
    state: State,
    /// Device serial for [`SourceMode::Live`].
    device_serial: Option<String>,
    /// Filter shared with the UI.
    filters: Arc<RwLock<TuiFilterSet>>,
    /// Channel to the output-file writer, when configured.
    file_tx: Option<tokio_mpsc::UnboundedSender<LogEntry>>,
    /// Channel delivering parsed entries to the UI.
    update_tx: tokio_mpsc::UnboundedSender<IngestUpdate>,
    /// Stop request flag.
    stop: Arc<AtomicBool>,
    /// Pause switch.
    pause: PauseGate,
}

/// Entry point of the ingest task: dispatches to the reader for its source.
///
/// # Arguments
///
/// * `task` - The task description; consumed.
async fn run_ingest(task: IngestTask) {
    let mut ctx = IngestContext {
        args: task.args,
        state: task.state,
        filters: task.filters,
        file_tx: task.file_tx,
        update_tx: task.update_tx,
        stop: task.stop,
        pause: task.pause,
    };

    match task.source {
        SourceMode::File(path) => ingest_from_file(path, &mut ctx).await,
        SourceMode::Pipe => ingest_from_pipe(&mut ctx).await,
        SourceMode::Live => ingest_from_live(task.device_serial, &mut ctx).await,
    }
}

/// A parsed log entry delivered from the ingest task to the UI.
pub struct IngestUpdate {
    /// The parsed log entry.
    pub entry: LogEntry,
    /// Whether the entry passed the TUI filter at the time it was parsed.
    pub matches_filter: bool,
    /// Snapshot of the parser state after this entry was processed.
    pub state: State,
}

/// Controller for a running (or idle) ingest task.
///
/// Dropping a `LogIngest` does not stop the task; call [`stop`](Self::stop)
/// explicitly.
pub struct LogIngest {
    /// Stop request flag shared with the task.
    stop: Arc<AtomicBool>,
    /// Pause switch shared with the task.
    pause: PauseGate,
    /// Join handle of the ingest task, if one is running.
    task: Option<TokioJoinHandle<()>>,
    /// Output-file writer, if `--output` is configured.
    file_output: Option<FileOutputTask>,
}

impl LogIngest {
    /// Creates an inert controller that is not reading from any source.
    ///
    /// Used before the real source is known (e.g. while waiting for the user
    /// to pick a device).
    ///
    /// # Returns
    ///
    /// The idle controller and the (never producing) receiving end of its
    /// update channel.
    pub fn idle() -> (Self, tokio_mpsc::UnboundedReceiver<IngestUpdate>) {
        let (_tx, update_rx) = tokio_mpsc::unbounded_channel();
        (
            Self {
                stop: Arc::new(AtomicBool::new(true)),
                pause: PauseGate::new(),
                task: None,
                file_output: None,
            },
            update_rx,
        )
    }

    /// Pauses or resumes the ingest task.
    ///
    /// # Arguments
    ///
    /// * `paused` - `true` to stop consuming new lines, `false` to resume.
    pub fn set_paused(&self, paused: bool) {
        self.pause.set_paused(paused);
    }

    /// Stops the ingest task and the output-file writer.
    ///
    /// Sets the stop flag, aborts the task (which kills a live `adb logcat`
    /// child via `kill_on_drop`) and drops the file writer channel so the
    /// writer flushes and exits.
    pub fn stop(&mut self) {
        self.stop.store(true, Relaxed);
        if let Some(task) = self.task.take() {
            task.abort();
        }
        self.file_output = None;
    }

    /// Starts ingesting from `source` on the given runtime.
    ///
    /// # Arguments
    ///
    /// * `handle` - Runtime on which the tasks are spawned.
    /// * `source` - Where to read log lines from.
    /// * `args` - The active CLI arguments.
    /// * `state` - Initial parser state.
    /// * `device_serial` - Device to read from for [`SourceMode::Live`].
    /// * `filters` - Filter shared with the UI; read for every entry.
    ///
    /// # Returns
    ///
    /// The controller for the new task and the receiving end of the channel
    /// on which [`IngestUpdate`]s arrive.
    pub fn start(
        handle: &Handle,
        source: SourceMode,
        args: CliArgs,
        state: State,
        device_serial: Option<String>,
        filters: Arc<RwLock<TuiFilterSet>>,
    ) -> (Self, tokio_mpsc::UnboundedReceiver<IngestUpdate>) {
        let (update_tx, update_rx) = tokio_mpsc::unbounded_channel();
        let stop = Arc::new(AtomicBool::new(false));
        let pause = PauseGate::new();
        let file_output = FileOutputTask::start(handle, &args, state.clone());
        let file_tx = file_output.as_ref().map(|output| output.tx.clone());

        let task = handle.spawn(run_ingest(IngestTask {
            source,
            args,
            state,
            device_serial,
            filters,
            file_tx,
            update_tx,
            stop: Arc::clone(&stop),
            pause: pause.clone(),
        }));

        (
            Self {
                stop,
                pause,
                task: Some(task),
                file_output,
            },
            update_rx,
        )
    }
}

impl PauseGate {
    /// Creates another handle to the same pause switch.
    ///
    /// All clones share the paused flag and the resume notification.
    fn clone(&self) -> Self {
        Self {
            paused: Arc::clone(&self.paused),
            resume: Arc::clone(&self.resume),
        }
    }
}
