---
name: batstat-device-onboarder
description: >-
  Guide and execute the onboarding of new peripherals (mice, keyboards, controllers) into the BatStat tray battery monitor application.
---

# BatStat Device Onboarder

## Overview
This skill guides you through onboarding a new wireless/wired peripheral into the BatStat tray battery monitor. It uses direct HID/HID++ probing utilities to identify the exact USB/HID interfaces, device slots, and battery feature pages supported by the device, and then generates the necessary Rust plugin files and registrations.

## Dependencies
This skill requires `cargo` and the `hidapi` compilation environment to run the utility scripts.

## Quick Start
When a user asks to onboard a new peripheral:
1. Ask the user for:
   - Device Vendor (e.g. Logitech, Pulsar)
   - Device Type (e.g. Mouse, Keyboard, Gamepad)
   - Model Name (e.g. Pro Superlight)
   - Current battery level percentage reported by the official vendor software (e.g. 74%).
2. Follow the **Utility Scripts** section below to scan and identify the target device and its active battery reporting slot.

## Utility Scripts

This skill includes two helper scripts inside the `scripts/` folder:
- `list_devices.rs`: Enumerates all connected HID devices.
- `probe_device.rs`: Probes battery-reporting slots for a target Vendor ID and Product ID, looking for the slot reporting a battery level closest to the expected percentage.

### How to Run:
The AI agent must copy the helper script(s) to the project's `src/bin/` directory temporarily, run them via `cargo`, parse the outputs, and delete them from `src/bin/` when done.

#### 1. Enumerating Connected Devices
Copy `list_devices.rs` to `src/bin/list_devices.rs` and run:
```bash
cargo run --bin list_devices
```
Search the output for the target vendor (e.g., `Logitech` or `0x046d`) to identify the **Vendor ID (VID)** and **Product ID (PID)** of the receiver/device.

#### 2. Probing Slots and Features
Copy `probe_device.rs` to `src/bin/probe_device.rs` and run:
```bash
cargo run --bin probe_device -- --target-vid <VID> --target-pid <PID> --expected-battery <EXPECTED_PERCENTAGE>
```
*Example*:
```bash
cargo run --bin probe_device -- --target-vid 0x046d --target-pid 0xc54d --expected-battery 74
```
Analyze the output to find the matching slot index (e.g. `MATCH FOUND: Slot 0x01 reports 74%`).

---

## Onboarding Implementation Steps

Once the interface usage page, usage, and device index/slot are identified:

### 1. Create a new Plugin
Create a new plugin file `src/plugins/<device_name>.rs` following this structure:
```rust
use crate::plugins::{DeviceBatteryStatus, DeviceInstance, DevicePlugin};
use hidapi::HidApi;

pub struct MyDevicePlugin;

impl DevicePlugin for MyDevicePlugin {
    fn scan(&self, api: &HidApi) -> Vec<Box<dyn DeviceInstance>> {
        let mut instances: Vec<Box<dyn DeviceInstance>> = Vec::new();
        for dev_info in api.device_list() {
            if dev_info.vendor_id() == TARGET_VID && dev_info.usage_page() == TARGET_USAGE_PAGE && dev_info.usage() == TARGET_USAGE {
                instances.push(Box::new(MyDeviceInstance {
                    path: dev_info.path().to_owned(),
                    product_id: dev_info.product_id(),
                }));
            }
        }
        instances
    }
}

pub struct MyDeviceInstance {
    path: std::ffi::CString,
    product_id: u16,
}

impl DeviceInstance for MyDeviceInstance {
    fn unique_id(&self) -> String {
        format!("mydevice_{:#06x}", self.product_id)
    }

    fn default_name(&self) -> String {
        "My Device Name".to_string()
    }

    fn query_battery(&self, api: &HidApi) -> Result<DeviceBatteryStatus, String> {
        let device = api.open_path(&self.path)
            .map_err(|e| format!("Failed to open path: {}", e))?;
        
        // [Query code and byte parsing logic goes here]
        
        Ok(DeviceBatteryStatus::simple(percentage, charging, true))
    }
}
```

### 2. Register the Plugin
1. Add the module in `src/plugins/mod.rs` (`pub mod <device_name>;`).
2. Add `Box::new(plugins::<device_name>::MyDevicePlugin)` to the `plugins` vector in the background polling loop in `src/main.rs`.
3. Map the unique ID prefix in `get_default_low_icon` in `src/main.rs` to map to the correct default icon (e.g. `low_mouse.png` or `low_keyboard.png`).

### 3. Update the UI
1. Update `tag` in `src/ui.rs` to classify the device prefix with its device type tag (e.g. `MOUSE` or `KEYBOARD`).
2. Update `default_icon_name` in `src/ui.rs` to map the device prefix to the correct default icon file.

### 4. Verify & Document
1. Run `cargo check` to verify compilation.
2. Update `DEVICELIST.md` with connection protocols and battery telemetry details.

---

## Common Mistakes
- **Forgetting to Clean Up**: Always delete `list_devices.rs` and `probe_device.rs` from `src/bin/` after executing the scan, keeping the git workspace clean.
- **Incorrect Usage/Usage Page**: Always target the vendor collection (usually `usage_page == 0xff00` and `usage == 0x0002` or `0x0001`). Querying standard mouse/keyboard interfaces will result in `Unzulässige Funktion` (Invalid Function) errors.
- **Hardcoding Device Index**: Receivers can have multiple devices connected. Use the discovered index (usually `0x01` but sometimes `0x02` or higher) in the query payloads.
