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

//! Single ADB target as returned by `adb devices`.

use crate::AdbState;

/// One row from `adb devices`: serial or emulator id plus its reported state.
#[derive(Debug)]
pub struct AdbDevice {
    /// Device serial, USB id, or emulator name (first column of `adb devices`).
    pub device_id: String,
    /// Connection state string mapped to [`AdbState`].
    pub device_state: AdbState,
}
