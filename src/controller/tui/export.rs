#![deny(clippy::unwrap_used)]

use std::collections::VecDeque;
use std::fs::File;
use std::path::Path;
use std::thread;
use std::thread::JoinHandle;

use crate::CliArgs;
use crate::LogEntry;
use crate::State;
use crate::Writer;
use crate::render_entry;

use super::app::SourceMode;

pub struct ExportJob {
    pub path: String,
    handle: JoinHandle<Result<usize, String>>,
}

impl ExportJob {
    pub fn start(
        path: String,
        entries: VecDeque<LogEntry>,
        state: &State,
        args: &CliArgs,
    ) -> Result<Self, String> {
        let mut render_state = state.clone();
        render_state.last_tag = None;
        let args = args.clone();
        let target = path.clone();

        let handle = thread::Builder::new()
            .name(format!("{}-export", env!("CARGO_PKG_NAME")))
            .spawn(move || write_entries(&target, &entries, &mut render_state, &args))
            .map_err(|err| format!("cannot start export: {err}"))?;

        Ok(Self { path, handle })
    }

    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }

    pub fn finish(self) -> String {
        match self.handle.join() {
            Ok(Ok(count)) => format!(
                "exported {} entries to {}",
                crate::controller::util::format_usize_separated(count),
                self.path
            ),
            Ok(Err(err)) => format!("export failed: {err}"),
            Err(_) => format!("export failed: {}", self.path),
        }
    }
}

fn write_entries(
    path: &str,
    entries: &VecDeque<LogEntry>,
    state: &mut State,
    args: &CliArgs,
) -> Result<usize, String> {
    let file = File::create(path).map_err(|err| format!("cannot create {path}: {err}"))?;
    let mut writer = Writer::new_file(file);
    for entry in entries {
        render_entry(entry, state, args, std::slice::from_mut(&mut writer));
    }
    writer.flush();
    Ok(entries.len())
}

pub fn default_export_file_name(source: &SourceMode, device: Option<&str>) -> String {
    let origin = match source {
        SourceMode::Live => device.unwrap_or("pipe").to_string(),
        SourceMode::File(path) => Path::new(path)
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_else(|| "pipe".to_string()),
        SourceMode::Pipe => "pipe".to_string(),
    };
    let time = chrono::Local::now().format("%I-%M-%S%.3f_%p");
    format!("{}_{origin}_{time}.log", env!("CARGO_PKG_NAME"))
}
