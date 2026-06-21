use crate::plugins::{DeviceBatteryStatus, DeviceInstance, DevicePlugin};
use hidapi::HidApi;

const LOGITECH_VENDOR_ID: u16 = 0x046d;
const CMD_ROOT_GET_FEATURE: u8 = 0x00;
const FEATURE_UNIFIED_BATTERY: u16 = 0x1004;
const CMD_UNIFIED_BATTERY_GET_STATUS: u8 = 0x10;

pub struct LogitechPlugin;

impl DevicePlugin for LogitechPlugin {
    fn scan(&self, api: Option<&HidApi>) -> Vec<Box<dyn DeviceInstance>> {
        let mut instances: Vec<Box<dyn DeviceInstance>> = Vec::new();
        if let Some(api) = api {
            for dev_info in api.device_list() {
                if dev_info.vendor_id() == LOGITECH_VENDOR_ID {
                    // Look for the vendor interface that handles HID++ commands
                    if dev_info.usage_page() == 0xff00 && dev_info.usage() == 0x0002 {
                        instances.push(Box::new(LogitechDeviceInstance {
                            path: dev_info.path().to_owned(),
                            product_id: dev_info.product_id(),
                        }));
                    }
                }
            }
        }
        instances
    }
}

pub struct LogitechDeviceInstance {
    path: std::ffi::CString,
    product_id: u16,
}

impl DeviceInstance for LogitechDeviceInstance {
    fn unique_id(&self) -> String {
        format!("logitech_{:#06x}", self.product_id)
    }

    fn default_name(&self) -> String {
        // Special-case the Pro Superlight receiver product ID
        if self.product_id == 0xc54d {
            "Logitech Pro Superlight".to_string()
        } else {
            format!("Logitech Wireless Device ({:#06x})", self.product_id)
        }
    }

    fn query_battery(&self, api: Option<&HidApi>) -> Result<DeviceBatteryStatus, String> {
        let api = api.ok_or_else(|| "HIDAPI not initialized".to_string())?;
        let device = api.open_path(&self.path)
            .map_err(|e| format!("Failed to open Logitech device path: {}", e))?;

        // 1. Get feature index of Unified Battery (0x1004) from Root feature (0x00)
        let mut req = [0u8; 20];
        req[0] = 0x11; // Long report ID
        req[1] = 0x01; // Device index (usually 0x01 for first wireless device)
        req[2] = 0x00; // Root feature index
        req[3] = CMD_ROOT_GET_FEATURE;
        req[4] = (FEATURE_UNIFIED_BATTERY >> 8) as u8;
        req[5] = (FEATURE_UNIFIED_BATTERY & 0xff) as u8;

        device.write(&req)
            .map_err(|e| format!("HID write failed for GetFeature: {}", e))?;

        let mut buf = [0u8; 64];
        let bytes_read = device.read_timeout(&mut buf, 150)
            .map_err(|e| format!("HID read timeout for GetFeature: {}", e))?;

        if bytes_read == 0 {
            return Ok(DeviceBatteryStatus::Offline);
        }

        if bytes_read < 20 {
            return Err(format!("GetFeature response too short: {} bytes", bytes_read));
        }

        // Response format:
        // buf[0] == 0x11
        // buf[1] == 0x01 (Device index)
        // buf[2] == 0x00 (Root feature)
        // buf[3] == 0x00 (Function ID for GetFeature)
        // buf[4] == Feature Index (if > 0, found!)
        if buf[0] != 0x11 || buf[1] != 0x01 || buf[2] != 0x00 || buf[3] != 0x00 {
            return Err("Unexpected response to GetFeature request".to_string());
        }

        let feature_idx = buf[4];
        if feature_idx == 0 {
            return Err("Unified Battery feature (0x1004) not supported by device".to_string());
        }

        // 2. Query Unified Battery Status from the discovered feature index
        let mut status_req = [0u8; 20];
        status_req[0] = 0x11;
        status_req[1] = 0x01;
        status_req[2] = feature_idx;
        status_req[3] = CMD_UNIFIED_BATTERY_GET_STATUS;

        device.write(&status_req)
            .map_err(|e| format!("HID write failed for GetBatteryStatus: {}", e))?;

        let bytes_read = device.read_timeout(&mut buf, 150)
            .map_err(|e| format!("HID read timeout for GetBatteryStatus: {}", e))?;

        if bytes_read == 0 {
            return Ok(DeviceBatteryStatus::Offline);
        }

        if bytes_read < 20 {
            return Err(format!("GetBatteryStatus response too short: {} bytes", bytes_read));
        }

        // Check if report ID, device index, feature index, and command ID match
        if buf[0] != 0x11 || buf[1] != 0x01 || buf[2] != feature_idx || buf[3] != CMD_UNIFIED_BATTERY_GET_STATUS {
            return Err("Unexpected response to GetBatteryStatus request".to_string());
        }

        // Parameters:
        // buf[4] == state of charge percentage (0-100)
        // buf[5] == next level / level class (we don't need this)
        // buf[6] == charging status (0 = Discharging, 1 = Recharging/Charging, etc.)
        let percentage = buf[4];
        let charging = buf[6] == 1; // 1 = Recharging/charging

        Ok(DeviceBatteryStatus::simple(percentage, charging, true))
    }
}
