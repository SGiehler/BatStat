pub mod pulsar;
pub mod steelseries;
pub mod xbox;
pub mod logitech;


#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelType {
    Main,
    Left,
    Right,
    Case,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BatteryChannel {
    pub channel_type: ChannelType,
    pub percentage: u8,
    pub charging: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceBatteryStatus {
    Offline,
    Online {
        channels: [Option<BatteryChannel>; 4],
    },
}

impl DeviceBatteryStatus {
    pub fn simple(percentage: u8, charging: bool, is_online: bool) -> Self {
        if is_online {
            Self::Online {
                channels: [
                    Some(BatteryChannel {
                        channel_type: ChannelType::Main,
                        percentage,
                        charging,
                    }),
                    None,
                    None,
                    None,
                ],
            }
        } else {
            Self::Offline
        }
    }

    pub fn is_online(&self) -> bool {
        match self {
            Self::Offline => false,
            Self::Online { channels } => channels.iter().any(|c| c.is_some()),
        }
    }

    #[allow(dead_code)]
    pub fn is_charging(&self) -> bool {
        match self {
            Self::Offline => false,
            Self::Online { channels } => channels.iter().flatten().any(|c| c.charging),
        }
    }

    pub fn effective_percentage(&self) -> u8 {
        match self {
            Self::Offline => 0,
            Self::Online { channels } => channels.iter().flatten().map(|c| c.percentage).min().unwrap_or(0),
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
