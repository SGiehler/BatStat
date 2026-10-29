# BatStat v2 — Native Tray Flyout UI Redesign Specification

**Date:** 2026-10-29  
**Status:** Draft for User Review  
**Scope:** Complete overhaul of BatStat's desktop UI (`src/ui/`, `src/win_flyout.rs`, and window/tray wiring in `src/main.rs` and `src/config.rs`), preserving existing system tray icon rendering, cycling, and hardware polling plugins.

---

## 1. Goals & Non-Goals

### Goals
1. **Native Tray Flyout Experience:** Replace the oversized `760×720` settings dashboard with a sleek, borderless `380×490` logical-pixel flyout anchored directly above the Windows system tray icon on **single left-click**.
2. **Resource-Light Native Architecture:** Keep `eframe`/`egui` + Win32/DWM (`<15 MB` RAM, `<5 ms` popup response, `0%` CPU when hidden, zero new external crates).
3. **Telemetry-First Visual Hierarchy:** Surface peripheral battery percentages, charging states, and multi-channel readouts (*Left / Right / Case*) cleanly at a glance without KPI card clutter, redundant filter bars, or raw hardware IDs.
4. **Progressive Disclosure:** Tuck per-device configuration (rename, pause toggle, alert threshold slider, and low-battery tray icon selector) into a smooth click-to-expand inline accordion drawer (at most one device expanded at a time).
5. **Drag & Drop Everywhere It Matters:**
   - Drag the borderless flyout header to move the window anywhere on screen (auto-pinning it open).
   - Drag peripheral cards via grip handles (`⋮⋮`) to reorder devices in both the flyout and the tray context menu.
   - Drag and drop `.png` or `.ico` files onto an expanded device drawer to set custom low-battery tray icons.
6. **Configurable Icons Folder & Rich Icon Picker:** Allow choosing a custom low-battery icon per device (via visual thumbnail dropdown, 1-click color dots, file browser, or file drop) AND configuring a custom Icons Folder path in Preferences via native folder dialog (`rfd`).
7. **100% Instant Auto-Save:** Eliminate the modal *"Save & Close / Cancel"* bar; persist all changes automatically and update the tray icon live.

### Non-Goals
- Modifying HID/WGI peripheral battery query protocols (`src/plugins/*`).
- Changing the system tray icon pixel art (`ok.png`), 4×7 dynamic percentage tray icon rendering, or 3-second low-battery tray cycling logic.

---

## 2. Window, Tray & Drag Behavior

### 2.1 Single-Click Tray Toggle & Right-Click Menu
- `TrayIconBuilder` is configured with `.with_menu_on_left_click(false)`.
- **Left-Click (`MouseButton::Left`, `MouseButtonState::Up`):**
  - Captures the tray icon's physical screen `rect` from `TrayIconEvent::Click`.
  - Toggles flyout visibility: if visible, hides it; if hidden (and focus was not just lost within the last `250 ms` due to clicking the tray icon itself), positions and shows it.
- **Right-Click:** Continues to open the native Windows context menu showing online peripheral battery summaries (ordered to match `config.devices`), `Open BatStat`, and `Exit`.

### 2.2 Smart Tray-Anchored Positioning & Win11 DWM Styling
- **Viewport Configuration:** `380.0 × 490.0` logical pixels, `decorations(false)`, `taskbar(false)`, `resizable(false)`, `always_on_top(true)`, window title `"BatStat"`.
- **Win11 DWM Native Corners & Shadow (`src/win_flyout.rs`):**
  - Calls `DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &DWMWCP_ROUND, 4)` so Windows 11 renders hardware-composited anti-aliased rounded corners and native drop shadow around the borderless viewport.
  - Applies `WS_EX_TOOLWINDOW` style as belt-and-suspenders so the flyout never creates a redundant taskbar button.
- **Monitor-Aware Placement (`compute_flyout_position`):**
  - Uses `MonitorFromPoint` + `GetMonitorInfoW` (`rcWork` vs `rcMonitor`) around the tray icon center.
  - Centers horizontally over the tray icon rect, clamped within `[rcWork.left + 12px, rcWork.right - flyout_w - 12px]`.
  - Places vertically `12px` inside the taskbar-adjacent edge of `rcWork` (e.g., `rcWork.bottom - flyout_h - 12px` for bottom taskbars; adapts if taskbar is top/left/right).

### 2.3 Draggable Header, Pinning & Focus-Loss Auto-Dismiss
- **Unpinned Mode (Default):**
  - Clicking outside the flyout (`!ctx.input(|i| i.focused)`) or pressing `Esc` (when no inline rename/confirm sub-state is active) hides the flyout and records `last_focus_lost_at = Instant::now()`.
  - Opening native `rfd` file/folder dialogs sets `suppress_focus_hide = true` while the dialog is active so the flyout stays open.
- **Window Dragging (`StartDrag`) & Pinning:**
  - Dragging empty space on the top header bar invokes `egui::ViewportCommand::StartDrag` and automatically sets `is_pinned = true` + `has_custom_position = true` so moving the flyout aside never causes focus-loss dismissal.
  - Clicking the header **Pin** button toggles `is_pinned`. When unpinned, the next tray toggle re-anchors the flyout above the tray icon (`has_custom_position = false`).
  - A subtle header **Close (`✕`)** button always hides the flyout regardless of pin state.
- **Zero-Idle Loop:** When hidden (`visible == false`), `BatStatApp` does not schedule `100 ms` polling repaints; `TrayIconEvent`, `MenuEvent`, and single-instance wakeup explicitly wake `egui::Context` via `request_repaint()`.

---

## 3. UI Views & Component Specification

### 3.1 Top Header Bar (`48 px` height, Draggable)
- **Left:** Glowing emerald status dot (`#34D399` when `>= 1` device online, muted slate when idle) + **BatStat** (`15px` semibold) + subtle count label (`3 active` or `Idle`).
- **Right (Vector Icon Buttons, `28×28` hit targets):**
  1. **Rescan (`F5` / `Ctrl+R`):** Triggers immediate background peripheral poll; animates rotation for `1.5s`.
  2. **Pin Toggle:** Upright highlighted pin when pinned; angled muted pin when unpinned.
  3. **View Switch (Gear ↔ Back Arrow):** Toggles between **Devices View** and **Preferences View**. Shows a subtle green badge dot on the Gear icon if a GitHub software update is available.
  4. **Close (`✕`):** Hides the flyout.

### 3.2 Devices View (`src/ui/devices_view.rs`)
- **Empty State:** Shown only when `config.devices.is_empty()` — centered vector bolt badge, concise explanation, and a primary `Scan Peripherals` button.
- **Active / Online Peripheral Cards:**
  - Rounded card (`12px` radius, `#1C1E26` fill, `#282B36` border; low-battery state uses subtle coral-tinted surface `#251619` and `#5C232A` border).
  - **Left:** Subtle `⋮⋮` drag-reorder grip (brightens on card hover) + `36×36` rounded icon badge with custom vector peripheral glyph (Mouse, Gamepad, Earbuds, Keyboard, Dongle).
  - **Center/Right (Single-Channel):** Device name (`14px` medium), optional `Low` or `Charging ⚡` pill, right-aligned tabular percentage (`17px` bold, colored Emerald / Amber / Coral), and a sleek `6px`-tall rounded pill gauge below.
  - **Center/Right (Multi-Channel, e.g. Arctis GameBuds):** Device name + charging summary on top, followed by a compact 3-column mini-grid (`L`, `R`, `Case`) each showing label, tabular `%`, and `4px` mini pill bar.
- **Offline / Paused Peripheral Rows:**
  - Rendered below online cards under a subtle `Offline / Paused` divider (if any exist) as compact `38px` single-line rows (`⋮⋮` grip + muted dot + device name + `Sleeping` or `Paused` label) to preserve scannability. Clicking still expands the full configuration drawer.
- **Click-to-Expand Device Drawer (Accordion — 1 open at a time):**
  - Clicking a card/row header toggles `expanded_device_id: Option<String>`.
  - Inside the recessed drawer (`#111217` background, `12px` padding):
    1. **Name & Monitoring Row:** Inline editable name field (pencil/click to edit, `Enter` or focus-out commits) on the left; `Monitoring` animated toggle switch on the right.
    2. **Low Battery Alert Row:** Label `Alert below` + live tabular value (`20%`) + custom-painted smooth slider (`5%..=95%`, `1%` resolution, emerald/coral track).
    3. **Alert Tray Icon Picker Row:**
       - `24×24` live thumbnail preview of the device's effective low-battery tray icon.
       - **Visual Icon Dropdown:** Lists `Default (<Category>)` plus all `.png` and `.ico` files found in the active Icons Folder, each rendered with its `16×16` thumbnail and formatted name.
       - **Quick Color Dots:** 5 circular swatches (`White`, `Red`, `Orange`, `Yellow`, `Blue`) mapping to `None` / `low_<base>_<color>.png`.
       - **`Browse…` Button + Drag & Drop:** Opens `rfd::FileDialog` (`png`, `ico`) — or accepts files dropped onto the drawer via `ctx.input(|i| &i.raw.dropped_files)` — copying external files into the active Icons Folder and assigning `low_battery_icon_path`.
    4. **Offline Footer Action:** When the device is offline, shows a right-aligned `Remove device` link with inline 2-step confirmation (`Remove? [Cancel] [Delete]`). When online, shows subtle `Active on bus` hint.

### 3.3 Drag-and-Drop Reordering Mechanics
- Dragging a card's `⋮⋮` grip sets `dragging_device_id: Option<String>`.
- During drag, pointer Y is compared against rendered card vertical centers; crossing a neighbor swaps the two entries in `config.devices` immediately so the list reflows smoothly under the cursor.
- Releasing the pointer persists `config.toml` and calls `request_tray_update` so the tray menu order updates in sync.

### 3.4 Preferences View (`src/ui/preferences_view.rs`)
Clean single-column scroll area organized into compact grouped sections:
1. **Taskbar & Polling**
   - **Tray Battery Readout:** Dropdown to choose `Default Icon (Checkmark)` or any configured device channel (`Pulsar X2`, `GameBuds (Left)`, `GameBuds (Right)`, etc.), paired with a live `20×20` tray icon preview box.
   - **Poll Interval:** Custom 3-way segmented control (`15s` · `30s` · `60s`; displays custom value pill if edited manually in `config.toml`).
2. **System Behavior**
   - **Launch at Login:** Animated toggle calling `autostart::set_autostart` immediately on change.
   - **Low Battery Notifications:** Animated toggle + compact `Test` button when enabled.
3. **Icons & Storage**
   - **Icons Folder:** Displays active folder path (`config.custom_icons_dir` or default `%APPDATA%\BatStat\icons`).
     - `Change…` button opens `rfd::FileDialog::new().pick_folder()`, updates `config.custom_icons_dir`, seeds default icon variants into the folder if missing (without overwriting existing user files), reloads UI icon textures, and updates the tray.
     - `Open` button opens the active Icons Folder in Explorer.
     - `Reset` button (visible only when a custom folder is set) restores the default `%APPDATA%\BatStat\icons` path.
   - **Config Folder:** One-click `Open Config Directory` shortcut (`%APPDATA%\BatStat`).
4. **Updates & Diagnostics**
   - **Software Update Row:** Shows `BatStat v<CARGO_PKG_VERSION>` + live GitHub release status (`Up to date`, `Check`, `Install vX.Y.Z`, or download progress bar) + `GitHub` link.
   - **Diagnostics Accordion:** Collapsible `Debug Logs` section containing the `Diagnostic Logging` toggle, `Refresh / Copy / Clear / Open` buttons, and a `110px` monospace tail viewer for `debug.log`.

---

## 4. Data Model, Module Architecture & Persistence

### 4.1 Config Schema Addition (`src/config.rs`)
Add one backward-compatible optional field to `AppConfig`:
```rust
#[serde(default)]
pub custom_icons_dir: Option<String>,
```
- Introduce `resolve_icons_dir(config: &AppConfig) -> Option<PathBuf>` returning `custom_icons_dir` when set, otherwise `%APPDATA%\BatStat\icons`.
- Update `setup_icons_folder(icons_dir: &Path)` so built-in `.png` files and tinted color variants are only written when `!target_file.exists()`, preserving user-customized icons.
- Update tray low-battery icon lookup in `src/main.rs` to resolve relative icon filenames against `resolve_icons_dir(&s.config)`.

### 4.2 Modular File Structure & Win32 Wiring
Enable `"Win32_Graphics_Gdi"` and `"Win32_Graphics_Dwm"` features on the existing `windows-sys` dependency in `Cargo.toml` (zero new external crates; `rfd` is already present). Share a single `WINDOW_TITLE = "BatStat"` constant across `ViewportBuilder`, `find_our_window()`, and single-instance wakeup in `src/main.rs`.

Replace the 2,960-line `src/ui.rs` with a clean `src/ui/` module tree plus `src/win_flyout.rs`:
- `src/win_flyout.rs` — Win32 DWM corner preference (`DWMWCP_ROUND`), `WS_EX_TOOLWINDOW`, and monitor work-area flyout coordinate calculation (`compute_flyout_position`).
- `src/ui/mod.rs` — `SettingsWindow` (flyout state machine, shortcuts, file-drop handling, auto-save coordinator, toast overlay, `eframe::App` implementation).
- `src/ui/theme.rs` — Obsidian color tokens, Windows `Segoe UI Variable` / `Segoe UI` + `Cascadia Mono` / `Consolas` font loader, and pure-vector icon drawing functions.
- `src/ui/widgets.rs` — Reusable custom-painted widgets (`toggle_switch`, `pill_battery_bar`, `smooth_threshold_slider`, `segmented_control`, `icon_button`, `color_dot`).
- `src/ui/devices_view.rs` — Online cards, multi-channel sub-grids, compact offline rows, drag-to-reorder logic, and the expandable per-device configuration drawer + icon picker.
- `src/ui/preferences_view.rs` — Tray readout selector, poll interval, autostart/notifications, configurable Icons Folder (`rfd`), updater card, and collapsible log viewer.

### 4.3 Auto-Save & Error Handling Flow
- `AutoSaveState` tracks `save_immediately: bool` and `save_on_release: bool`.
- Discrete actions (`toggle_switch`, swatch click, dropdown pick, rename commit, device delete, reorder drop, folder change) trigger immediate `save_config(&self.config)` + `config_saved_in_place = true` (which syncs `SharedState` and wakes the tray thread).
- Continuous drags (threshold slider) update `self.config` in-memory each frame and flush to disk once `!ctx.input(|i| i.pointer.any_down())` or when the flyout closes.
- Any I/O or OS error (`save_config`, `set_autostart`, icon copy, updater failure) displays a non-blocking `2.5s` pill toast at the bottom of the flyout.

---

## 5. Verification & Testing Strategy

1. **Unit Tests (`cargo test`):**
   - `config.rs`: Serialization/deserialization backward compatibility with and without `custom_icons_dir`, `resolve_icons_dir` fallback, non-destructive icon seeding.
   - `win_flyout.rs`: Pure coordinate math tests for `compute_flyout_position` across bottom, top, left, and right taskbars, edge clamping near screen corners, and `100%` / `150%` / `200%` DPI scale factors.
   - `ui/*`: Device category & icon label formatting, multi-channel tray option generation, drag-reorder swap invariants, single-open accordion state, Escape key hierarchy (cancel rename → cancel delete confirm → unexpand/close flyout), auto-save flush on pointer release, and headless `egui::Context::run` rendering passes for Empty, Online/Offline Devices, Expanded Drawer, and Preferences + Logs views.
2. **Release Build & Live Smoke Check:**
   - Verify `cargo build` and `cargo test` pass with zero warnings.
