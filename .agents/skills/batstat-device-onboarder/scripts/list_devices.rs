use hidapi::HidApi;

fn main() {
    println!("Scanning for HID devices...");
    match HidApi::new() {
        Ok(api) => {
            let mut devices: Vec<_> = api.device_list().collect();
            // Sort by vendor ID, then product ID, then interface number
            devices.sort_by_key(|d| (d.vendor_id(), d.product_id(), d.interface_number()));

            for dev in devices {
                let manufacturer = dev.manufacturer_string().unwrap_or("Unknown");
                let product = dev.product_string().unwrap_or("Unknown");
                println!(
                    "VID: {:#06x} | PID: {:#06x} | IF: {:2} | Page: {:#06x} | Usage: {:#06x} | {} - {} | Path: {:?}",
                    dev.vendor_id(),
                    dev.product_id(),
                    dev.interface_number(),
                    dev.usage_page(),
                    dev.usage(),
                    manufacturer,
                    product,
                    dev.path()
                );
            }
        }
        Err(e) => {
            eprintln!("Failed to initialize HidApi: {}", e);
        }
    }
}
