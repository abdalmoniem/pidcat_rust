use crate::AdbState;

#[derive(Debug)]
pub struct AdbDevice {
    pub device_id: String,
    pub device_state: AdbState,
}
