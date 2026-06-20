use crate::plugins::{DeviceBatteryStatus, DeviceInstance, DevicePlugin};
use hidapi::HidApi;

const STEELSERIES_VENDOR_ID: u16 = 0x1038;
const GAMEBUDS_PID: u16 = 0x230a;

pub struct SteelSeriesPlugin;

impl DevicePlugin for SteelSeriesPlugin {
    fn scan(&self, api: &HidApi) -> Vec<Box<dyn DeviceInstance>> {
        let mut instances: Vec<Box<dyn DeviceInstance>> = Vec::new();
        for dev_info in api.device_list() {
            if dev_info.vendor_id() == STEELSERIES_VENDOR_ID && dev_info.product_id() == GAMEBUDS_PID {
                if dev_info.usage_page() == 0xffc0 || dev_info.interface_number() == 3 {
                    let path = dev_info.path().to_owned();
                    // Return both Left and Right earbud instances for this device
                    instances.push(Box::new(GameBudsLeftInstance { path: path.clone() }));
                    instances.push(Box::new(GameBudsRightInstance { path }));
                }
            }
        }
        instances
    }
}

fn query_buds_raw(api: &HidApi, path: &std::ffi::CStr) -> Result<(u8, bool, bool, u8, bool, bool), String> {
    let device = api.open_path(path)
        .map_err(|e| format!("Failed to open device path: {}", e))?;

    let cmd = [0x00, 0xb0];
    device.write(&cmd)
        .map_err(|e| format!("HID write failed: {}", e))?;

    let mut buf = [0u8; 64];
    let bytes_read = device.read_timeout(&mut buf, 1500)
        .map_err(|e| format!("HID read timeout: {}", e))?;

    if bytes_read < 14 {
        return Err(format!("Response too short: {} bytes", bytes_read));
    }

    if buf[0] != 0xb0 {
        return Err(format!("Unexpected response code: {:#x}, expected 0xb0", buf[0]));
    }

    // Charging state: buf[1] for Left, buf[2] for Right (1 = charging, 0 = discharging/not in case)
    let left_charging = buf[1] == 1;
    let right_charging = buf[2] == 1;

    // Connection state: buf[13] bitmask (bit 0 = Left online, bit 1 = Right online)
    let left_online = (buf[13] & 0x01) != 0;
    let right_online = (buf[13] & 0x02) != 0;

    let left_percentage = buf[5];
    let right_percentage = buf[6];

    Ok((
        left_percentage,
        left_charging,
        left_online,
        right_percentage,
        right_charging,
        right_online,
    ))
}

pub struct GameBudsLeftInstance {
    path: std::ffi::CString,
}

impl DeviceInstance for GameBudsLeftInstance {
    fn unique_id(&self) -> String {
        format!("gamebuds_left_{}", self.path.to_string_lossy())
    }

    fn default_name(&self) -> String {
        "GameBuds - Left".to_string()
    }

    fn query_battery(&self, api: &HidApi) -> Result<DeviceBatteryStatus, String> {
        let (left_pct, left_chg, left_on, _, _, _) = query_buds_raw(api, &self.path)?;
        Ok(DeviceBatteryStatus {
            percentage: left_pct,
            charging: left_chg,
            is_online: left_on,
        })
    }
}

pub struct GameBudsRightInstance {
    path: std::ffi::CString,
}

impl DeviceInstance for GameBudsRightInstance {
    fn unique_id(&self) -> String {
        format!("gamebuds_right_{}", self.path.to_string_lossy())
    }

    fn default_name(&self) -> String {
        "GameBuds - Right".to_string()
    }

    fn query_battery(&self, api: &HidApi) -> Result<DeviceBatteryStatus, String> {
        let (_, _, _, right_pct, right_chg, right_on) = query_buds_raw(api, &self.path)?;
        Ok(DeviceBatteryStatus {
            percentage: right_pct,
            charging: right_chg,
            is_online: right_on,
        })
    }
}
