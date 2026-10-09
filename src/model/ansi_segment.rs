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

//! Mapping ANSI escape codes to indices in stripped (visible) text.

/// An escape sequence and the visible-text index where it applies.
#[derive(Debug, Clone)]
pub struct AnsiSegment {
    /// Full CSI (or related) escape sequence, including `\x1b` prefix.
    pub code: String,
    /// Byte/character index in plain text after escapes are removed.
    pub pos: usize,
}
