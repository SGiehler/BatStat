pub mod pulsar;
pub mod steelseries;
pub mod xbox;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeviceBatteryStatus {
    pub percentage: u8,
    pub charging: bool,
    pub is_online: bool,
    
    pub left_percentage: Option<u8>,
    pub right_percentage: Option<u8>,
    pub left_charging: Option<bool>,
    pub right_charging: Option<bool>,
    pub left_online: Option<bool>,
    pub right_online: Option<bool>,
}

impl DeviceBatteryStatus {
    pub fn simple(percentage: u8, charging: bool, is_online: bool) -> Self {
        Self {
            percentage,
            charging,
            is_online,
            left_percentage: None,
            right_percentage: None,
            left_charging: None,
            right_charging: None,
            left_online: None,
            right_online: None,
        }
    }
}

pub trait DeviceInstance: Send + Sync {
    /// Unique identifier for config tracking (e.g. "pulsar_5406_path" or "xbox_slot_0")
    fn unique_id(&self) -> String;
    
    /// Default user-friendly display name
    fn default_name(&self) -> String;
    
    /// Polls the device and returns its current battery status
    fn query_battery(&self, api: &hidapi::HidApi) -> Result<DeviceBatteryStatus, String>;
}

pub trait DevicePlugin {
    /// Scans the system for matching active devices and returns them as instances
    fn scan(&self, api: &hidapi::HidApi) -> Vec<Box<dyn DeviceInstance>>;
}
