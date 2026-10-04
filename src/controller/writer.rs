use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use std::fs::File;

use std::io::BufWriter;
use std::io::Result;
use std::io::Stdout;
use std::io::Write;
use std::io::stdout;

use crate::ValueOrPanic;

#[derive(Debug)]
enum WriterTarget {
    Console(BufWriter<Stdout>),
    File(BufWriter<File>),
    Buffer(Vec<u8>),
}

impl Display for WriterTarget {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Console(stdout) => write!(formatter, "{stdout:?}"),
            Self::File(file) => write!(formatter, "{file:?}"),
            Self::Buffer(_) => write!(formatter, "Buffer"),
        }
    }
}

impl Write for WriterTarget {
    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        match self {
            Self::Console(stdout) => stdout.write_all(buffer).map(|_| buffer.len()),
            Self::File(file) => file.write_all(buffer).map(|_| buffer.len()),
            Self::Buffer(output) => {
                output.extend_from_slice(buffer);
                Ok(buffer.len())
            }
        }
    }

    fn flush(&mut self) -> Result<()> {
        match self {
            Self::Console(stdout) => stdout.flush(),
            Self::File(file) => file.flush(),
            Self::Buffer(_) => Ok(()),
        }
    }
}

#[derive(Debug)]
pub struct Writer {
    pub width: Option<i16>,
    pub show_colors: bool,
    target: WriterTarget,
}

impl Writer {
    pub fn new_console(width: i16, show_colors: bool) -> Self {
        Self {
            width: Some(width),
            show_colors,
            target: WriterTarget::Console(BufWriter::new(stdout())),
        }
    }

    pub fn new_file(file: File) -> Self {
        Self {
            width: None,
            show_colors: false,
            target: WriterTarget::File(BufWriter::new(file)),
        }
    }

    pub fn new_buffer(width: i16, show_colors: bool) -> Self {
        Self {
            width: Some(width),
            show_colors,
            target: WriterTarget::Buffer(Vec::default()),
        }
    }

    pub fn take_buffer(self) -> String {
        match self.target {
            WriterTarget::Buffer(buffer) => {
                String::from_utf8(buffer).unwrap_or_panic("Buffer writer produced invalid UTF-8")
            }
            _ => panic!("Writer is not a buffer"),
        }
    }

    pub fn write(&mut self, text: &str) {
        let err_msg = format!("Failed to write to {target}", target = self.target);
        self.target.write(text.as_bytes()).unwrap_or_panic(&err_msg);
    }

    pub fn flush(&mut self) {
        let err_msg = format!("Failed to flush {target}", target = self.target);
        self.target.flush().unwrap_or_panic(&err_msg);
    }
}
