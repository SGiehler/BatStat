use crate::plugins::{DeviceBatteryStatus, DeviceInstance, DevicePlugin};
use hidapi::HidApi;
use windows_sys::Win32::UI::Input::XboxController::{
    XInputGetBatteryInformation,
    BATTERY_DEVTYPE_GAMEPAD,
    XINPUT_BATTERY_INFORMATION,
    BATTERY_TYPE_DISCONNECTED,
    BATTERY_TYPE_WIRED,
    BATTERY_LEVEL_EMPTY,
    BATTERY_LEVEL_LOW,
    BATTERY_LEVEL_MEDIUM,
    BATTERY_LEVEL_FULL,
};

pub struct XboxPlugin;

impl DevicePlugin for XboxPlugin {
    fn scan(&self, _api: &HidApi) -> Vec<Box<dyn DeviceInstance>> {
        let mut instances: Vec<Box<dyn DeviceInstance>> = Vec::new();
        for slot in 0..4 {
            if is_xbox_connected(slot) {
                instances.push(Box::new(XboxDeviceInstance { slot }));
            }
        }
        instances
    }
}

fn is_xbox_connected(slot: u32) -> bool {
    unsafe {
        let mut battery_info = std::mem::zeroed::<XINPUT_BATTERY_INFORMATION>();
        let result = XInputGetBatteryInformation(
            slot,
            BATTERY_DEVTYPE_GAMEPAD,
            &mut battery_info,
        );
        result == 0 && battery_info.BatteryType != BATTERY_TYPE_DISCONNECTED
    }
}

pub struct XboxDeviceInstance {
    slot: u32,
}

impl DeviceInstance for XboxDeviceInstance {
    fn unique_id(&self) -> String {
        format!("xbox_slot_{}", self.slot)
    }

    fn default_name(&self) -> String {
        format!("Xbox Controller (Slot {})", self.slot)
    }

    fn query_battery(&self, _api: &HidApi) -> Result<DeviceBatteryStatus, String> {
        unsafe {
            let mut battery_info = std::mem::zeroed::<XINPUT_BATTERY_INFORMATION>();
            let result = XInputGetBatteryInformation(
                self.slot,
                BATTERY_DEVTYPE_GAMEPAD,
                &mut battery_info,
            );

            if result != 0 {
                return Err(format!("XInput query failed: code {}", result));
            }

            if battery_info.BatteryType == BATTERY_TYPE_DISCONNECTED {
                return Ok(DeviceBatteryStatus::simple(0, false, false));
            }

            let charging = battery_info.BatteryType == BATTERY_TYPE_WIRED;
            
            // XInput doesn't provide exact percentages, only Empty/Low/Medium/Full
            let percentage = match battery_info.BatteryLevel {
                BATTERY_LEVEL_EMPTY => 5,
                BATTERY_LEVEL_LOW => 25,
                BATTERY_LEVEL_MEDIUM => 55,
                BATTERY_LEVEL_FULL => 90,
                _ => 50, // Unknown
            };

            Ok(DeviceBatteryStatus::simple(percentage, charging, true))
        }
    }
}
