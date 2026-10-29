# BatStat v2 Native Tray Flyout UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace BatStat's legacy `760×720` settings dashboard with a borderless `380×490` Windows 11 DWM tray flyout featuring single-click tray toggle, window & device drag-and-drop, progressive-disclosure device drawers, configurable custom icons directory, and live auto-save.

**Architecture:** Modularize the UI into focused `src/ui/*` submodules (`theme`, `widgets`, `devices_view`, `preferences_view`, `mod`) backed by a dedicated Win32/DWM positioning & window-styling module (`src/win_flyout.rs`) and non-destructive custom icon folder management in `src/config.rs`. All controls are custom-painted on `eframe`/`egui` (`0.29`) with zero WebView overhead, keeping memory `<15 MB` and hidden idle CPU at `0%`.

**Tech Stack:** Rust 2021, `eframe` / `egui` `0.29.1`, `tray-icon` `0.19`, `windows-sys` `0.59` (`Win32_UI_WindowsAndMessaging`, `Win32_Graphics_Gdi`, `Win32_Graphics_Dwm`), `rfd` `0.15`, `image` `0.25`, `serde` / `toml`.

**Spec:** `docs/superpowers/specs/2026-10-29-tray-flyout-ui-redesign.md`

## Global Constraints

- **Zero New External Crates:** Use only crates already present in `Cargo.toml` (`eframe`, `tray-icon`, `windows-sys`, `rfd`, `image`, `serde`, `toml`, `notify-rust`, `ureq`, `chrono`); only add `"Win32_Graphics_Gdi"` and `"Win32_Graphics_Dwm"` feature flags to `windows-sys`.
- **Untouched Hardware & Tray Pixel Art:** Do not alter `src/plugins/*`, `src/icons/*.png` pixel art, `render_percentage_icon` 4×7 bitmap rendering, or 3-second low-battery tray cycling.
- **Viewport & Window Identity:** Logical viewport size `380.0 × 490.0`, `decorations(false)`, `taskbar(false)`, `resizable(false)`, `always_on_top(true)`, shared `WINDOW_TITLE = "BatStat"`.
- **Resource Footprint:** `<15 MB` RAM; no periodic `request_repaint_after` polling loop while hidden (`visible == false`).
- **100% Auto-Save:** No Save/Cancel footer bar; discrete edits persist immediately, continuous slider drags persist on pointer release or flyout hide.

## Review Focus

1. **Tray left-click focus-loss race:** Clicking the tray icon while the flyout is open steals Win32 window focus ~30–80 ms before `MouseButtonState::Up` arrives; `should_open_on_tray_click` must return `false` when focus was lost `< 250 ms` ago so clicking the tray icon closes rather than re-opens the flyout (pinned in Task 2).
2. **Negative multi-monitor coordinates:** Secondary monitors positioned left of or above the primary display have negative `rcWork` coordinates; `compute_flyout_position` must clamp strictly against `work_rect` bounds rather than `0` (pinned in Task 2).
3. **Custom Icons Folder preservation:** Seeding default icon variants into a user-configured `custom_icons_dir` (or `%APPDATA%\BatStat\icons`) must never overwrite existing files such as a customized `ok.png` or `low_mouse_red.png` (pinned in Task 1).
4. **Dropped/imported icon validation:** Dropping or selecting an unsupported file extension (e.g., `.txt` or `.jpg`) must return a clean `Err` without mutating `low_battery_icon_path`, whereas valid `.png`/`.ico` files outside the active icons directory are copied in and referenced by filename (pinned in Task 1).
5. **Interrupted slider drag auto-save:** Closing or hiding the flyout (`Esc` or focus loss) while `save_on_release` is pending must flush unsaved threshold changes before teardown so edits are never lost (pinned in Task 4).

---

### Task 1: Configurable Icons Directory, Non-Destructive Seeding & Icon Import

**Files:**
- Modify: `Cargo.toml:14-23`
- Modify: `src/config.rs:14-184`

**Interfaces:**
- Produces:
  - `AppConfig::custom_icons_dir: Option<String>` (`#[serde(default)]`)
  - `pub fn resolve_icons_dir(config: &AppConfig) -> Option<PathBuf>`
  - `pub fn setup_icons_folder_at(dir: &std::path::Path)`
  - `pub fn get_icon_list_for(config: &AppConfig) -> Vec<String>`
  - `pub fn import_icon_file(src: &std::path::Path, icons_dir: &std::path::Path) -> Result<String, String>`

- [ ] **Step 1: Add Win32 GDI & DWM features in `Cargo.toml` and write failing config/icon tests in `src/config.rs`**

Add `"Win32_Graphics_Gdi"` and `"Win32_Graphics_Dwm"` to `windows-sys` features in `Cargo.toml`. In `src/config.rs` `mod tests`, add:

```rust
#[test]
fn test_custom_icons_dir_resolution_and_serde_backcompat() {
    let legacy_toml = r#"
        polling_interval_secs = 60
        autostart = false
        devices = []
    "#;
    let mut cfg: super::AppConfig = toml::from_str(legacy_toml).unwrap();
    assert_eq!(cfg.custom_icons_dir, None);
    assert_eq!(super::resolve_icons_dir(&cfg), super::get_icons_dir_path());

    cfg.custom_icons_dir = Some("C:\\CustomIcons\\BatStat".to_string());
    assert_eq!(
        super::resolve_icons_dir(&cfg),
        Some(std::path::PathBuf::from("C:\\CustomIcons\\BatStat"))
    );
}

#[test]
fn test_setup_icons_folder_seeds_missing_without_overwriting_existing() {
    let temp = std::env::temp_dir().join(format!("batstat_test_icons_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp).unwrap();

    let custom_bytes = b"custom-user-icon-bytes";
    std::fs::write(temp.join("low_mouse_red.png"), custom_bytes).unwrap();

    super::setup_icons_folder_at(&temp);

    // Existing customized file must NOT be overwritten
    assert_eq!(std::fs::read(temp.join("low_mouse_red.png")).unwrap(), custom_bytes);
    // Missing built-ins and variants must be seeded
    assert!(temp.join("ok.png").exists());
    assert!(temp.join("low_gamepad_blue.png").exists());

    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn test_import_custom_icon_validates_extension_and_copies_to_icons_dir() {
    let temp = std::env::temp_dir().join(format!("batstat_import_test_{}", std::process::id()));
    let src_dir = temp.join("external");
    let icons_dir = temp.join("icons");
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&icons_dir).unwrap();

    let bad_file = src_dir.join("notes.txt");
    std::fs::write(&bad_file, b"nope").unwrap();
    assert!(super::import_icon_file(&bad_file, &icons_dir).is_err());

    let good_file = src_dir.join("my_mouse.PNG");
    std::fs::write(&good_file, b"png-data").unwrap();
    let imported = super::import_icon_file(&good_file, &icons_dir).unwrap();
    assert_eq!(imported, "my_mouse.PNG");
    assert!(icons_dir.join("my_mouse.PNG").exists());

    let _ = std::fs::remove_dir_all(&temp);
}
```

- [ ] **Step 2: Run `cargo test` on `config::tests` to verify compilation failure**

Run: `cargo test config::tests`
Expected: FAIL (`custom_icons_dir`, `resolve_icons_dir`, `setup_icons_folder_at`, `import_icon_file` not defined).

- [ ] **Step 3: Implement `custom_icons_dir` and icon folder helpers in `src/config.rs`**

Add `#[serde(default)] pub custom_icons_dir: Option<String>` to `AppConfig` (initialized to `None` in `Default`).
Implement:
- `resolve_icons_dir(config: &AppConfig) -> Option<PathBuf>`: returns `Some(PathBuf::from(trimmed))` when `config.custom_icons_dir` has a non-empty trimmed string, otherwise `get_icons_dir_path()`.
- `setup_icons_folder_at(dir: &Path)`: creates `dir` if missing and writes each default (`ok.png`, `low_*.png`) and tinted variant (`red`, `orange`, `yellow`, `blue`) **only when `!path.exists()`**. Keep `setup_icons_folder()` delegating to `get_icons_dir_path()`.
- `get_icon_list_for(config: &AppConfig) -> Vec<String>`: lists sorted `.png`/`.ico` filenames (case-insensitive extension check, excluding `ok.png`) from `resolve_icons_dir(config)`. Keep `get_icon_list()` delegating to `AppConfig::default()`.
- `import_icon_file(src: &Path, icons_dir: &Path) -> Result<String, String>`: validates `.png` or `.ico` extension case-insensitively, creates `icons_dir` if needed, copies `src` to `icons_dir.join(file_name)` unless `src` already resolves to that exact path, and returns the filename string.

- [ ] **Step 4: Run `cargo test config::tests` to verify all tests pass**

Run: `cargo test config::tests`
Expected: PASS (all existing and new `config::tests` green).

- [ ] **Step 5: Commit Task 1**

```powershell
git add Cargo.toml src/config.rs
git commit -m "feat(config): add custom_icons_dir, non-destructive icon seeding, and icon import"
```

---

### Task 2: Win11 DWM Flyout Styling, Monitor-Aware Positioning & Tray Toggle Timing

**Files:**
- Create: `src/win_flyout.rs`
- Modify: `src/main.rs:1-7` (add `mod win_flyout;`)

**Interfaces:**
- Produces:
  - `pub const WINDOW_TITLE: &str = "BatStat";`
  - `pub const FLYOUT_WIDTH: f32 = 380.0;`
  - `pub const FLYOUT_HEIGHT: f32 = 490.0;`
  - `pub const TASKBAR_GAP_PX: i32 = 12;`
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub struct ScreenRect { pub left: i32, pub top: i32, pub right: i32, pub bottom: i32 }`
  - `pub fn compute_flyout_position(tray_rect: Option<ScreenRect>, cursor_pos: Option<(i32, i32)>, monitor_rect: ScreenRect, work_rect: ScreenRect, flyout_phys_size: (i32, i32)) -> (i32, i32)`
  - `pub fn should_open_on_tray_click(currently_visible: bool, last_focus_lost: Option<std::time::Instant>, now: std::time::Instant) -> bool`
  - `pub fn find_flyout_window() -> Option<windows_sys::Win32::Foundation::HWND>`
  - `pub fn apply_flyout_window_style(hwnd: windows_sys::Win32::Foundation::HWND)`
  - `pub fn position_and_show_flyout(hwnd: windows_sys::Win32::Foundation::HWND, tray_rect: Option<ScreenRect>, cursor_pos: Option<(i32, i32)>, scale_factor: f32, preserve_position: bool)`
  - `pub fn hide_flyout(hwnd: windows_sys::Win32::Foundation::HWND)`

- [ ] **Step 1: Create `src/win_flyout.rs` with failing unit tests and register `mod win_flyout;` in `src/main.rs`**

Add `mod win_flyout;` in `src/main.rs`. Create `src/win_flyout.rs` with stub signatures and unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn test_bottom_taskbar_centers_over_tray_and_clamps_right_edge() {
        let mon = ScreenRect { left: 0, top: 0, right: 1920, bottom: 1080 };
        let work = ScreenRect { left: 0, top: 0, right: 1920, bottom: 1032 };
        let tray = ScreenRect { left: 1840, top: 1032, right: 1872, bottom: 1080 };
        let (x, y) = compute_flyout_position(Some(tray), None, mon, work, (380, 490));
        // Centered at 1856 -> 1856 - 190 = 1666, which overflows 1920 - 380 - 12 = 1528 -> clamped to 1528
        assert_eq!(x, 1528);
        assert_eq!(y, 1032 - 490 - 12);
    }

    #[test]
    fn test_top_and_side_taskbars_anchor_to_correct_work_edge() {
        let mon = ScreenRect { left: 0, top: 0, right: 1920, bottom: 1080 };
        // Top taskbar (48px at top)
        let work_top = ScreenRect { left: 0, top: 48, right: 1920, bottom: 1080 };
        let tray_top = ScreenRect { left: 1700, top: 0, right: 1732, bottom: 48 };
        let (_, y_top) = compute_flyout_position(Some(tray_top), None, mon, work_top, (380, 490));
        assert_eq!(y_top, 48 + 12);

        // Left taskbar (72px on left)
        let work_left = ScreenRect { left: 72, top: 0, right: 1920, bottom: 1080 };
        let tray_left = ScreenRect { left: 0, top: 900, right: 72, bottom: 932 };
        let (x_left, _) = compute_flyout_position(Some(tray_left), None, mon, work_left, (380, 490));
        assert_eq!(x_left, 72 + 12);
    }

    #[test]
    fn test_flyout_position_negative_secondary_monitor_coords() {
        let mon = ScreenRect { left: -1920, top: 0, right: 0, bottom: 1080 };
        let work = ScreenRect { left: -1920, top: 0, right: 0, bottom: 1032 };
        let tray = ScreenRect { left: -220, top: 1032, right: -188, bottom: 1080 };
        let (x, y) = compute_flyout_position(Some(tray), None, mon, work, (380, 490));
        assert!(x >= -1920 + 12 && x + 380 <= -12);
        assert_eq!(y, 1032 - 490 - 12);
    }

    #[test]
    fn test_tray_toggle_suppresses_reopen_after_recent_focus_loss() {
        let t0 = Instant::now();
        assert!(!should_open_on_tray_click(true, None, t0));
        assert!(!should_open_on_tray_click(false, Some(t0), t0 + Duration::from_millis(80)));
        assert!(should_open_on_tray_click(false, Some(t0), t0 + Duration::from_millis(300)));
        assert!(should_open_on_tray_click(false, None, t0));
    }
}
```

- [ ] **Step 2: Run `cargo test win_flyout::tests` to verify failure**

Run: `cargo test win_flyout::tests`
Expected: FAIL on stub assertions.

- [ ] **Step 3: Implement geometry, debounce, DWM rounded corners & Win32 placement in `src/win_flyout.rs`**

Implement:
- `compute_flyout_position`: detects taskbar edge by comparing `work_rect` vs `monitor_rect` (`work.top > mon.top` => top taskbar, `work.left > mon.left` => left taskbar, `work.right < mon.right` => right taskbar, default => bottom taskbar). Uses `tray_rect` center if valid (`right > left`), else `cursor_pos`, else bottom-right of `work_rect`, clamping `(x, y)` inside `[work_rect.left + 12, work_rect.right - w - 12]` and `[work_rect.top + 12, work_rect.bottom - h - 12]`.
- `should_open_on_tray_click`: returns `!currently_visible && last_focus_lost.map_or(true, |t| now.saturating_duration_since(t) >= Duration::from_millis(250))`.
- `apply_flyout_window_style`: calls `DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE as u32, &DWMWCP_ROUND as *const _ as _, 4)` and updates `GWL_EXSTYLE` to set `WS_EX_TOOLWINDOW` and clear `WS_EX_APPWINDOW`.
- `position_and_show_flyout`: queries `MonitorFromPoint(anchor_pt, MONITOR_DEFAULTTONEAREST)` + `GetMonitorInfoW`; when `!preserve_position`, moves window to `compute_flyout_position` via `SetWindowPos` (`SWP_NOSIZE | SWP_NOZORDER | SWP_SHOWWINDOW`), else calls `ShowWindow(hwnd, SW_SHOW)`; then calls `SetForegroundWindow(hwnd)`.

- [ ] **Step 4: Run `cargo test win_flyout::tests` to verify all tests pass**

Run: `cargo test win_flyout::tests`
Expected: PASS (4/4 tests green).

- [ ] **Step 5: Commit Task 2**

```powershell
git add src/win_flyout.rs src/main.rs
git commit -m "feat(flyout): add Win11 DWM rounded flyout styling and monitor-aware tray positioning"
```

---

### Task 3: Obsidian Theme, Pure-Vector Icon Painter & Custom Flyout Widgets

**Files:**
- Move: `src/ui.rs` -> `src/ui/mod.rs` (initial module root)
- Create: `src/ui/theme.rs`
- Create: `src/ui/widgets.rs`

**Interfaces:**
- Produces in `src/ui/theme.rs`:
  - Obsidian color constants (`BG_FLYOUT`, `BORDER_FLYOUT`, `BG_CARD`, `BG_CARD_HOVER`, `BG_DRAWER`, `BG_LOW_CARD`, `BORDER_LOW_CARD`, `ACCENT_ONLINE`, `ACCENT_CHARGING`, `ACCENT_LOW`, `TEXT_PRIMARY`, `TEXT_SECONDARY`, `TEXT_MUTED`, `TEXT_DIM`)
  - `pub enum DeviceKind { Mouse, Gamepad, Buds, Keyboard, Peripheral }`
  - `pub enum ActionIcon { Refresh, Pin { pinned: bool }, Gear, Back, Close, Folder, Grip }`
  - `pub fn configure_flyout_fonts(ctx: &egui::Context)`
  - `pub fn apply_flyout_visuals(ctx: &egui::Context)`
  - `pub fn draw_device_icon(painter: &egui::Painter, rect: egui::Rect, kind: DeviceKind, color: egui::Color32)`
  - `pub fn draw_action_icon(painter: &egui::Painter, rect: egui::Rect, icon: ActionIcon, color: egui::Color32, spin_angle: f32)`
- Produces in `src/ui/widgets.rs`:
  - `pub fn toggle_switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response`
  - `pub fn pill_battery_bar(ui: &mut egui::Ui, percentage: u8, charging: bool, threshold: u8, width: f32, height: f32) -> egui::Response`
  - `pub fn smooth_threshold_slider(ui: &mut egui::Ui, value: &mut u8, width: f32) -> egui::Response`
  - `pub fn segmented_poll_selector(ui: &mut egui::Ui, secs: &mut u64) -> bool`
  - `pub fn header_icon_button(ui: &mut egui::Ui, icon: ActionIcon, active: bool, badge_dot: bool, spin_angle: f32, tooltip: &str) -> egui::Response`
  - `pub fn color_swatch_dot(ui: &mut egui::Ui, color: egui::Color32, selected: bool, tooltip: &str) -> egui::Response`

- [ ] **Step 1: Move `src/ui.rs` to `src/ui/mod.rs` and write failing headless widget tests in `src/ui/widgets.rs`**

Move `src/ui.rs` to `src/ui/mod.rs`, declare `pub mod theme; pub mod widgets;`, and add tests in `src/ui/widgets.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::{ActionIcon, DeviceKind, draw_device_icon};

    #[test]
    fn test_custom_widgets_render_without_panic_and_clamp_bounds() {
        let ctx = egui::Context::default();
        let mut enabled = true;
        let mut threshold = 20u8;
        let mut poll = 60u64;

        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let _ = toggle_switch(ui, &mut enabled);
                let _ = pill_battery_bar(ui, 88, false, 20, 200.0, 6.0);
                let _ = pill_battery_bar(ui, 14, true, 20, 120.0, 4.0);
                let _ = smooth_threshold_slider(ui, &mut threshold, 220.0);
                let _ = segmented_poll_selector(ui, &mut poll);
                let _ = header_icon_button(ui, ActionIcon::Pin { pinned: true }, true, false, 0.0, "Pin");
                let _ = color_swatch_dot(ui, egui::Color32::RED, true, "Red");

                let (r, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
                for kind in [DeviceKind::Mouse, DeviceKind::Gamepad, DeviceKind::Buds, DeviceKind::Keyboard, DeviceKind::Peripheral] {
                    draw_device_icon(ui.painter(), r, kind, egui::Color32::WHITE);
                }
            });
        });

        assert!((5..=95).contains(&threshold));
    }
}
```

- [ ] **Step 2: Run `cargo test ui::widgets::tests` to verify failure**

Run: `cargo test ui::widgets::tests`
Expected: FAIL (`theme` / `widgets` functions missing).

- [ ] **Step 3: Implement `src/ui/theme.rs` and `src/ui/widgets.rs`**

Implement Obsidian tokens (`BG_FLYOUT = #14151A`, `BG_CARD = #1C1E26`, `BG_DRAWER = #111217`, `ACCENT_ONLINE = #34D399`, `ACCENT_CHARGING = #FBBF24`, `ACCENT_LOW = #F87171`), font setup (`SegUIVar.ttf` -> `segoeui.ttf` proportional; `CascadiaMono.ttf` -> `consola.ttf` monospace), anti-aliased `egui::Painter` vector shapes for all `DeviceKind` and `ActionIcon` variants (rotating `Refresh` by `spin_angle`), and all 6 sleek custom widgets (`smooth_threshold_slider` uses `Sense::click_and_drag()` mapping pointer X linearly onto `5..=95u8` with `4px` rounded track and `14px` circular thumb).

- [ ] **Step 4: Run `cargo test ui::widgets::tests` to verify pass**

Run: `cargo test ui::widgets::tests`
Expected: PASS.

- [ ] **Step 5: Commit Task 3**

```powershell
git add src/ui
git commit -m "feat(ui): add Obsidian flyout theme, vector icon painter, and custom widgets"
```

---

### Task 4: Flyout Views, Drag-to-Reorder, Expandable Icon Drawer & Auto-Save Shell

**Files:**
- Create: `src/ui/devices_view.rs`
- Create: `src/ui/preferences_view.rs`
- Modify: `src/ui/mod.rs` (replace legacy dashboard with `380×490` Flyout shell)

**Interfaces:**
- Consumes: `crate::config::{resolve_icons_dir, setup_icons_folder_at, get_icon_list_for, import_icon_file}`, `crate::ui::theme::*`, `crate::ui::widgets::*`
- Produces in `src/ui/mod.rs` (preserving `main.rs` integration fields plus new flyout state):
  - `pub enum FlyoutView { Devices, Preferences }`
  - `pub fn get_device_category(unique_id: &str) -> (&'static str, DeviceKind, &'static str, &'static str)`
  - `pub fn reorder_devices(devices: &mut Vec<DeviceConfig>, from_idx: usize, to_idx: usize) -> bool`
  - `SettingsWindow` fields: `config`, `active_devices`, `device_statuses`, `update_status`, `view: FlyoutView`, `expanded_device_id: Option<String>`, `dragging_device_id: Option<String>`, `is_pinned: bool`, `has_custom_position: bool`, `suppress_focus_hide: bool`, `save_immediately: bool`, `save_on_release: bool`, `config_saved_in_place: bool`, `device_removed: bool`, `request_close: bool`, `request_poll: bool`, `request_test_notification: bool`, `request_update_check: bool`, `request_download_install: Option<String>`
  - `SettingsWindow::flush_autosave(&mut self)` (saves `config.toml` and sets `config_saved_in_place = true` when dirty)

- [ ] **Step 1: Write failing unit & headless rendering tests for the new Flyout UI in `src/ui/mod.rs`**

Replace legacy dashboard tests in `src/ui/mod.rs` with tests covering the spec and Review Focus #5:

```rust
#[test]
fn test_reorder_devices_preserves_elements_and_updates_priority() {
    let mut devs = vec![
        DeviceConfig { unique_id: "a".into(), name: "A".into(), enabled: true, threshold: 20, low_battery_icon_path: None },
        DeviceConfig { unique_id: "b".into(), name: "B".into(), enabled: true, threshold: 20, low_battery_icon_path: None },
        DeviceConfig { unique_id: "c".into(), name: "C".into(), enabled: true, threshold: 20, low_battery_icon_path: None },
    ];
    assert!(reorder_devices(&mut devs, 2, 0));
    let ids: Vec<&str> = devs.iter().map(|d| d.unique_id.as_str()).collect();
    assert_eq!(ids, vec!["c", "a", "b"]);
}

#[test]
fn test_autosave_flushes_pending_slider_change_on_close() {
    let mut cfg = AppConfig::default();
    cfg.devices.push(DeviceConfig {
        unique_id: "pulsar_1".into(),
        name: "Pulsar X2".into(),
        enabled: true,
        threshold: 20,
        low_battery_icon_path: None,
    });
    let mut win = SettingsWindow::new(cfg, vec![], std::collections::HashMap::new());
    win.config.devices[0].threshold = 35;
    win.save_on_release = true;
    win.flush_autosave();
    assert!(!win.save_on_release);
    assert!(win.config_saved_in_place);
}

#[test]
fn test_escape_hierarchy_in_flyout() {
    let cfg = AppConfig::default();
    let mut win = SettingsWindow::new(cfg, vec![], std::collections::HashMap::new());
    win.expanded_device_id = Some("pulsar_1".into());
    win.editing_device_id = Some("pulsar_1".into());

    win.handle_escape_press();
    assert!(win.editing_device_id.is_none());
    assert!(win.expanded_device_id.is_some());
    assert!(!win.request_close);

    win.handle_escape_press();
    assert!(win.expanded_device_id.is_none());
    assert!(!win.request_close);

    win.handle_escape_press();
    assert!(win.request_close);
}

#[test]
fn test_flyout_renders_devices_expanded_drawer_and_preferences_views() {
    let ctx = egui::Context::default();
    let mut cfg = AppConfig::default();
    cfg.devices.push(DeviceConfig {
        unique_id: "gamebuds".into(),
        name: "Arctis GameBuds".into(),
        enabled: true,
        threshold: 15,
        low_battery_icon_path: None,
    });
    cfg.devices.push(DeviceConfig {
        unique_id: "keyboard_1".into(),
        name: "Keychron K2".into(),
        enabled: true,
        threshold: 20,
        low_battery_icon_path: None,
    });
    let mut statuses = std::collections::HashMap::new();
    statuses.insert("gamebuds".into(), DeviceBatteryStatus::Online {
        channels: [
            Some(BatteryChannel { channel_type: ChannelType::Left, percentage: 92, charging: false }),
            Some(BatteryChannel { channel_type: ChannelType::Right, percentage: 85, charging: true }),
            Some(BatteryChannel { channel_type: ChannelType::Case, percentage: 100, charging: false }),
            None,
        ],
    });
    let mut win = SettingsWindow::new(cfg, vec!["gamebuds".into()], statuses);
    win.expanded_device_id = Some("gamebuds".into());

    let _ = ctx.run(egui::RawInput::default(), |ctx| win.render_flyout(ctx));
    win.view = FlyoutView::Preferences;
    win.logs_expanded = true;
    let _ = ctx.run(egui::RawInput::default(), |ctx| win.render_flyout(ctx));
}
```

- [ ] **Step 2: Run `cargo test ui::tests` to verify failure**

Run: `cargo test ui::tests`
Expected: FAIL (`FlyoutView`, `reorder_devices`, `flush_autosave`, `render_flyout` missing).

- [ ] **Step 3: Implement `src/ui/devices_view.rs`, `src/ui/preferences_view.rs`, and the Flyout shell in `src/ui/mod.rs`**

- `src/ui/devices_view.rs`:
  - Renders empty state when `config.devices.is_empty()`.
  - Splits devices into Online/Enabled cards first (preserving `config.devices` order) and compact `38px` Offline/Paused rows below a subtle divider.
  - Each item's `⋮⋮` grip supports drag-to-reorder across `config.devices` via `reorder_devices`, triggering auto-save on pointer release.
  - Clicking an item toggles `expanded_device_id` (1 open at a time) revealing: inline rename + monitoring toggle, `smooth_threshold_slider` (`save_on_release = true`), **Alert Tray Icon** row (`24×24` preview + thumbnail `ComboBox` of `get_icon_list_for(&self.config)` + 5 `color_swatch_dot` presets + `Browse…` button calling `rfd::FileDialog` guarded by `suppress_focus_hide` + dropped file handler via `import_icon_file`), and 2-step offline `Remove device`.
- `src/ui/preferences_view.rs`:
  - Renders **Taskbar & Polling** (`tray_battery_channel` dropdown + `20×20` live preview + `segmented_poll_selector`), **System Behavior** (`Launch at Login` + `Low Battery Notifications` + `Test`), **Icons & Storage** (active path from `resolve_icons_dir`, `Change…` via `rfd::FileDialog::new().pick_folder()` guarded by `suppress_focus_hide` -> calls `setup_icons_folder_at`, clears `icon_textures` to reload, saves immediately; `Open` in Explorer; `Reset` when `custom_icons_dir.is_some()`; `Config Folder` button), and **Updates & Diagnostics** (GitHub release status/install button + collapsible `Debug Logs` drawer).
- `src/ui/mod.rs`:
  - Renders outer `1px` `BORDER_FLYOUT` frame, `48px` draggable header (`Sense::click_and_drag()` -> `ViewportCommand::StartDrag` + `is_pinned = true` + `has_custom_position = true` on `drag_started()`), action buttons (`Refresh`, `Pin`, `Gear`/`Back` with update dot, `Close`), scrollable active view, `2.5s` toast pill, and auto-save coordinator (`ensure_textures` loads from `resolve_icons_dir(&self.config)`).

- [ ] **Step 4: Run `cargo test` to verify all UI and project tests pass**

Run: `cargo test`
Expected: PASS (all `config`, `win_flyout`, `ui::widgets`, and `ui::tests` green).

- [ ] **Step 5: Commit Task 4**

```powershell
git add src/ui
git commit -m "feat(ui): implement tray flyout views, drag reorder, icon drawer, and auto-save"
```

---

### Task 5: Single-Click Tray Toggle, Focus-Loss Auto-Dismiss & Ordered Tray Menu Wiring

**Files:**
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `crate::win_flyout::*`, `crate::config::{resolve_icons_dir, setup_icons_folder_at}`, `crate::ui::SettingsWindow`

- [ ] **Step 1: Write unit test in `src/main.rs` verifying tray menu device ordering & custom icon path resolution**

Add a `#[cfg(test)] mod tests` block at the bottom of `src/main.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, DeviceConfig};

    #[test]
    fn test_resolve_device_low_icon_path_uses_custom_icons_dir() {
        let mut cfg = AppConfig::default();
        cfg.custom_icons_dir = Some("D:\\MyIcons".to_string());
        let dev = DeviceConfig {
            unique_id: "pulsar_1".into(),
            name: "Pulsar".into(),
            enabled: true,
            threshold: 20,
            low_battery_icon_path: Some("low_mouse_red.png".into()),
        };
        let resolved = resolve_device_icon_path(&cfg, &dev);
        assert_eq!(
            resolved,
            Some(std::path::PathBuf::from("D:\\MyIcons").join("low_mouse_red.png"))
        );
    }
}
```

- [ ] **Step 2: Run `cargo test tests::test_resolve_device_low_icon_path_uses_custom_icons_dir` to verify failure**

Run: `cargo test test_resolve_device_low_icon_path_uses_custom_icons_dir`
Expected: FAIL (`resolve_device_icon_path` not defined).

- [ ] **Step 3: Wire borderless Flyout viewport, single-click tray toggle, focus-loss auto-hide, and custom icon resolution in `src/main.rs`**

Update `src/main.rs`:
1. Extract `fn resolve_device_icon_path(cfg: &AppConfig, dev: &DeviceConfig) -> Option<std::path::PathBuf>` using `crate::config::resolve_icons_dir(cfg)` for relative filenames, and call it in `update_tray_icon_and_menu_local`. Call `setup_icons_folder_at` on startup with `resolve_icons_dir(&config)`.
2. In `update_tray_icon_and_menu_local`, iterate `&s.config.devices` (filtering `s.active_device_ids.contains(&dev.unique_id)`) instead of `&s.active_device_ids` so drag-reordering in the flyout also controls tray context menu order. Rename menu label `"Settings"` to `"Open BatStat"`.
3. Configure `TrayIconBuilder` with `.with_menu_on_left_click(false)`.
4. Configure `ViewportBuilder` with `.with_title(win_flyout::WINDOW_TITLE).with_inner_size([win_flyout::FLYOUT_WIDTH, win_flyout::FLYOUT_HEIGHT]).with_decorations(false).with_taskbar(false).with_resizable(false).with_always_on_top()`. Use `win_flyout::WINDOW_TITLE` in the single-instance `FindWindowW` check.
5. In `BatStatApp`:
   - Track `style_applied: bool`, `last_tray_rect: Option<win_flyout::ScreenRect>`, `last_click_pos: Option<(i32, i32)>`, `last_focus_lost: Option<std::time::Instant>`, `opened_at: Option<std::time::Instant>`, `saved_pin_state: bool`, and `saved_custom_pos: bool`.
   - Handle `TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, rect, position, .. }`: convert `rect` to `ScreenRect`, check `win_flyout::should_open_on_tray_click(self.visible, self.last_focus_lost, Instant::now())`; if opening, instantiate/reuse `SettingsWindow`, restore `is_pinned`/`has_custom_position`, and call `win_flyout::position_and_show_flyout(hwnd, self.last_tray_rect, self.last_click_pos, ctx.pixels_per_point(), self.saved_custom_pos)`; if closing, flush autosave and call `hide_flyout`.
   - When `self.visible` is `true`, apply `apply_flyout_window_style(hwnd)` once, and if `!ui_state.is_pinned && !ui_state.suppress_focus_hide && opened_at.elapsed() > 200ms && !ctx.input(|i| i.focused)`, flush autosave, set `last_focus_lost = Some(Instant::now())`, and hide the flyout.
   - Remove the unconditional `ctx.request_repaint_after(100ms)` when `!self.visible` (keep a `250ms` live-status refresh repaint **only while `self.visible == true`** so hidden idle CPU is `0%`).

- [ ] **Step 4: Run full test suite and release build to verify zero warnings and all tests green**

Run: `cargo test && cargo build --release`
Expected: PASS (`0` warnings, all unit and headless UI tests pass, `target/release/batstat.exe` compiled cleanly).

- [ ] **Step 5: Commit Task 5**

```powershell
git add src/main.rs
git commit -m "feat(app): wire single-click tray flyout, focus-loss auto-hide, and ordered tray menu"
```
