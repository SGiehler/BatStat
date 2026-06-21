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

fn calculate_pulsar_percentage(voltage: u16, charging: bool) -> u8 {
    const W: [u16; 21] = [
        3050, 3420, 3480, 3540, 3600, 3660, 3720, 3760, 3800, 3840,
        3880, 3920, 3940, 3960, 3980, 4000, 4020, 4040, 4060, 4080,
        4110
    ];

    let mut s: f64;
    if voltage > W[W.len() - 1] {
        s = if charging { 99.0 } else { 100.0 };
    } else {
        let mut a = None;
        for n in 0..W.len() {
            if voltage <= W[n] {
                a = Some(n);
                break;
            }
        }

        if let Some(a_idx) = a {
            if a_idx == 0 {
                s = 0.0;
            } else {
                let interval_width = (W[a_idx] - W[a_idx - 1]) as f64 / 5.0;
                s = ((voltage - W[a_idx - 1]) as f64) / interval_width + 5.0 * ((a_idx - 1) as f64);
            }
            if s == 0.0 || s == 15.0 {
                s += 1.0;
            }
        } else {
            s = 0.0;
        }
    }

    s.round() as u8
}

pub struct PulsarPlugin;

impl DevicePlugin for PulsarPlugin {
    fn scan(&self, api: Option<&HidApi>) -> Vec<Box<dyn DeviceInstance>> {
        let mut instances: Vec<Box<dyn DeviceInstance>> = Vec::new();
        if let Some(api) = api {
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

    fn query_battery(&self, api: Option<&HidApi>) -> Result<DeviceBatteryStatus, String> {
        let api = api.ok_or_else(|| "HIDAPI not initialized".to_string())?;
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

        let raw_percentage = buf[6];
        let charging = buf[7] != 0;
        let voltage = ((buf[8] as u16) << 8) | (buf[9] as u16);

        let percentage = if voltage > 0 {
            calculate_pulsar_percentage(voltage, charging)
        } else {
            raw_percentage
        };

        Ok(DeviceBatteryStatus::simple(percentage, charging, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_pulsar_percentage() {
        assert_eq!(calculate_pulsar_percentage(3810, false), 41);
        assert_eq!(calculate_pulsar_percentage(3920, false), 55);
        assert_eq!(calculate_pulsar_percentage(4110, false), 100);
        assert_eq!(calculate_pulsar_percentage(3050, false), 1);
        assert_eq!(calculate_pulsar_percentage(4150, false), 100);
        assert_eq!(calculate_pulsar_percentage(4150, true), 99);
    }
}
