#![deny(clippy::unwrap_used)]

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::time::Duration;

use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::CliArgs;
use crate::LogEntry;
use crate::State;
use crate::build_logcat_command;
use crate::process_line;
use crate::trim_log_line;
use crate::trim_log_line_bytes;

use super::app::SourceMode;

const INGEST_BATCH_SIZE: usize = 128;
const PAUSE_POLL: Duration = Duration::from_millis(50);

async fn wait_while_paused(paused: &AtomicBool, stop: &AtomicBool) {
    while paused.load(Relaxed) && !stop.load(Relaxed) {
        tokio::time::sleep(PAUSE_POLL).await;
    }
}

fn flush_batch(batch: &mut Vec<IngestItem>, tx: &mpsc::UnboundedSender<Vec<IngestItem>>) {
    if batch.is_empty() {
        return;
    }
    let _ = tx.send(std::mem::take(batch));
}

fn ingest_line(
    line: String,
    state: &mut State,
    args: &CliArgs,
    batch: &mut Vec<IngestItem>,
    tx: &mpsc::UnboundedSender<Vec<IngestItem>>,
) {
    if let Some(entry) = process_line(&line, state, args) {
        batch.push(IngestItem {
            entry,
            state: state.clone(),
        });
        if batch.len() >= INGEST_BATCH_SIZE {
            flush_batch(batch, tx);
        }
    }
}

#[derive(Clone)]
pub struct IngestItem {
    pub entry: LogEntry,
    pub state: State,
}

pub struct LogIngest {
    batch_rx: mpsc::UnboundedReceiver<Vec<IngestItem>>,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    task: Option<JoinHandle<()>>,
}

impl LogIngest {
    pub fn idle() -> Self {
        let (_tx, batch_rx) = mpsc::unbounded_channel();
        Self {
            batch_rx,
            stop: Arc::new(AtomicBool::new(true)),
            paused: Arc::new(AtomicBool::new(false)),
            task: None,
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
    }

    pub fn try_recv_batch(&mut self) -> Option<Vec<IngestItem>> {
        self.batch_rx.try_recv().ok()
    }

    pub fn start(
        handle: &tokio::runtime::Handle,
        source: SourceMode,
        args: CliArgs,
        mut state: State,
        device_serial: Option<String>,
    ) -> Self {
        let (batch_tx, batch_rx) = mpsc::unbounded_channel();
        let stop = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));
        let stop_task = Arc::clone(&stop);
        let paused_task = Arc::clone(&paused);

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
                                &mut batch,
                                &batch_tx,
                            ),
                            Err(_) => break,
                        }
                    }

                    let _ = child.kill().await;
                }
            }

            flush_batch(&mut batch, &batch_tx);
        });

        Self {
            batch_rx,
            stop,
            paused,
            task: Some(task),
        }
    }
}
