use hidapi::HidApi;
use std::env;

fn parse_u16(s: &str) -> Option<u16> {
    if s.starts_with("0x") || s.starts_with("0X") {
        u16::from_str_radix(&s[2..], 16).ok()
    } else {
        s.parse::<u16>().ok()
    }
}

fn parse_u8(s: &str) -> Option<u8> {
    if s.starts_with("0x") || s.starts_with("0X") {
        u8::from_str_radix(&s[2..], 16).ok()
    } else {
        s.parse::<u8>().ok()
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut target_vid = None;
    let mut target_pid = None;
    let mut expected_battery = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--target-vid" => {
                if i + 1 < args.len() {
                    target_vid = parse_u16(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --target-vid requires an argument");
                    return;
                }
            }
            "--target-pid" => {
                if i + 1 < args.len() {
                    target_pid = parse_u16(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --target-pid requires an argument");
                    return;
                }
            }
            "--expected-battery" => {
                if i + 1 < args.len() {
                    expected_battery = parse_u8(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --expected-battery requires an argument");
                    return;
                }
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                i += 1;
            }
        }
    }

    let vid = match target_vid {
        Some(v) => v,
        None => {
            eprintln!("Error: --target-vid is required (e.g. --target-vid 0x046d)");
            return;
        }
    };

    let expected = match expected_battery {
        Some(b) => b,
        None => {
            eprintln!("Error: --expected-battery is required (e.g. --expected-battery 74)");
            return;
        }
    };

    println!("Probing devices with VID: {:#06x} (PID filter: {:?}) looking for expected battery: {}%...", vid, target_pid, expected);

    let api = match HidApi::new() {
        Ok(api) => api,
        Err(e) => {
            eprintln!("Failed to initialize HID API: {}", e);
            return;
        }
    };

    let mut found_any = false;

    for dev_info in api.device_list() {
        if dev_info.vendor_id() == vid {
            if let Some(pid) = target_pid {
                if dev_info.product_id() != pid {
                    continue;
                }
            }

            // We look for vendor interface with usage_page == 0xff00
            if dev_info.usage_page() == 0xff00 {
                println!(
                    "Probing interface Path: {:?} | PID: {:#06x} | IF: {} | Usage: {:#06x}",
                    dev_info.path(),
                    dev_info.product_id(),
                    dev_info.interface_number(),
                    dev_info.usage()
                );

                let device = match api.open_path(dev_info.path()) {
                    Ok(d) => d,
                    Err(e) => {
                        println!("  Failed to open path: {}", e);
                        continue;
                    }
                };

                // Probe slots 0x01 to 0x06
                for dev_idx in 0x01..=0x06 {
                    // Try to find Unified Battery (0x1004) first
                    let mut feature_idx = 0;
                    
                    let mut req = [0u8; 20];
                    req[0] = 0x11; // Long report
                    req[1] = dev_idx; // Device index
                    req[2] = 0x00; // Root feature index
                    req[3] = 0x00; // Function 0: GetFeature
                    req[4] = 0x10; // Unified Battery MSB
                    req[5] = 0x04; // Unified Battery LSB

                    if device.write(&req).is_ok() {
                        let mut buf = [0u8; 64];
                        if let Ok(n) = device.read_timeout(&mut buf, 100) {
                            if n >= 20 && buf[0] == 0x11 && buf[1] == dev_idx && buf[2] == 0x00 && buf[3] == 0x00 {
                                feature_idx = buf[4];
                            }
                        }
                    }

                    if feature_idx > 0 {
                        // Query Unified Battery Status
                        let mut bat_req = [0u8; 20];
                        bat_req[0] = 0x11;
                        bat_req[1] = dev_idx;
                        bat_req[2] = feature_idx;
                        bat_req[3] = 0x10; // GetStatus (Function 1 << 4)

                        if device.write(&bat_req).is_ok() {
                            let mut bat_buf = [0u8; 64];
                            if let Ok(bn) = device.read_timeout(&mut bat_buf, 100) {
                                if bn >= 20 && bat_buf[0] == 0x11 && bat_buf[1] == dev_idx && bat_buf[2] == feature_idx && bat_buf[3] == 0x10 {
                                    let pct = bat_buf[4];
                                    let charging = bat_buf[6] == 1;
                                    println!(
                                        "  [Slot {:#04x}] Reports battery: {}% (Charging: {}) [Unified Battery 0x1004]",
                                        dev_idx, pct, charging
                                    );
                                    found_any = true;

                                    let diff = (pct as i16 - expected as i16).abs();
                                    if diff <= 2 {
                                        println!(
                                            "  >>> MATCH FOUND: Slot {:#04x} reports {}% (Expected: {}%, Diff: {}) <<<",
                                            dev_idx, pct, expected, diff
                                        );
                                    }
                                }
                            }
                        }
                    } else {
                        // Try legacy Battery Level Status (0x1000)
                        let mut legacy_req = [0u8; 20];
                        legacy_req[0] = 0x11;
                        legacy_req[1] = dev_idx;
                        legacy_req[2] = 0x00;
                        legacy_req[3] = 0x00;
                        legacy_req[4] = 0x10; // Battery Status MSB
                        legacy_req[5] = 0x00; // Battery Status LSB

                        if device.write(&legacy_req).is_ok() {
                            let mut buf = [0u8; 64];
                            if let Ok(n) = device.read_timeout(&mut buf, 100) {
                                if n >= 20 && buf[0] == 0x11 && buf[1] == dev_idx && buf[2] == 0x00 && buf[3] == 0x00 {
                                    let legacy_idx = buf[4];
                                    if legacy_idx > 0 {
                                        // Query legacy status (GetBatteryLevelStatus is Function 0)
                                        let mut bat_req = [0u8; 20];
                                        bat_req[0] = 0x11;
                                        bat_req[1] = dev_idx;
                                        bat_req[2] = legacy_idx;
                                        bat_req[3] = 0x00;

                                        if device.write(&bat_req).is_ok() {
                                            if let Ok(bn) = device.read_timeout(&mut buf, 100) {
                                                if bn >= 20 && buf[0] == 0x11 && buf[1] == dev_idx && buf[2] == legacy_idx && buf[3] == 0x00 {
                                                    let pct = buf[4];
                                                    let charging = buf[5] != 0; // standard mapping varies
                                                    println!(
                                                        "  [Slot {:#04x}] Reports battery: {}% (Charging: {}) [Legacy Battery 0x1000]",
                                                        dev_idx, pct, charging
                                                    );
                                                    found_any = true;

                                                    let diff = (pct as i16 - expected as i16).abs();
                                                    if diff <= 2 {
                                                        println!(
                                                            "  >>> MATCH FOUND: Slot {:#04x} reports {}% (Expected: {}%, Diff: {}) <<<",
                                                            dev_idx, pct, expected, diff
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if !found_any {
        println!("No active battery-reporting slots found on the target device.");
    }
}
