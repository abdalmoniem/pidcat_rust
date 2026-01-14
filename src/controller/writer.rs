use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use std::fs::File;

use std::io::Result;
use std::io::Stdout;
use std::io::Write;
use std::io::stdout;

use crate::ValueOrPanic;

#[derive(Debug)]
enum WriterTarget {
    Console(Stdout),
    File(File),
}

impl Display for WriterTarget {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Console(stdout) => write!(formatter, "{stdout:?}"),
            Self::File(file) => write!(formatter, "{file:?}"),
        }
    }
}

impl Write for WriterTarget {
    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        match self {
            Self::Console(stdout) => stdout.write_all(buffer).map(|_| buffer.len()),
            Self::File(file) => file.write_all(buffer).map(|_| buffer.len()),
        }
    }

    fn flush(&mut self) -> Result<()> {
        match self {
            Self::Console(stdout) => stdout.flush(),
            Self::File(file) => file.flush(),
        }
    }
}

#[derive(Debug)]
pub struct Writer {
    pub width: i16,
    pub show_colors: bool,
    target: WriterTarget,
}

impl Writer {
    pub fn new_console(width: i16, show_colors: bool) -> Self {
        Self {
            width,
            show_colors,
            target: WriterTarget::Console(stdout()),
        }
    }

    pub fn new_file(file: File) -> Self {
        Self {
            width: -1,
            show_colors: false,
            target: WriterTarget::File(file),
        }
    }

    pub fn write(&mut self, text: &str) {
        let err_msg = format!("Failed to write to {}", self.target);
        self.target.write(text.as_bytes()).unwrap_or_panic(&err_msg);
    }

    pub fn flush(&mut self) {
        let err_msg = format!("Failed to flush {}", self.target);
        self.target.flush().unwrap_or_panic(&err_msg);
    }
}
