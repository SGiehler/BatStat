# Supported Devices List

Below is a detailed list of the devices currently supported by BatStat, along with their respective connection APIs, vendor/product IDs, and battery channels.

---

## 🖱️ Pulsar Wireless Gaming Mice

- **API / Protocol**: Direct HID commands (HIDAPI)
- **Interface Filter**: Usage page $\ge$ `0xFF00` (config/telemetry interface)
- **Vendor IDs**:
  - `0x3554`
  - `0x3710`
- **Supported Channels**: Single channel (`Main`)
- **Reported Data**: Exact battery percentage and charging state.

---

## 🎮 Xbox Controllers

- **API / Protocol**: `Windows.Gaming.Input` (WGI) WinRT API
- **Supported Controllers**: Xbox One, Xbox Series X/S, and Xbox Elite wireless gamepads.
- **Supported Channels**: Single channel (`Main`)
- **Reported Data**: Exact battery capacity percentage (derived from `RemainingCapacityInMilliwattHours` / `FullChargeCapacityInMilliwattHours`), charging state, and connection presence.

---

## 🎧 SteelSeries Arctis GameBuds

- **API / Protocol**: Direct HID commands (HIDAPI)
- **Interface Filter**: Usage page `0xFFC0` or Interface Number `3`
- **Vendor ID**: `0x1038` (SteelSeries)
- **Product ID**: `0x230a` (Arctis GameBuds)
- **Supported Channels**: Dual-channel (`Left` earbud & `Right` earbud)
- **Reported Data**: Dynamic connection detection (online/offline status per earbud), individual battery percentages, and charging states.

---

## 🖱️ Logitech Gaming Mice (HID++ 2.0)

- **API / Protocol**: Direct HID commands (HIDAPI / HID++ 2.0 Protocol)
- **Interface Filter**: Usage page `0xFF00`, Usage `0x0002` (custom vendor interface)
- **Vendor ID**: `0x046d` (Logitech)
- **Supported Channels**: Single channel (`Main`)
- **Reported Data**: Exact battery percentage and charging state via the Unified Battery (`0x1004`) feature. Automatically falls back to last known battery state when the device is sleeping or turned off.

