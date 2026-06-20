use crate::plugins::{DeviceBatteryStatus, DeviceInstance, DevicePlugin};
use hidapi::HidApi;

const PULSAR_VENDOR_ID_1: u16 = 0x3554;
const PULSAR_VENDOR_ID_2: u16 = 0x3710;
const CMD_POWER: u8 = 0x04;
const REPORT_ID: u8 = 0x08;

fn calculate_checksum(payload: &[u8]) -> u8 {
    let sum: u32 = payload.iter().map(|&b| b as u32).sum();
    (0x55_u32.wrapping_sub(sum) & 0xFF) as u8
}

pub struct PulsarPlugin;

impl DevicePlugin for PulsarPlugin {
    fn scan(&self, api: &HidApi) -> Vec<Box<dyn DeviceInstance>> {
        let mut instances: Vec<Box<dyn DeviceInstance>> = Vec::new();
        for dev_info in api.device_list() {
            let vid = dev_info.vendor_id();
            if vid == PULSAR_VENDOR_ID_1 || vid == PULSAR_VENDOR_ID_2 {
                // Only match config/telemetry interface (usage page >= 0xFF00)
                if dev_info.usage_page() >= 0xFF00 {
                    instances.push(Box::new(PulsarDeviceInstance {
                        path: dev_info.path().to_owned(),
                        product_id: dev_info.product_id(),
                    }));
                }
            }
        }
        instances
    }
}

pub struct PulsarDeviceInstance {
    path: std::ffi::CString,
    product_id: u16,
}

impl DeviceInstance for PulsarDeviceInstance {
    fn unique_id(&self) -> String {
        // Path-independent unique ID to avoid multiple duplicate config entries
        format!("pulsar_{:#06x}", self.product_id)
    }

    fn default_name(&self) -> String {
        format!("Pulsar Mouse ({:#06x})", self.product_id)
    }

    fn query_battery(&self, api: &HidApi) -> Result<DeviceBatteryStatus, String> {
        let device = api.open_path(&self.path)
            .map_err(|e| format!("Failed to open device path: {}", e))?;

        let mut payload = [0u8; 17];
        payload[0] = REPORT_ID;
        payload[1] = CMD_POWER;
        
        let checksum = calculate_checksum(&payload[0..16]);
        payload[16] = checksum;

        device.write(&payload)
            .map_err(|e| format!("HID write failed: {}", e))?;

        let mut buf = [0u8; 64];
        let bytes_read = device.read_timeout(&mut buf, 1500)
            .map_err(|e| format!("HID read timeout: {}", e))?;

        if bytes_read < 17 {
            return Err(format!("Response too short: {} bytes, expected at least 17", bytes_read));
        }

        if buf[0] != REPORT_ID || buf[1] != CMD_POWER {
            return Err(format!(
                "Unexpected report ID or command: Report ID={:#x}, Command={:#x}",
                buf[0], buf[1]
            ));
        }

        let percentage = buf[6];
        let charging = buf[7] != 0;

        Ok(DeviceBatteryStatus::simple(percentage, charging, true))
    }
}
