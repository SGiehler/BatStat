# BatStat ⚡

BatStat is a lightweight, native Windows system tray battery monitor for wireless gaming peripherals, written in Rust. It runs unobtrusively in the background, alerts you when devices are running low, and provides a sleek settings UI to customize thresholds and startup behavior.

## Features

- 🔋 **Multi-Channel Support**: Dynamically queries and renders multi-channel battery levels (e.g., Left and Right earbud percentages, plus charging case support).
- 🔄 **Smart Tray Cycling**: When multiple devices are low, the tray icon and its tooltip automatically cycle between them at a 3-second interval.
- ⚙️ **Premium Settings UI**: A modern dark-mode GUI built with `egui` and `eframe` to configure alert thresholds, toggle notifications, and set custom icons.
- 🚀 **Autostart Integration**: Option to automatically launch BatStat with Windows via registry setup.
- 🔔 **Windows Alerts**: Real-time desktop notifications when a device reaches its alert threshold.

## Supported Hardware

BatStat uses a modular plugin architecture to scan and query devices. For a complete list of technical specifications, vendor/product IDs, and connection types, see the [Supported Devices List](DEVICELIST.md).

1. **Pulsar Wireless Gaming Mice** (via custom HID queries).
2. **Xbox Controllers** (via modern Windows Gaming Input [WGI] APIs for precise percentages).
3. **SteelSeries Arctis GameBuds** (via dual-channel Left/Right HID queries).

## Getting Started

### Prerequisites

- [Rust Toolchain](https://rustup.rs/) (stable version).
- Windows 10/11 (for WinRT WGI APIs and notification center support).

### Building from Source

Clone the repository and build the release binary:

```bash
git clone https://github.com/SGiehler/BatStat.git
cd BatStat
cargo build --release
```

The compiled binary will be located at `target/release/batstat.exe`.

## License

This project is licensed under the [Creative Commons Attribution-NonCommercial-ShareAlike 4.0 International License](LICENSE).
