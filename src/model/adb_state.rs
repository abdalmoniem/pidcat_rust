#[derive(Debug)]
pub enum AdbState {
    Device,
    Emulator,
    Offline,
    UnAuthorized,
    Recovery,
    Sideload,
    NoPermissions,
    NoDevice,
}

impl From<&str> for AdbState {
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
    fn from(str: String) -> Self {
        Self::from(str.as_str())
    }
}
