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

struct FileOutputTask {
    tx: tokio_mpsc::UnboundedSender<LogEntry>,
}

impl FileOutputTask {
    fn start(handle: &Handle, args: &CliArgs, initial_state: State) -> Option<Self> {
        let path = args.output_path.as_ref()?.clone();
        let (tx, rx) = tokio_mpsc::unbounded_channel();
        let args = args.clone();

        handle.spawn(run_file_output(rx, args, initial_state, path));

        Some(Self { tx })
    }
}

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

struct PauseGate {
    paused: Arc<AtomicBool>,
    resume: Arc<Notify>,
}

impl PauseGate {
    fn new() -> Self {
        Self {
            paused: Arc::new(AtomicBool::new(false)),
            resume: Arc::new(Notify::new()),
        }
    }

    fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Relaxed);
        if !paused {
            self.resume.notify_waiters();
        }
    }

    async fn wait_if_paused(&self, stop: &AtomicBool) {
        while self.paused.load(Relaxed) && !stop.load(Relaxed) {
            self.resume.notified().await;
        }
    }
}

struct IngestContext {
    args: CliArgs,
    state: State,
    filters: Arc<RwLock<TuiFilterSet>>,
    file_tx: Option<tokio_mpsc::UnboundedSender<LogEntry>>,
    update_tx: tokio_mpsc::UnboundedSender<IngestUpdate>,
    stop: Arc<AtomicBool>,
    pause: PauseGate,
}

impl IngestContext {
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

    async fn wait_if_paused(&self) {
        self.pause.wait_if_paused(&self.stop).await;
    }
}

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

struct IngestTask {
    source: SourceMode,
    args: CliArgs,
    state: State,
    device_serial: Option<String>,
    filters: Arc<RwLock<TuiFilterSet>>,
    file_tx: Option<tokio_mpsc::UnboundedSender<LogEntry>>,
    update_tx: tokio_mpsc::UnboundedSender<IngestUpdate>,
    stop: Arc<AtomicBool>,
    pause: PauseGate,
}

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

pub struct IngestUpdate {
    pub entry: LogEntry,
    pub matches_filter: bool,
    pub state: State,
}

pub struct LogIngest {
    stop: Arc<AtomicBool>,
    pause: PauseGate,
    task: Option<TokioJoinHandle<()>>,
    file_output: Option<FileOutputTask>,
}

impl LogIngest {
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

    pub fn set_paused(&self, paused: bool) {
        self.pause.set_paused(paused);
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Relaxed);
        if let Some(task) = self.task.take() {
            task.abort();
        }
        self.file_output = None;
    }

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
    fn clone(&self) -> Self {
        Self {
            paused: Arc::clone(&self.paused),
            resume: Arc::clone(&self.resume),
        }
    }
}
