use crate::plugins::{DeviceBatteryStatus, DeviceInstance, DevicePlugin};
use hidapi::HidApi;
use windows::Gaming::Input::Gamepad;

pub struct XboxPlugin;

impl DevicePlugin for XboxPlugin {
    fn scan(&self, _api: &HidApi) -> Vec<Box<dyn DeviceInstance>> {
        let mut instances: Vec<Box<dyn DeviceInstance>> = Vec::new();
        if let Ok(gamepads) = Gamepad::Gamepads() {
            if let Ok(size) = gamepads.Size() {
                for idx in 0..size {
                    if let Ok(gp) = gamepads.GetAt(idx) {
                        instances.push(Box::new(XboxDeviceInstance {
                            slot: idx,
                            gamepad: gp,
                        }));
                    }
                }
            }
        }
        instances
    }
}

pub struct XboxDeviceInstance {
    slot: u32,
    gamepad: Gamepad,
}

impl DeviceInstance for XboxDeviceInstance {
    fn unique_id(&self) -> String {
        format!("xbox_slot_{}", self.slot)
    }

    fn default_name(&self) -> String {
        format!("Xbox Controller (Slot {})", self.slot)
    }

    fn query_battery(&self, _api: &HidApi) -> Result<DeviceBatteryStatus, String> {
        let report = self.gamepad.TryGetBatteryReport()
            .map_err(|e| format!("Failed to get battery report: {:?}", e))?;
        
        let status = report.Status()
            .map_err(|e| format!("Failed to get battery status: {:?}", e))?;
        
        if status == windows::System::Power::BatteryStatus::NotPresent {
            return Ok(DeviceBatteryStatus::simple(0, false, false));
        }

        let charging = status == windows::System::Power::BatteryStatus::Charging;

        let rem_val = report.RemainingCapacityInMilliwattHours()
            .ok()
            .and_then(|r| r.Value().ok());
        let full_val = report.FullChargeCapacityInMilliwattHours()
            .ok()
            .and_then(|r| r.Value().ok());
        
        let percentage = if let (Some(rem), Some(full)) = (rem_val, full_val) {
            if full > 0 {
                let pct = (rem as f32 / full as f32 * 100.0) as u8;
                pct.min(100)
            } else {
                50
            }
        } else {
            // Fallback status mappings if capacity reports are unsupported
            match status {
                windows::System::Power::BatteryStatus::Idle => 100,
                windows::System::Power::BatteryStatus::Discharging => 55, // assume medium
                _ => 50,
            }
        };

        Ok(DeviceBatteryStatus::simple(percentage, charging, true))
    }
}
