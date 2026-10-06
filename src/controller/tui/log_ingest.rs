#![deny(clippy::unwrap_used)]

use std::path::Path;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Command;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::task::JoinHandle as TokioJoinHandle;

use crate::CliArgs;
use crate::LogEntry;
use crate::State;
use crate::TuiFilterSet;
use crate::ValueOrPanic;
use crate::build_logcat_command;
use crate::open_output_writer;
use crate::process_line;
use crate::render_entry;
use crate::trim_log_line;
use crate::trim_log_line_bytes;

use super::app::SourceMode;

const INGEST_BATCH_SIZE: usize = 128;
const PAUSE_POLL: Duration = Duration::from_millis(50);

struct FileOutputThread {
    tx: mpsc::Sender<LogEntry>,
    join: Option<JoinHandle<()>>,
}

impl FileOutputThread {
    fn start(args: CliArgs, initial_state: State) -> Option<Self> {
        let path = args.output_path.as_ref()?;
        let (tx, rx) = mpsc::channel();
        let path = path.clone();
        let join = thread::Builder::new()
            .name("pidcat-file-output".into())
            .spawn(move || {
                let mut writer = open_output_writer(&path);
                let mut state = initial_state;
                while let Ok(entry) = rx.recv() {
                    render_entry(&entry, &mut state, &args, std::slice::from_mut(&mut writer));
                }
                writer.flush();
            })
            .ok()?;

        Some(Self {
            tx,
            join: Some(join),
        })
    }
}

impl Drop for FileOutputThread {
    fn drop(&mut self) {
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn send_file_entry(tx: &mpsc::Sender<LogEntry>, entry: &LogEntry) {
    let _ = tx.send(entry.clone());
}

async fn wait_while_paused(paused: &AtomicBool, stop: &AtomicBool) {
    while paused.load(Relaxed) && !stop.load(Relaxed) {
        tokio::time::sleep(PAUSE_POLL).await;
    }
}

fn flush_batch(
    batch: &mut Vec<IngestItem>,
    state: &State,
    tx: &tokio_mpsc::UnboundedSender<IngestBatch>,
) {
    if batch.is_empty() {
        return;
    }
    let _ = tx.send(IngestBatch {
        items: std::mem::take(batch),
        state: state.clone(),
    });
}

fn ingest_line(
    line: String,
    state: &mut State,
    args: &CliArgs,
    filters: &Arc<RwLock<TuiFilterSet>>,
    file_tx: &Option<mpsc::Sender<LogEntry>>,
    batch: &mut Vec<IngestItem>,
    tx: &tokio_mpsc::UnboundedSender<IngestBatch>,
) {
    let Some(entry) = process_line(&line, state, args) else {
        return;
    };

    let matches_filter = filters
        .read()
        .unwrap_or_panic("filter lock poisoned")
        .matches(&entry, state);

    if matches_filter && let Some(file_tx) = file_tx {
        send_file_entry(file_tx, &entry);
    }

    batch.push(IngestItem {
        entry,
        matches_filter,
    });

    if batch.len() >= INGEST_BATCH_SIZE {
        flush_batch(batch, state, tx);
    }
}

#[derive(Clone)]
pub struct IngestItem {
    pub entry: LogEntry,
    pub matches_filter: bool,
}

pub struct IngestBatch {
    pub items: Vec<IngestItem>,
    pub state: State,
}

pub struct LogIngest {
    batch_rx: tokio_mpsc::UnboundedReceiver<IngestBatch>,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    task: Option<TokioJoinHandle<()>>,
    file_output: Option<FileOutputThread>,
}

impl LogIngest {
    pub fn idle() -> Self {
        let (_tx, batch_rx) = tokio_mpsc::unbounded_channel();
        Self {
            batch_rx,
            stop: Arc::new(AtomicBool::new(true)),
            paused: Arc::new(AtomicBool::new(false)),
            task: None,
            file_output: None,
        }
    }

    pub fn paused_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.paused)
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Relaxed);
        if let Some(task) = self.task.take() {
            task.abort();
        }
        self.file_output = None;
    }

    pub fn try_recv_batch(&mut self) -> Option<IngestBatch> {
        self.batch_rx.try_recv().ok()
    }

    pub fn start(
        handle: &tokio::runtime::Handle,
        source: SourceMode,
        args: CliArgs,
        mut state: State,
        device_serial: Option<String>,
        filters: Arc<RwLock<TuiFilterSet>>,
    ) -> Self {
        let (batch_tx, batch_rx) = tokio_mpsc::unbounded_channel();
        let stop = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));
        let stop_task = Arc::clone(&stop);
        let paused_task = Arc::clone(&paused);
        let file_output = FileOutputThread::start(args.clone(), state.clone());
        let file_tx = file_output.as_ref().map(|output| output.tx.clone());

        let task = handle.spawn(async move {
            let mut batch = Vec::with_capacity(INGEST_BATCH_SIZE);

            match source {
                SourceMode::File(path) => {
                    let Ok(file) = tokio::fs::File::open(Path::new(&path)).await else {
                        return;
                    };
                    let mut lines = BufReader::new(file).lines();
                    while !stop_task.load(Relaxed) {
                        wait_while_paused(&paused_task, &stop_task).await;
                        if stop_task.load(Relaxed) {
                            break;
                        }
                        match lines.next_line().await {
                            Ok(Some(line)) => ingest_line(
                                trim_log_line(&line),
                                &mut state,
                                &args,
                                &filters,
                                &file_tx,
                                &mut batch,
                                &batch_tx,
                            ),
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }
                }
                SourceMode::Pipe => {
                    let stdin = tokio::io::stdin();
                    let mut lines = BufReader::new(stdin).lines();
                    while !stop_task.load(Relaxed) {
                        wait_while_paused(&paused_task, &stop_task).await;
                        if stop_task.load(Relaxed) {
                            break;
                        }
                        match lines.next_line().await {
                            Ok(Some(line)) => ingest_line(
                                trim_log_line(&line),
                                &mut state,
                                &args,
                                &filters,
                                &file_tx,
                                &mut batch,
                                &batch_tx,
                            ),
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }
                }
                SourceMode::Live => {
                    let adb_command = build_logcat_command(&args, device_serial.as_deref());
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

                    while !stop_task.load(Relaxed) {
                        wait_while_paused(&paused_task, &stop_task).await;
                        if stop_task.load(Relaxed) {
                            break;
                        }
                        buffer.clear();
                        match reader.read_until(b'\n', &mut buffer).await {
                            Ok(0) => break,
                            Ok(_) => ingest_line(
                                trim_log_line_bytes(&buffer),
                                &mut state,
                                &args,
                                &filters,
                                &file_tx,
                                &mut batch,
                                &batch_tx,
                            ),
                            Err(_) => break,
                        }
                    }

                    let _ = child.kill().await;
                }
            }

            flush_batch(&mut batch, &state, &batch_tx);
        });

        Self {
            batch_rx,
            stop,
            paused,
            task: Some(task),
            file_output,
        }
    }
}
