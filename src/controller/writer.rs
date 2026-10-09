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

//! Output sinks used when rendering log entries.
//!
//! A `Writer` wraps one destination (the console, a file, or an in-memory buffer) together with
//! the layout settings the renderers need: the available line width and whether ANSI colors
//! should be emitted. The renderers in `log_processor` write
//! to a slice of writers so one entry can be shown on the console and saved to a file at once.

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

/// The concrete destination behind a [`Writer`].
#[derive(Debug)]
enum WriterTarget {
    /// Buffered standard output.
    Console(BufWriter<Stdout>),
    /// Buffered file, for example the `--output` path.
    File(BufWriter<File>),
    /// In-memory byte buffer, retrievable through [`Writer::take_buffer`].
    Buffer(Vec<u8>),
}

/// Human-readable description of the target, used in panic messages when a write or flush fails.
impl Display for WriterTarget {
    /// Formats the underlying writer with `{:?}` for console and file targets, and as the literal
    /// `Buffer` for in-memory targets (whose contents are not printed).
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Console(stdout) => write!(formatter, "{stdout:?}"),
            Self::File(file) => write!(formatter, "{file:?}"),
            Self::Buffer(_) => write!(formatter, "Buffer"),
        }
    }
}

/// Dispatches byte writes and flushes to the active target variant.
impl Write for WriterTarget {
    /// Writes the whole `buffer` to the target and reports it as fully written.
    ///
    /// Console and file targets use `write_all` on their buffered writer, so the returned count is
    /// always `buffer.len()` on success; the in-memory target appends the bytes to its vector.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of the underlying console or file writer. The buffer target never
    /// fails.
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

    /// Flushes the buffered console or file writer; flushing the in-memory buffer is a no-op.
    ///
    /// # Errors
    ///
    /// Returns the I/O error of the underlying console or file writer.
    fn flush(&mut self) -> Result<()> {
        match self {
            Self::Console(stdout) => stdout.flush(),
            Self::File(file) => file.flush(),
            Self::Buffer(_) => Ok(()),
        }
    }
}

/// A rendering destination plus the layout settings used when writing to it.
///
/// Construct one with [`Writer::new_console`], [`Writer::new_file`], or [`Writer::new_buffer`].
/// Writes go through [`Writer::write`] and are only guaranteed to reach the destination after
/// [`Writer::flush`].
#[derive(Debug)]
pub struct Writer {
    /// Available line width in columns used for wrapping, or `None` when output must not be
    /// wrapped (files). A value of `-1` is treated by the renderers as "unlimited width".
    pub width: Option<i16>,
    /// Whether ANSI color escape sequences are kept; when `false` they are stripped before
    /// writing.
    pub show_colors: bool,
    /// The destination the text is written to.
    target: WriterTarget,
}

/// Constructors and output operations for [`Writer`].
impl Writer {
    /// Creates a writer for buffered standard output.
    ///
    /// `width` is the terminal width used for wrapping and `show_colors` controls whether ANSI
    /// colors are emitted.
    pub fn new_console(width: i16, show_colors: bool) -> Self {
        Self {
            width: Some(width),
            show_colors,
            target: WriterTarget::Console(BufWriter::new(stdout())),
        }
    }

    /// Creates a writer for a buffered file.
    ///
    /// File output is never wrapped (`width` is `None`) and never colored (`show_colors` is
    /// `false`), so saved logs stay plain text.
    pub fn new_file(file: File) -> Self {
        Self {
            width: None,
            show_colors: false,
            target: WriterTarget::File(BufWriter::new(file)),
        }
    }

    /// Creates a writer that collects its output in memory.
    ///
    /// Use [`Writer::take_buffer`] to retrieve the rendered text. `width` and `show_colors`
    /// behave as for [`Writer::new_console`].
    pub fn new_buffer(width: i16, show_colors: bool) -> Self {
        Self {
            width: Some(width),
            show_colors,
            target: WriterTarget::Buffer(Vec::default()),
        }
    }

    /// Consumes a buffer writer and returns everything written to it as a [`String`].
    ///
    /// # Panics
    ///
    /// Panics if this writer was not created with [`Writer::new_buffer`], or if the collected
    /// bytes are not valid UTF-8.
    pub fn take_buffer(self) -> String {
        match self.target {
            WriterTarget::Buffer(buffer) => {
                String::from_utf8(buffer).unwrap_or_panic("Buffer writer produced invalid UTF-8")
            }
            _ => panic!("Writer is not a buffer"),
        }
    }

    /// Writes `text` to the target exactly as given (no wrapping, coloring, or stripping is
    /// applied here).
    ///
    /// # Panics
    ///
    /// Panics with a message naming the target if the write fails.
    pub fn write(&mut self, text: &str) {
        let err_msg = format!("Failed to write to {target}", target = self.target);
        self.target.write(text.as_bytes()).unwrap_or_panic(&err_msg);
    }

    /// Flushes any buffered output to the destination.
    ///
    /// # Panics
    ///
    /// Panics with a message naming the target if the flush fails.
    pub fn flush(&mut self) {
        let err_msg = format!("Failed to flush {target}", target = self.target);
        self.target.flush().unwrap_or_panic(&err_msg);
    }
}
