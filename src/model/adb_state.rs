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

//! Connection states reported by the Android Debug Bridge.

/// Device row state as printed by `adb devices -l` (second column).
#[derive(Debug)]
pub enum AdbState {
    /// Ready for commands (`device`).
    Device,
    /// Emulator instance (`emulator`).
    Emulator,
    /// Present but not connected (`offline`).
    Offline,
    /// USB debugging not authorized on the device (`unauthorized`).
    UnAuthorized,
    /// Booted into recovery (`recovery`).
    Recovery,
    /// Side-load / fastboot-adjacent mode (`sideload`).
    Sideload,
    /// Host lacks permission to access the device (`no permissions`).
    NoPermissions,
    /// No device selected or available (`no device`).
    NoDevice,
}

impl From<&str> for AdbState {
    /// Parses the exact lowercase state token from `adb devices`.
    ///
    /// # Panics
    ///
    /// Panics if `str` is not one of the known ADB state strings.
    fn from(str: &str) -> Self {
        match str {
            "device" => Self::Device,
            "emulator" => Self::Emulator,
            "offline" => Self::Offline,
            "unauthorized" => Self::UnAuthorized,
            "recovery" => Self::Recovery,
            "sideload" => Self::Sideload,
            "no permissions" => Self::NoPermissions,
            "no device" => Self::NoDevice,
            _ => panic!("Invalid AdbState: {str}"),
        }
    }
}

impl From<String> for AdbState {
    /// Parses owned state text via [`From<&str>`].
    fn from(str: String) -> Self {
        Self::from(str.as_str())
    }
}

impl AdbState {
    /// Returns the canonical `adb devices` state string for this variant.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Device => "device",
            Self::Emulator => "emulator",
            Self::Offline => "offline",
            Self::UnAuthorized => "unauthorized",
            Self::Recovery => "recovery",
            Self::Sideload => "sideload",
            Self::NoPermissions => "no permissions",
            Self::NoDevice => "no device",
        }
    }
}
