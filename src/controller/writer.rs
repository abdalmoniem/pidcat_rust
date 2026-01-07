use colored::*;
use std::fs::File;
use std::io::{self, Write};

#[derive(Debug)]
pub enum WriterTarget {
    Console(io::Stdout),
    File(File),
}

impl Write for WriterTarget {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Console(stdout) => stdout.write(buf),
            Self::File(file) => file.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
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
    pub target: WriterTarget,
}

impl Writer {
    pub fn new_console(width: i16, show_colors: bool) -> Self {
        Self {
            width,
            show_colors,
            target: WriterTarget::Console(io::stdout()),
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
        let res = write!(self.target, "{text}");

        if let Err(err) = res {
            eprintln!("{}", err.to_string().red().bold());
        }
    }

    pub fn flush(&mut self) {
        let res = self.target.flush();

        if let Err(err) = res {
            eprintln!("{}", err.to_string().red().bold());
        }
    }
}
