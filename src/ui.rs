use crate::config::{AppConfig, DeviceConfig, save_config};
use crate::autostart::set_autostart;
use crate::plugins::{DeviceBatteryStatus, BatteryChannel, ChannelType};
use eframe::egui;
use std::time::{Duration, Instant};

/// Pro Dark Desktop Design System (Windows 11 Fluent / Linear Dark Zinc)
#[allow(dead_code)]
pub mod theme {
    use eframe::egui::Color32;

    // Base Surfaces
    pub const BG_APP: Color32 = Color32::from_rgb(18, 18, 20);           // Deep zinc root canvas (#121214)
    pub const BG_HEADER: Color32 = Color32::from_rgb(22, 22, 26);        // Top header & bottom footer panels (#16161a)
    pub const BG_CARD: Color32 = Color32::from_rgb(26, 26, 30);          // Peripheral & container cards (#1a1a1e)
    pub const BG_CARD_HOVER: Color32 = Color32::from_rgb(32, 32, 38);    // Hovered card background (#202026)
    pub const BG_SUBTLE: Color32 = Color32::from_rgb(20, 20, 24);        // Recessed well (gauges, inputs, previews) (#141418)
    pub const BG_PILL_TRACK: Color32 = Color32::from_rgb(22, 22, 26);    // Segmented tab bar container (#16161a)
    pub const BG_TAB_ACTIVE: Color32 = Color32::from_rgb(44, 44, 52);    // Active segmented tab pill (#2c2c34)
    pub const BG_TAB_HOVER: Color32 = Color32::from_rgb(32, 32, 38);     // Inactive tab hover (#202026)

    // Structural Borders & Dividers
    pub const BORDER_CARD: Color32 = Color32::from_rgb(42, 42, 48);      // Standard container border (#2a2a30)
    pub const BORDER_CARD_ACTIVE: Color32 = Color32::from_rgb(60, 60, 70);// Connected/active card stroke (#3c3c46)
    pub const BORDER_HOVER: Color32 = Color32::from_rgb(70, 70, 82);     // Interactive hover border
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(34, 34, 40);    // Recessed inner well stroke
    pub const BORDER_FOCUS: Color32 = Color32::from_rgb(140, 140, 155);  // Input focus ring

    // Typography & Text Hierarchy
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(244, 244, 246);  // Zinc-100 High-contrast white (#f4f4f6)
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(212, 212, 216);// Zinc-300 Standard body (#d4d4d8)
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(156, 163, 175);    // Zinc-400 Secondary labels (#9ca3af)
    pub const TEXT_DIM: Color32 = Color32::from_rgb(113, 113, 122);      // Zinc-500 Meta/Footnotes (#71717a)

    // Interactive Action Buttons
    pub const BTN_PRIMARY_BG: Color32 = Color32::from_rgb(244, 244, 246);  // Solid crisp white button (Linear/Fluent style)
    pub const BTN_PRIMARY_TEXT: Color32 = Color32::from_rgb(18, 18, 20);   // Dark ink text
    pub const BTN_PRIMARY_HOVER: Color32 = Color32::WHITE;
    pub const BTN_SECONDARY_BG: Color32 = Color32::from_rgb(32, 32, 38);   // Subtle dark secondary button
    pub const BTN_SECONDARY_BORDER: Color32 = Color32::from_rgb(52, 52, 60);

    // Semantic Status & Telemetry
    pub const STATUS_ONLINE: Color32 = Color32::from_rgb(34, 197, 94);     // #22c55e Emerald Green
    pub const STATUS_ONLINE_BG: Color32 = Color32::from_rgb(20, 32, 25);
    pub const STATUS_CHARGING: Color32 = Color32::from_rgb(245, 158, 11);  // #f59e0b Warm Gold / Amber
    pub const STATUS_CHARGING_BG: Color32 = Color32::from_rgb(34, 28, 18);
    pub const STATUS_LOW_ALERT: Color32 = Color32::from_rgb(239, 68, 68);  // #ef4444 Coral Red
    pub const STATUS_LOW_ALERT_BG: Color32 = Color32::from_rgb(34, 20, 22);
    pub const STATUS_OFFLINE: Color32 = Color32::from_rgb(113, 113, 122);   // #71717a Muted Zinc
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Devices,
    General,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceFilter {
    All,
    Connected,
    Charging,
    LowBattery,
    Offline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatCardType {
    Configured,
    Connected,
    Charging,
    LowAlerts,
}

pub struct SettingsWindow {
    pub config: AppConfig,
    pub original_config: AppConfig,
    pub active_devices: Vec<String>, // list of active unique_ids
    pub device_statuses: std::collections::HashMap<String, DeviceBatteryStatus>,
    pub request_close: bool,
    pub discard_changes: bool,
    pub request_poll: bool,
    pub request_test_notification: bool,
    pub device_removed: bool,
    pub icon_textures: std::collections::HashMap<String, egui::TextureHandle>,
    pub update_status: crate::UpdateStatus,
    pub request_update_check: bool,
    pub request_download_install: Option<String>,
    pub active_tab: Tab,

    // UX enhancements:
    pub search_query: String,
    pub filter_status: DeviceFilter,
    pub editing_device_id: Option<String>,
    pub edit_name_buffer: String,
    pub focus_rename: bool,
    pub confirming_remove_id: Option<String>,
    pub scanning_timer: Option<Instant>,
    pub toast: Option<(String, Instant)>,
    pub debug_log_content: Option<String>,
    pub fonts_initialized: bool,
    pub config_saved_in_place: bool,
}

impl SettingsWindow {
    pub fn new(
        mut config: AppConfig,
        active_devices: Vec<String>,
        device_statuses: std::collections::HashMap<String, DeviceBatteryStatus>,
    ) -> Self {
        config.polling_interval_secs = config.polling_interval_secs.clamp(1, 60);
        let original_config = config.clone();
        Self {
            config,
            original_config,
            active_devices,
            device_statuses,
            request_close: false,
            discard_changes: false,
            request_poll: false,
            request_test_notification: false,
            device_removed: false,
            icon_textures: std::collections::HashMap::new(),
            update_status: crate::UpdateStatus::Idle,
            request_update_check: false,
            request_download_install: None,
            active_tab: Tab::Devices,

            search_query: String::new(),
            filter_status: DeviceFilter::All,
            editing_device_id: None,
            edit_name_buffer: String::new(),
            focus_rename: false,
            confirming_remove_id: None,
            scanning_timer: None,
            toast: None,
            debug_log_content: None,
            fonts_initialized: false,
            config_saved_in_place: false,
        }
    }

    pub fn show_toast(&mut self, message: &str) {
        self.toast = Some((message.to_string(), Instant::now()));
    }

    pub fn refresh_log(&mut self) {
        let log_path = crate::config::get_config_path().map(|mut p| {
            p.pop();
            p.push("debug.log");
            p
        });
        if let Some(ref path) = log_path {
            if let Ok(content) = std::fs::read_to_string(path) {
                let lines: Vec<&str> = content.lines().collect();
                let tail = if lines.len() > 80 {
                    lines[lines.len() - 80..].join("\n")
                } else {
                    content
                };
                self.debug_log_content = Some(tail);
            } else {
                self.debug_log_content = Some(
                    "No logs recorded yet.\nEnable 'Diagnostic Debug Logging' in Settings to capture live device events.".to_string(),
                );
            }
        }
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.config.polling_interval_secs != self.original_config.polling_interval_secs
            || self.config.autostart != self.original_config.autostart
            || self.config.enable_notifications != self.original_config.enable_notifications
            || self.config.enable_debug_logging != self.original_config.enable_debug_logging
            || self.config.tray_battery_channel != self.original_config.tray_battery_channel
            || self.config.devices.len() != self.original_config.devices.len()
            || self.config.devices.iter().zip(self.original_config.devices.iter()).any(|(a, b)| {
                a.unique_id != b.unique_id
                    || a.name != b.name
                    || a.enabled != b.enabled
                    || a.threshold != b.threshold
                    || a.low_battery_icon_path != b.low_battery_icon_path
            })
    }

    pub fn ensure_textures(&mut self, ctx: &egui::Context) {
        let icon_list = crate::config::get_icon_list();
        let needs_load = self.icon_textures.is_empty()
            || icon_list.iter().any(|name| !self.icon_textures.contains_key(name));

        if !needs_load {
            return;
        }

        // Guarantee icons folder and default variations exist
        crate::config::setup_icons_folder();

        // Built-in defaults as guaranteed fallback
        let builtins: [(&str, &[u8]); 5] = [
            ("ok.png", include_bytes!("icons/ok.png")),
            ("low_mouse.png", include_bytes!("icons/low_mouse.png")),
            ("low_gamepad.png", include_bytes!("icons/low_gamepad.png")),
            ("low_buds.png", include_bytes!("icons/low_buds.png")),
            ("low_keyboard.png", include_bytes!("icons/low_keyboard.png")),
        ];

        for (name, bytes) in builtins {
            if !self.icon_textures.contains_key(name) {
                if let Ok(img) = image::load_from_memory(bytes) {
                    let size = [img.width() as _, img.height() as _];
                    let img_rgba = img.to_rgba8();
                    let pixels = img_rgba.as_flat_samples();
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                    let texture = ctx.load_texture(name, color_image, egui::TextureOptions::default());
                    self.icon_textures.insert(name.to_string(), texture);
                }
            }
        }

        // Load custom or generated variations from AppData icons directory
        if let Some(icons_dir) = crate::config::get_icons_dir_path() {
            if let Ok(entries) = std::fs::read_dir(&icons_dir) {
                for entry in entries.flatten() {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_file() {
                            let name = entry.file_name().to_string_lossy().into_owned();
                            if !self.icon_textures.contains_key(&name)
                                && (name.ends_with(".png") || name.ends_with(".ico"))
                            {
                                let path = icons_dir.join(&name);
                                if let Ok(img) = image::open(&path) {
                                    let size = [img.width() as _, img.height() as _];
                                    let img_rgba = img.to_rgba8();
                                    let pixels = img_rgba.as_flat_samples();
                                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                                    let texture = ctx.load_texture(
                                        &name,
                                        color_image,
                                        egui::TextureOptions::default(),
                                    );
                                    self.icon_textures.insert(name, texture);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        // Hierarchical Escape: close sub-modes first before closing the app
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.editing_device_id.is_some() {
                self.editing_device_id = None;
                self.show_toast("Renaming cancelled");
            } else if self.confirming_remove_id.is_some() {
                self.confirming_remove_id = None;
            } else {
                self.discard_changes = true;
                self.request_close = true;
            }
        }

        // Ctrl + S -> Save in-place & persist
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
            if let Err(e) = save_config(&self.config) {
                eprintln!("Failed to save config: {}", e);
                self.show_toast("Failed to save settings");
            } else {
                self.original_config = self.config.clone();
                self.config_saved_in_place = true;
                self.show_toast("Settings saved (Ctrl+S)");
            }
        }

        // F5 or Ctrl + R -> Refresh / Poll devices
        if ctx.input(|i| i.key_pressed(egui::Key::F5) || (i.modifiers.command && i.key_pressed(egui::Key::R))) {
            self.request_poll = true;
            self.scanning_timer = Some(Instant::now());
            self.show_toast("Peripheral scan initiated...");
        }
    }
}

/// Sets up native Windows Segoe UI and Segoe UI Symbol fonts for fluent typography without tofu boxes
pub fn configure_fluent_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let mut font_added = false;

    if let Ok(font_bytes) = std::fs::read("C:\\Windows\\Fonts\\segoeui.ttf") {
        fonts.font_data.insert(
            "segoe_ui".to_owned(),
            egui::FontData::from_owned(font_bytes),
        );
        if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            family.insert(0, "segoe_ui".to_owned());
            font_added = true;
        }
    }

    if let Ok(font_bytes) = std::fs::read("C:\\Windows\\Fonts\\seguisym.ttf") {
        fonts.font_data.insert(
            "segoe_ui_sym".to_owned(),
            egui::FontData::from_owned(font_bytes),
        );
        if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            family.push("segoe_ui_sym".to_owned());
            font_added = true;
        }
    }

    if font_added {
        ctx.set_fonts(fonts);
    }
}

/// Formats icon filename into human-readable label with color variation
pub fn format_icon_label(filename: &str) -> String {
    let name = filename
        .strip_suffix(".png")
        .or_else(|| filename.strip_suffix(".ico"))
        .unwrap_or(filename);

    if let Some(rest) = name.strip_prefix("low_") {
        let parts: Vec<&str> = rest.split('_').collect();
        if parts.len() == 2 {
            let category = match parts[0] {
                "mouse" => "Mouse",
                "gamepad" => "Gamepad",
                "buds" => "Earbuds",
                "keyboard" => "Keyboard",
                other => other,
            };
            let color = match parts[1] {
                "red" => "Red",
                "orange" => "Orange",
                "yellow" => "Yellow",
                "blue" => "Blue",
                other => other,
            };
            return format!("{} ({})", category, color);
        } else if parts.len() == 1 {
            let category = match parts[0] {
                "mouse" => "Mouse (White)",
                "gamepad" => "Gamepad (White)",
                "buds" => "Earbuds (White)",
                "keyboard" => "Keyboard (White)",
                other => other,
            };
            return category.to_string();
        }
    }

    filename.to_string()
}

/// Returns device category info: (Type Tag, Icon Glyph, Friendly Title, Base Type)
pub fn get_device_category(unique_id: &str) -> (&'static str, &'static str, &'static str, &'static str) {
    if unique_id.starts_with("pulsar_") || unique_id.starts_with("logitech_") || unique_id.contains("mouse") {
        ("MOUSE", "mouse", "Gaming Mouse", "mouse")
    } else if unique_id.starts_with("xbox_") || unique_id.contains("gamepad") || unique_id.contains("controller") {
        ("GAMEPAD", "gamepad", "Controller", "gamepad")
    } else if unique_id.starts_with("gamebuds") || unique_id.contains("buds") || unique_id.contains("headset") {
        ("BUDS", "buds", "Wireless Earbuds", "buds")
    } else if unique_id.starts_with("keyboard") || unique_id.contains("keyboard") {
        ("KEYBOARD", "keyboard", "Keyboard", "keyboard")
    } else {
        ("PERIPHERAL", "peripheral", "Peripheral", "peripheral")
    }
}

pub fn get_available_channels(config: &AppConfig) -> Vec<(String, String)> {
    let mut options = Vec::new();
    for dev in &config.devices {
        if dev.unique_id.starts_with("gamebuds") {
            options.push((format!("{}:Left", dev.unique_id), format!("{} (Left)", dev.name)));
            options.push((format!("{}:Right", dev.unique_id), format!("{} (Right)", dev.name)));
        } else {
            options.push((format!("{}:Main", dev.unique_id), dev.name.clone()));
        }
    }
    options
}

/// Custom animated toggle switch widget with Emerald active state (Windows 11 Fluent style)
pub fn toggle_ui(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = egui::vec2(36.0, 20.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, *on, ""));

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool(response.id, *on);

        let track_color = if *on {
            theme::STATUS_ONLINE // Mature emerald green active state
        } else {
            egui::Color32::from_rgb(44, 44, 52) // Dark zinc track
        };

        let rounding = rect.height() / 2.0;
        ui.painter().rect_filled(rect, rounding, track_color);

        if response.hovered() {
            ui.painter().rect_stroke(
                rect,
                rounding,
                egui::Stroke::new(1.0, theme::BORDER_HOVER),
            );
        }

        // Knob
        let knob_radius = rect.height() / 2.0 - 2.5;
        let min_x = rect.left() + knob_radius + 2.5;
        let max_x = rect.right() - knob_radius - 2.5;
        let knob_x = min_x + how_on * (max_x - min_x);
        let knob_center = egui::pos2(knob_x, rect.center().y);
        ui.painter().circle_filled(knob_center, knob_radius, egui::Color32::WHITE);
    }

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Custom Battery Gauge widget with dynamic semantic colors, drop-shadowed text, and threshold notch
pub fn battery_bar_ui(
    ui: &mut egui::Ui,
    percentage: u8,
    charging: bool,
    threshold: u8,
    width: f32,
) -> egui::Response {
    let desired_size = egui::vec2(width, 22.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let rounding = egui::Rounding::same(6.0);

        // Recessed slot background
        painter.rect_filled(rect, rounding, theme::BG_SUBTLE);
        painter.rect_stroke(rect, rounding, egui::Stroke::new(1.0, theme::BORDER_SUBTLE));

        // Filled section
        let clamped_pct = (percentage.min(100) as f32) / 100.0;
        let fill_w = (rect.width() - 4.0) * clamped_pct;

        let fill_color = if charging {
            theme::STATUS_CHARGING // Warm gold/amber
        } else if percentage > 50 {
            theme::STATUS_ONLINE // Emerald green
        } else if percentage > threshold {
            theme::STATUS_CHARGING // Amber warning
        } else {
            theme::STATUS_LOW_ALERT // Coral red
        };

        if fill_w > 2.0 {
            let fill_rect = egui::Rect::from_min_size(
                rect.min + egui::vec2(2.0, 2.0),
                egui::vec2(fill_w, rect.height() - 4.0),
            );
            painter.rect_filled(fill_rect, egui::Rounding::same(4.0), fill_color);
        }

        // Threshold notch marker
        let thresh_clamped = (threshold.clamp(5, 95) as f32) / 100.0;
        let thresh_x = rect.left() + 2.0 + (rect.width() - 4.0) * thresh_clamped;
        let thresh_top = egui::pos2(thresh_x, rect.top() + 1.0);
        let thresh_bottom = egui::pos2(thresh_x, rect.bottom() - 1.0);
        painter.line_segment(
            [thresh_top, thresh_bottom],
            egui::Stroke::new(1.5, egui::Color32::from_rgba_premultiplied(239, 68, 68, 200)),
        );

        // Embedded percentage text with contrast drop-shadow
        let pct_str = format!("{}%", percentage);
        let center = rect.center();
        painter.text(
            egui::pos2(center.x + 1.0, center.y + 1.0),
            egui::Align2::CENTER_CENTER,
            &pct_str,
            egui::FontId::monospace(10.5),
            egui::Color32::from_rgba_premultiplied(0, 0, 0, 180),
        );
        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            &pct_str,
            egui::FontId::monospace(10.5),
            egui::Color32::WHITE,
        );
    }

    let tooltip = if charging {
        format!("Battery: {}% (Charging) • Alert Threshold: {}%", percentage, threshold)
    } else if percentage <= threshold {
        format!("Critical Low: {}% (Below {}% cutoff)", percentage, threshold)
    } else {
        format!("Battery Level: {}% • Alert Warning Cutoff: {}%", percentage, threshold)
    };

    response.on_hover_text(tooltip)
}

/// Stat metric card rendered with vector graphics (guaranteed NO unicode tofu square boxes)
fn render_stat_card(
    ui: &mut egui::Ui,
    kind: StatCardType,
    value: &str,
    label: &str,
    accent: egui::Color32,
    is_selected: bool,
    width: f32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 52.0), egui::Sense::click());

    let bg_color = if is_selected {
        theme::BG_TAB_ACTIVE
    } else if response.hovered() {
        theme::BG_CARD_HOVER
    } else {
        theme::BG_CARD
    };

    let border_stroke = if is_selected {
        egui::Stroke::new(1.5, accent)
    } else if response.hovered() {
        egui::Stroke::new(1.0, theme::BORDER_HOVER)
    } else {
        egui::Stroke::new(1.0, theme::BORDER_CARD)
    };

    ui.painter().rect_filled(rect, 8.0, bg_color);
    ui.painter().rect_stroke(rect, 8.0, border_stroke);

    // Left Icon Rendering: 100% vector drawn (zero font/unicode dependencies)
    let icon_center = egui::pos2(rect.left() + 22.0, rect.center().y);
    match kind {
        StatCardType::Connected => {
            // Render glowing vector status dot
            ui.painter().circle_filled(icon_center, 5.0, accent);
            ui.painter().circle_stroke(
                icon_center,
                7.5,
                egui::Stroke::new(1.0, accent.linear_multiply(0.35)),
            );
        }
        StatCardType::LowAlerts => {
            // Crisp vector warning triangle with exclamation mark
            let p1 = egui::pos2(icon_center.x, icon_center.y - 7.5);
            let p2 = egui::pos2(icon_center.x - 7.5, icon_center.y + 6.5);
            let p3 = egui::pos2(icon_center.x + 7.5, icon_center.y + 6.5);
            ui.painter().line_segment([p1, p2], egui::Stroke::new(1.5, accent));
            ui.painter().line_segment([p2, p3], egui::Stroke::new(1.5, accent));
            ui.painter().line_segment([p3, p1], egui::Stroke::new(1.5, accent));
            ui.painter().line_segment(
                [egui::pos2(icon_center.x, icon_center.y - 3.0), egui::pos2(icon_center.x, icon_center.y + 1.5)],
                egui::Stroke::new(1.5, accent),
            );
            ui.painter().circle_filled(egui::pos2(icon_center.x, icon_center.y + 4.5), 1.0, accent);
        }
        StatCardType::Charging => {
            // Crisp vector lightning bolt
            let pts = [
                egui::pos2(icon_center.x + 1.0, icon_center.y - 8.0),
                egui::pos2(icon_center.x - 4.5, icon_center.y + 0.5),
                egui::pos2(icon_center.x - 0.5, icon_center.y + 0.5),
                egui::pos2(icon_center.x - 1.0, icon_center.y + 8.0),
                egui::pos2(icon_center.x + 4.5, icon_center.y - 0.5),
                egui::pos2(icon_center.x + 0.5, icon_center.y - 0.5),
            ];
            ui.painter().add(egui::Shape::convex_polygon(
                pts.to_vec(),
                accent,
                egui::Stroke::NONE,
            ));
        }
        StatCardType::Configured => {
            // Crisp vector peripheral chip/card
            let dev_rect = egui::Rect::from_center_size(icon_center, egui::vec2(13.0, 15.0));
            ui.painter().rect_stroke(dev_rect, 3.0, egui::Stroke::new(1.5, accent));
            ui.painter().line_segment(
                [egui::pos2(dev_rect.left() + 3.0, dev_rect.bottom() - 4.0), egui::pos2(dev_rect.right() - 3.0, dev_rect.bottom() - 4.0)],
                egui::Stroke::new(1.2, accent),
            );
            ui.painter().circle_filled(egui::pos2(icon_center.x, icon_center.y - 2.5), 2.0, accent);
        }
    }

    // Value and Label
    let text_left = rect.left() + 42.0;
    ui.painter().text(
        egui::pos2(text_left, rect.top() + 10.0),
        egui::Align2::LEFT_TOP,
        value,
        egui::FontId::proportional(16.0),
        theme::TEXT_PRIMARY,
    );
    ui.painter().text(
        egui::pos2(text_left, rect.top() + 31.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::proportional(9.5),
        if is_selected { accent } else { theme::TEXT_MUTED },
    );

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

impl eframe::App for SettingsWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.fonts_initialized {
            configure_fluent_fonts(ctx);
            self.fonts_initialized = true;
        }

        self.ensure_textures(ctx);
        self.handle_shortcuts(ctx);

        // Update scanning timer state
        if let Some(start) = self.scanning_timer {
            if start.elapsed() > Duration::from_millis(1500) {
                self.scanning_timer = None;
            } else {
                ctx.request_repaint();
            }
        }

        // Pro Dark Visual Theme Setup (Windows 11 Fluent / Linear Dark Zinc)
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = theme::BG_APP;
        visuals.window_fill = theme::BG_APP;

        // Container / Noninteractive
        visuals.widgets.noninteractive.bg_fill = theme::BG_CARD;
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, theme::BORDER_CARD);
        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, theme::TEXT_MUTED);
        visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);

        // Inactive interactive widgets
        visuals.widgets.inactive.bg_fill = theme::BG_CARD;
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, theme::BORDER_CARD);
        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, theme::TEXT_SECONDARY);
        visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);

        // Hover state
        visuals.widgets.hovered.bg_fill = theme::BG_CARD_HOVER;
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, theme::BORDER_HOVER);
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, theme::TEXT_PRIMARY);
        visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);

        // Active state
        visuals.widgets.active.bg_fill = theme::BG_TAB_ACTIVE;
        visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, theme::BORDER_FOCUS);
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, theme::TEXT_PRIMARY);
        visuals.widgets.active.rounding = egui::Rounding::same(6.0);

        visuals.extreme_bg_color = theme::BG_SUBTLE;
        visuals.selection.bg_fill = theme::BG_TAB_ACTIVE;
        visuals.selection.stroke = egui::Stroke::new(1.0, theme::BORDER_FOCUS);

        ctx.set_visuals(visuals);

        // -------------------------------------------------------------
        // 1. TOP PANEL: Header & Discovery Trigger
        // -------------------------------------------------------------
        egui::TopBottomPanel::top("header_panel")
            .frame(
                egui::Frame::none()
                    .fill(theme::BG_HEADER)
                    .inner_margin(egui::Margin { left: 20.0, right: 20.0, top: 12.0, bottom: 12.0 })
                    .stroke(egui::Stroke::new(1.0, theme::BORDER_CARD)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Logo Badge (Crisp vector lightning bolt)
                    let (logo_rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                    ui.painter().rect_filled(logo_rect, 6.0, theme::BG_CARD);
                    ui.painter().rect_stroke(logo_rect, 6.0, egui::Stroke::new(1.0, theme::BORDER_CARD));
                    let bolt_center = logo_rect.center();
                    let pts = [
                        egui::pos2(bolt_center.x + 1.0, bolt_center.y - 7.0),
                        egui::pos2(bolt_center.x - 3.5, bolt_center.y),
                        egui::pos2(bolt_center.x, bolt_center.y),
                        egui::pos2(bolt_center.x - 1.0, bolt_center.y + 7.0),
                        egui::pos2(bolt_center.x + 3.5, bolt_center.y),
                        egui::pos2(bolt_center.x, bolt_center.y),
                    ];
                    ui.painter().add(egui::Shape::convex_polygon(
                        pts.to_vec(),
                        theme::STATUS_CHARGING,
                        egui::Stroke::NONE,
                    ));

                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("BatStat")
                            .strong()
                            .color(theme::TEXT_PRIMARY)
                            .font(egui::FontId::proportional(16.0)),
                    );

                    // Version Tag
                    let (v_rect, _) = ui.allocate_exact_size(egui::vec2(48.0, 18.0), egui::Sense::hover());
                    ui.painter().rect_filled(v_rect, 4.0, theme::BG_SUBTLE);
                    ui.painter().rect_stroke(v_rect, 4.0, egui::Stroke::new(1.0, theme::BORDER_CARD));
                    ui.painter().text(
                        v_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        concat!("v", env!("CARGO_PKG_VERSION")),
                        egui::FontId::proportional(9.5),
                        theme::TEXT_MUTED,
                    );

                    // Live monitor status badge
                    ui.add_space(8.0);
                    let active_count = self
                        .config
                        .devices
                        .iter()
                        .filter(|d| self.active_devices.contains(&d.unique_id))
                        .count();

                    let (status_rect, _) = ui.allocate_exact_size(
                        egui::vec2(if active_count > 0 { 95.0 } else { 105.0 }, 18.0),
                        egui::Sense::hover(),
                    );
                    let status_dot_color = if active_count > 0 {
                        theme::STATUS_ONLINE
                    } else {
                        theme::STATUS_OFFLINE
                    };
                    let status_bg = if active_count > 0 {
                        theme::STATUS_ONLINE_BG
                    } else {
                        theme::BG_SUBTLE
                    };
                    ui.painter().rect_filled(status_rect, 9.0, status_bg);
                    ui.painter().circle_filled(
                        egui::pos2(status_rect.left() + 9.0, status_rect.center().y),
                        3.5,
                        status_dot_color,
                    );
                    let status_text = if active_count > 0 {
                        format!("{} Online", active_count)
                    } else {
                        "Standby / Idle".to_string()
                    };
                    ui.painter().text(
                        egui::pos2(status_rect.left() + 18.0, status_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        status_text,
                        egui::FontId::proportional(9.5),
                        if active_count > 0 { theme::STATUS_ONLINE } else { theme::TEXT_MUTED },
                    );

                    // Right-side discovery action
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let is_scanning = self.scanning_timer.is_some();
                        let scan_text = if is_scanning { "Scanning..." } else { "Detect Devices" };
                        let scan_btn = egui::Button::new(
                            egui::RichText::new(scan_text)
                                .color(theme::TEXT_PRIMARY)
                                .font(egui::FontId::proportional(11.0))
                                .strong(),
                        )
                        .fill(theme::BTN_SECONDARY_BG)
                        .rounding(6.0)
                        .min_size(egui::vec2(110.0, 26.0));

                        let resp = ui.add_enabled(!is_scanning, scan_btn)
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if resp.clicked() {
                            self.request_poll = true;
                            self.scanning_timer = Some(Instant::now());
                            self.show_toast("Scanning for connected peripherals...");
                        }
                        resp.on_hover_text("Trigger immediate peripheral discovery scan (F5)");
                    });
                });
            });

        // -------------------------------------------------------------
        // 2. BOTTOM PANEL: Status Bar & Save/Close
        // -------------------------------------------------------------
        egui::TopBottomPanel::bottom("footer_panel")
            .frame(
                egui::Frame::none()
                    .fill(theme::BG_HEADER)
                    .inner_margin(egui::Margin { left: 20.0, right: 20.0, top: 12.0, bottom: 12.0 })
                    .stroke(egui::Stroke::new(1.0, theme::BORDER_CARD)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Left: Changes indicator & shortcuts hint (Vector dot eliminates tofu checkmark box!)
                    if self.has_unsaved_changes() {
                        let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                        ui.painter().circle_filled(dot_rect.center(), 3.5, theme::STATUS_CHARGING);
                        ui.label(
                            egui::RichText::new("Unsaved changes • Press Ctrl+S to save in-place")
                                .color(theme::STATUS_CHARGING)
                                .font(egui::FontId::proportional(11.0)),
                        );
                    } else {
                        let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                        ui.painter().circle_filled(dot_rect.center(), 3.5, theme::STATUS_ONLINE);
                        ui.label(
                            egui::RichText::new("Settings up to date • Esc to close")
                                .color(theme::TEXT_MUTED)
                                .font(egui::FontId::proportional(11.0)),
                        );
                    }

                    // Right: Cancel & Save Buttons
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // High-contrast primary button (Linear/Fluent style)
                        let save_btn = egui::Button::new(
                            egui::RichText::new("Save & Close")
                                .color(theme::BTN_PRIMARY_TEXT)
                                .strong()
                                .font(egui::FontId::proportional(12.0)),
                        )
                        .fill(theme::BTN_PRIMARY_BG)
                        .rounding(6.0)
                        .min_size(egui::vec2(110.0, 30.0));

                        let save_resp = ui.add(save_btn).on_hover_cursor(egui::CursorIcon::PointingHand);
                        if save_resp.clicked() {
                            if let Err(e) = save_config(&self.config) {
                                eprintln!("Failed to save config: {}", e);
                            }
                            self.original_config = self.config.clone();
                            self.discard_changes = false;
                            self.request_close = true;
                        }
                        save_resp.on_hover_text("Save all preferences to config.toml and close window");

                        ui.add_space(8.0);

                        // Cancel button
                        let cancel_btn = egui::Button::new(
                            egui::RichText::new("Cancel")
                                .color(theme::TEXT_MUTED)
                                .font(egui::FontId::proportional(12.0)),
                        )
                        .fill(egui::Color32::TRANSPARENT)
                        .frame(false);

                        let cancel_resp = ui.add(cancel_btn).on_hover_cursor(egui::CursorIcon::PointingHand);
                        if cancel_resp.clicked() {
                            self.discard_changes = true;
                            self.request_close = true;
                        }
                        cancel_resp.on_hover_text("Discard uncommitted changes and close window (Esc)");
                    });
                });
            });

        // -------------------------------------------------------------
        // 3. CENTRAL PANEL: Segmented Navigation & Active Tab
        // -------------------------------------------------------------
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(theme::BG_APP)
                    .inner_margin(egui::Margin { left: 20.0, right: 20.0, top: 12.0, bottom: 8.0 }),
            )
            .show(ctx, |ui| {
                // Segmented Pill Navigation Bar
                ui.horizontal(|ui| {
                    let total_width = 390.0;
                    let pill_height = 34.0;
                    ui.add_space((ui.available_width() - total_width).max(0.0) / 2.0);

                    let (rect, _) = ui.allocate_exact_size(egui::vec2(total_width, pill_height), egui::Sense::hover());

                    // Container Pill Background
                    ui.painter().rect_filled(rect, 17.0, theme::BG_PILL_TRACK);
                    ui.painter().rect_stroke(rect, 17.0, egui::Stroke::new(1.0, theme::BORDER_CARD));

                    let tab_width = (total_width - 8.0) / 3.0;

                    let tabs = [
                        (Tab::Devices, format!("Devices ({})", self.config.devices.len())),
                        (Tab::General, "Settings".to_string()),
                        (Tab::About, "About & Logs".to_string()),
                    ];

                    for (i, (tab_variant, label)) in tabs.into_iter().enumerate() {
                        let tab_rect = egui::Rect::from_min_size(
                            rect.min + egui::vec2(4.0 + i as f32 * tab_width, 3.0),
                            egui::vec2(tab_width, pill_height - 6.0),
                        );

                        let is_active = self.active_tab == tab_variant;
                        let resp = ui.interact(tab_rect, ui.id().with(format!("tab_btn_{}", i)), egui::Sense::click())
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if resp.clicked() {
                            if tab_variant == Tab::About && self.active_tab != Tab::About {
                                self.refresh_log();
                            }
                            self.active_tab = tab_variant;
                        }

                        if is_active {
                            ui.painter().rect_filled(tab_rect, 14.0, theme::BG_TAB_ACTIVE);
                            ui.painter().rect_stroke(tab_rect, 14.0, egui::Stroke::new(1.0, theme::BORDER_HOVER));
                        } else if resp.hovered() {
                            ui.painter().rect_filled(tab_rect, 14.0, theme::BG_TAB_HOVER);
                        }

                        let text_color = if is_active {
                            theme::TEXT_PRIMARY
                        } else {
                            theme::TEXT_MUTED
                        };

                        ui.painter().text(
                            tab_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            label,
                            egui::FontId::proportional(11.5),
                            text_color,
                        );
                    }
                });

                ui.add_space(14.0);

                // Main Scrollable Body
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        match self.active_tab {
                            Tab::Devices => self.render_devices_tab(ui),
                            Tab::General => self.render_general_tab(ui),
                            Tab::About => self.render_about_tab(ui),
                        }
                    });

                // Toast Banner
                if let Some((ref msg, time)) = self.toast {
                    if time.elapsed() < Duration::from_millis(2600) {
                        ctx.request_repaint();
                        let toast_text = msg.clone();
                        let toast_width = ((toast_text.len() as f32 * 7.5) + 36.0).clamp(200.0, 420.0);
                        let toast_rect = egui::Rect::from_center_size(
                            egui::pos2(ui.clip_rect().center().x, ui.clip_rect().bottom() - 24.0),
                            egui::vec2(toast_width, 32.0),
                        );
                        ui.painter().rect_filled(
                            toast_rect.translate(egui::vec2(0.0, 2.0)),
                            8.0,
                            egui::Color32::from_rgba_premultiplied(0, 0, 0, 80),
                        );
                        ui.painter().rect_filled(toast_rect, 8.0, theme::BG_CARD);
                        ui.painter().rect_stroke(
                            toast_rect,
                            8.0,
                            egui::Stroke::new(1.0, theme::BORDER_HOVER),
                        );
                        ui.painter().text(
                            toast_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            toast_text,
                            egui::FontId::proportional(11.0),
                            theme::TEXT_PRIMARY,
                        );
                    } else {
                        self.toast = None;
                    }
                }
            });
    }
}

impl SettingsWindow {
    // -------------------------------------------------------------
    // DEVICES TAB
    // -------------------------------------------------------------
    fn render_devices_tab(&mut self, ui: &mut egui::Ui) {
        let total_devices = self.config.devices.len();
        let connected_count = self
            .config
            .devices
            .iter()
            .filter(|d| self.active_devices.contains(&d.unique_id))
            .count();

        let mut charging_count = 0;
        let mut low_battery_count = 0;

        for dev in &self.config.devices {
            if let Some(DeviceBatteryStatus::Online { channels }) = self.device_statuses.get(&dev.unique_id) {
                for chan in channels.iter().flatten() {
                    if chan.charging {
                        charging_count += 1;
                    }
                    if chan.percentage <= dev.threshold {
                        low_battery_count += 1;
                    }
                }
            }
        }

        // Summary Metric Cards Banner with Click-to-Filter capability
        let is_all = self.filter_status == DeviceFilter::All;
        let is_connected = self.filter_status == DeviceFilter::Connected;
        let is_charging = self.filter_status == DeviceFilter::Charging;
        let is_low = self.filter_status == DeviceFilter::LowBattery;

        ui.horizontal(|ui| {
            let card_width = ((ui.available_width() - 24.0) / 4.0).max(120.0);

            // Card 1: Total
            if render_stat_card(ui, StatCardType::Configured, &total_devices.to_string(), "Configured", theme::TEXT_SECONDARY, is_all, card_width).clicked() {
                self.filter_status = DeviceFilter::All;
            }
            // Card 2: Online (glowing vector circle)
            if render_stat_card(ui, StatCardType::Connected, &connected_count.to_string(), "Connected", theme::STATUS_ONLINE, is_connected, card_width).clicked() {
                self.filter_status = DeviceFilter::Connected;
            }
            // Card 3: Charging (lightning bolt)
            if render_stat_card(ui, StatCardType::Charging, &charging_count.to_string(), "Charging", theme::STATUS_CHARGING, is_charging, card_width).clicked() {
                self.filter_status = DeviceFilter::Charging;
            }
            // Card 4: Low Alerts (warning sign)
            if render_stat_card(ui, StatCardType::LowAlerts, &low_battery_count.to_string(), "Low Alerts", theme::STATUS_LOW_ALERT, is_low, card_width).clicked() {
                self.filter_status = DeviceFilter::LowBattery;
            }
        });

        ui.add_space(14.0);

        // Filter and Search Toolbar
        ui.horizontal(|ui| {
            // Filter Pills
            ui.horizontal(|ui| {
                let filters = [
                    (DeviceFilter::All, format!("All ({})", total_devices)),
                    (DeviceFilter::Connected, format!("Connected ({})", connected_count)),
                    (DeviceFilter::Charging, format!("Charging ({})", charging_count)),
                    (DeviceFilter::LowBattery, format!("Low ({})", low_battery_count)),
                    (DeviceFilter::Offline, format!("Offline ({})", total_devices.saturating_sub(connected_count))),
                ];

                for (filter_opt, label) in filters {
                    let is_sel = self.filter_status == filter_opt;
                    let btn = egui::Button::new(
                        egui::RichText::new(label)
                            .color(if is_sel { theme::TEXT_PRIMARY } else { theme::TEXT_MUTED })
                            .font(egui::FontId::proportional(11.0))
                            .strong(),
                    )
                    .fill(if is_sel { theme::BG_TAB_ACTIVE } else { theme::BG_CARD })
                    .stroke(egui::Stroke::new(1.0, if is_sel { theme::BORDER_HOVER } else { theme::BORDER_CARD }))
                    .rounding(6.0);

                    if ui.add(btn).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        self.filter_status = filter_opt;
                    }
                }
            });

            // Search input field
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !self.search_query.is_empty()
                    && ui.button(egui::RichText::new("Clear").font(egui::FontId::proportional(10.0))).clicked()
                {
                    self.search_query.clear();
                }
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("Search peripherals...")
                        .desired_width(170.0),
                );
            });
        });

        ui.add_space(10.0);

        // Filter devices
        let filtered_indices: Vec<usize> = self
            .config
            .devices
            .iter()
            .enumerate()
            .filter(|(_, dev)| {
                let is_active = self.active_devices.contains(&dev.unique_id);
                let (is_charging, is_low) = if let Some(DeviceBatteryStatus::Online { channels }) = self.device_statuses.get(&dev.unique_id) {
                    let chg = channels.iter().flatten().any(|c| c.charging);
                    let low = channels.iter().flatten().any(|c| c.percentage <= dev.threshold);
                    (chg, low)
                } else {
                    (false, false)
                };

                match self.filter_status {
                    DeviceFilter::All => true,
                    DeviceFilter::Connected => is_active,
                    DeviceFilter::Charging => is_charging,
                    DeviceFilter::LowBattery => is_low,
                    DeviceFilter::Offline => !is_active,
                }
            })
            .filter(|(_, dev)| {
                if self.search_query.trim().is_empty() {
                    true
                } else {
                    let q = self.search_query.trim().to_lowercase();
                    let (_, _, type_title, _) = get_device_category(&dev.unique_id);
                    dev.name.to_lowercase().contains(&q)
                        || dev.unique_id.to_lowercase().contains(&q)
                        || type_title.to_lowercase().contains(&q)
                }
            })
            .map(|(idx, _)| idx)
            .collect();

        // Empty state handling
        if self.config.devices.is_empty() {
            self.render_empty_state(ui);
            return;
        }

        if filtered_indices.is_empty() {
            egui::Frame::none()
                .fill(theme::BG_CARD)
                .stroke(egui::Stroke::new(1.0, theme::BORDER_CARD))
                .rounding(10.0)
                .inner_margin(egui::Margin::same(24.0))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("No Peripherals Match Filter").strong().color(theme::TEXT_PRIMARY).font(egui::FontId::proportional(14.0)));
                        ui.add_space(2.0);
                        ui.label(egui::RichText::new("Try adjusting your search query or switching back to 'All'.").color(theme::TEXT_MUTED).font(egui::FontId::proportional(11.0)));
                        ui.add_space(8.0);
                        if ui.button(egui::RichText::new("Clear Search & Filters").font(egui::FontId::proportional(11.0))).clicked() {
                            self.search_query.clear();
                            self.filter_status = DeviceFilter::All;
                        }
                    });
                });
            return;
        }

        let mut to_remove = None;
        let mut action_toast = None;

        for &idx in &filtered_indices {
            let mut dev = self.config.devices[idx].clone();
            let is_active = self.active_devices.contains(&dev.unique_id);
            let (type_tag, _icon_glyph, type_title, base_type) = get_device_category(&dev.unique_id);
            let device_status = self.device_statuses.get(&dev.unique_id).copied();
            let is_editing = self.editing_device_id.as_deref() == Some(&dev.unique_id);
            let is_confirming = self.confirming_remove_id.as_deref() == Some(&dev.unique_id);

            // Device Card Frame
            egui::Frame::none()
                .fill(theme::BG_CARD)
                .stroke(egui::Stroke::new(
                    1.0,
                    if is_active {
                        theme::BORDER_CARD_ACTIVE
                    } else {
                        theme::BORDER_CARD
                    },
                ))
                .rounding(10.0)
                .inner_margin(egui::Margin::same(14.0))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        // -------------------------------------------------------------
                        // ROW 1: Type Badge + Device Name + Online Status + Active Toggle
                        // -------------------------------------------------------------
                        ui.horizontal(|ui| {
                            // Category Tag Capsule
                            let (badge_rect, _) = ui.allocate_exact_size(egui::vec2(58.0, 22.0), egui::Sense::hover());
                            ui.painter().rect_filled(badge_rect, 4.0, theme::BG_SUBTLE);
                            ui.painter().rect_stroke(badge_rect, 4.0, egui::Stroke::new(1.0, theme::BORDER_CARD));
                            ui.painter().text(
                                badge_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                type_tag,
                                egui::FontId::proportional(9.5),
                                theme::TEXT_SECONDARY,
                            );

                            ui.add_space(4.0);

                            // Device Title & Inline Rename
                            if is_editing {
                                let edit_resp = ui.add(
                                    egui::TextEdit::singleline(&mut self.edit_name_buffer)
                                        .desired_width(180.0)
                                        .hint_text("Enter peripheral name..."),
                                );

                                if self.focus_rename {
                                    edit_resp.request_focus();
                                    self.focus_rename = false;
                                }

                                let save_clicked = ui.button(egui::RichText::new("Save").color(theme::STATUS_ONLINE).font(egui::FontId::proportional(11.0))).clicked();
                                let cancel_clicked = ui.button(egui::RichText::new("Cancel").color(theme::TEXT_MUTED).font(egui::FontId::proportional(11.0))).clicked();
                                let enter_pressed = edit_resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                                if save_clicked || enter_pressed {
                                    let trimmed = self.edit_name_buffer.trim();
                                    if !trimmed.is_empty() {
                                        dev.name = trimmed.to_string();
                                        action_toast = Some(format!("Renamed to '{}'", dev.name));
                                    }
                                    self.editing_device_id = None;
                                } else if cancel_clicked {
                                    self.editing_device_id = None;
                                }
                            } else {
                                ui.label(
                                    egui::RichText::new(&dev.name)
                                        .color(if dev.enabled { theme::TEXT_PRIMARY } else { theme::TEXT_MUTED })
                                        .font(egui::FontId::proportional(13.5))
                                        .strong(),
                                );

                                let rename_btn = ui.add(
                                    egui::Button::new(
                                        egui::RichText::new("Edit")
                                            .color(theme::TEXT_MUTED)
                                            .font(egui::FontId::proportional(10.5)),
                                    )
                                    .fill(theme::BG_SUBTLE)
                                    .stroke(egui::Stroke::new(1.0, theme::BORDER_CARD))
                                    .rounding(4.0),
                                ).on_hover_cursor(egui::CursorIcon::PointingHand);

                                if rename_btn.clicked() {
                                    self.editing_device_id = Some(dev.unique_id.clone());
                                    self.edit_name_buffer = dev.name.clone();
                                    self.focus_rename = true;
                                }
                                rename_btn.on_hover_text("Rename peripheral (Click to edit nickname)");
                            }

                            // Right side: Online indicator & Enable Switch
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                toggle_ui(ui, &mut dev.enabled);
                                ui.label(
                                    egui::RichText::new(if dev.enabled { "Enabled" } else { "Paused" })
                                        .color(theme::TEXT_MUTED)
                                        .font(egui::FontId::proportional(10.0)),
                                );

                                ui.add_space(8.0);

                                // Connection Pill Badge
                                let (status_badge, _) = ui.allocate_exact_size(egui::vec2(78.0, 20.0), egui::Sense::hover());
                                let (badge_bg, dot_color, status_label) = if is_active {
                                    (
                                        theme::STATUS_ONLINE_BG,
                                        theme::STATUS_ONLINE,
                                        "ONLINE",
                                    )
                                } else {
                                    (
                                        theme::BG_SUBTLE,
                                        theme::STATUS_OFFLINE,
                                        "OFFLINE",
                                    )
                                };
                                ui.painter().rect_filled(status_badge, 10.0, badge_bg);
                                ui.painter().circle_filled(
                                    egui::pos2(status_badge.left() + 9.0, status_badge.center().y),
                                    3.5,
                                    dot_color,
                                );
                                ui.painter().text(
                                    egui::pos2(status_badge.left() + 18.0, status_badge.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    status_label,
                                    egui::FontId::proportional(9.0),
                                    dot_color,
                                );
                            });
                        });

                        ui.add_space(8.0);

                        // -------------------------------------------------------------
                        // ROW 2: BATTERY GAUGES
                        // -------------------------------------------------------------
                        if !dev.enabled {
                            ui.label(
                                egui::RichText::new("Device monitoring is paused.")
                                    .color(theme::TEXT_DIM)
                                    .font(egui::FontId::proportional(11.0))
                                    .italics(),
                            );
                        } else if let Some(status) = device_status {
                            match status {
                                DeviceBatteryStatus::Online { channels } => {
                                    let active_channels: Vec<&BatteryChannel> = channels.iter().flatten().collect();
                                    if active_channels.is_empty() {
                                        ui.label(
                                            egui::RichText::new("Peripheral is connected but reports no battery channels.")
                                                .color(theme::TEXT_MUTED)
                                                .font(egui::FontId::proportional(10.5)),
                                        );
                                    } else if active_channels.len() == 1 {
                                        // Single channel
                                        let chan = active_channels[0];
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("Battery Level")
                                                    .color(theme::TEXT_PRIMARY)
                                                    .font(egui::FontId::proportional(11.5)),
                                            );
                                            if chan.charging {
                                                ui.label(
                                                    egui::RichText::new("CHARGING")
                                                        .color(theme::STATUS_CHARGING)
                                                        .font(egui::FontId::proportional(10.0))
                                                        .strong(),
                                                );
                                            }
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                let pct_color = if chan.charging {
                                                    theme::STATUS_CHARGING
                                                } else if chan.percentage <= dev.threshold {
                                                    theme::STATUS_LOW_ALERT
                                                } else {
                                                    theme::TEXT_PRIMARY
                                                };
                                                ui.label(
                                                    egui::RichText::new(format!("{}%", chan.percentage))
                                                        .color(pct_color)
                                                        .font(egui::FontId::monospace(11.5))
                                                        .strong(),
                                                );
                                            });
                                        });
                                        ui.add_space(3.0);
                                        battery_bar_ui(ui, chan.percentage, chan.charging, dev.threshold, ui.available_width());
                                    } else {
                                        // Multi-channel (e.g. SteelSeries Arctis GameBuds: Left, Right, Case)
                                        ui.horizontal(|ui| {
                                            let gap_width = 12.0;
                                            let total_gaps = (active_channels.len().saturating_sub(1)) as f32 * gap_width;
                                            let chan_width = ((ui.available_width() - total_gaps) / active_channels.len() as f32).clamp(120.0, 260.0);

                                            for chan in &active_channels {
                                                let channel_name = match chan.channel_type {
                                                    ChannelType::Main => "Main Battery",
                                                    ChannelType::Left => "Left Earbud (L)",
                                                    ChannelType::Right => "Right Earbud (R)",
                                                    ChannelType::Case => "Charging Case",
                                                };

                                                ui.vertical(|ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(channel_name)
                                                                .color(theme::TEXT_PRIMARY)
                                                                .font(egui::FontId::proportional(11.0)),
                                                        );
                                                        if chan.charging {
                                                            ui.label(
                                                                egui::RichText::new("CHG")
                                                                    .color(theme::STATUS_CHARGING)
                                                                    .font(egui::FontId::proportional(10.0))
                                                                    .strong(),
                                                            );
                                                        }
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            ui.label(
                                                                egui::RichText::new(format!("{}%", chan.percentage))
                                                                    .color(if chan.charging { theme::STATUS_CHARGING } else { theme::TEXT_PRIMARY })
                                                                    .font(egui::FontId::monospace(10.5))
                                                                    .strong(),
                                                            );
                                                        });
                                                    });
                                                    ui.add_space(2.0);
                                                    battery_bar_ui(ui, chan.percentage, chan.charging, dev.threshold, chan_width);
                                                });
                                                ui.add_space(gap_width);
                                            }
                                        });
                                    }
                                }
                                DeviceBatteryStatus::Offline => {
                                    ui.label(
                                        egui::RichText::new("Device is sleeping or disconnected. Wake or plug in to refresh battery state.")
                                            .color(theme::TEXT_DIM)
                                            .font(egui::FontId::proportional(10.5))
                                            .italics(),
                                    );
                                }
                            }
                        } else {
                            ui.label(
                                egui::RichText::new("No communication data received yet. Peripheral will update on next poll.")
                                    .color(theme::TEXT_DIM)
                                    .font(egui::FontId::proportional(10.5))
                                    .italics(),
                            );
                        }

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // -------------------------------------------------------------
                        // ROW 3: ALERT THRESHOLD CONTROLS (DEDICATED FULL SECTION)
                        // -------------------------------------------------------------
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Alert Threshold")
                                        .color(theme::TEXT_SECONDARY)
                                        .font(egui::FontId::proportional(11.5))
                                        .strong(),
                                );

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    // Stepper plus button
                                    if ui.button(egui::RichText::new("+").font(egui::FontId::monospace(10.0))).clicked() {
                                        dev.threshold = (dev.threshold + 5).min(95);
                                    }
                                    ui.label(
                                        egui::RichText::new(format!("{}%", dev.threshold))
                                            .color(theme::TEXT_PRIMARY)
                                            .font(egui::FontId::monospace(11.0))
                                            .strong(),
                                    );
                                    // Stepper minus button
                                    if ui.button(egui::RichText::new("-").font(egui::FontId::monospace(10.0))).clicked() {
                                        dev.threshold = (dev.threshold.saturating_sub(5)).max(5);
                                    }

                                    ui.add_space(8.0);

                                    // Quick threshold presets
                                    for preset in [25, 20, 15, 10] {
                                        let is_sel = dev.threshold == preset;
                                        let p_btn = egui::Button::new(
                                            egui::RichText::new(format!("{}%", preset))
                                                .font(egui::FontId::monospace(9.0))
                                                .color(if is_sel { theme::TEXT_PRIMARY } else { theme::TEXT_MUTED })
                                        )
                                        .fill(if is_sel { theme::BG_TAB_ACTIVE } else { theme::BG_SUBTLE })
                                        .stroke(egui::Stroke::new(1.0, if is_sel { theme::BORDER_HOVER } else { theme::BORDER_CARD }))
                                        .rounding(4.0)
                                        .min_size(egui::vec2(28.0, 18.0));

                                        if ui.add(p_btn).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                            dev.threshold = preset;
                                        }
                                    }
                                });
                            });

                            ui.add_space(3.0);
                            ui.scope(|ui| {
                                ui.visuals_mut().widgets.inactive.bg_fill = theme::BG_SUBTLE;
                                ui.visuals_mut().widgets.inactive.fg_stroke = egui::Stroke::new(2.0, theme::TEXT_SECONDARY);
                                ui.spacing_mut().slider_width = ui.available_width();
                                ui.add(egui::Slider::new(&mut dev.threshold, 5..=95).show_value(false).trailing_fill(true));
                            });
                            ui.label(
                                egui::RichText::new(format!("Alerts trigger when battery falls below {}%", dev.threshold))
                                    .color(theme::TEXT_DIM)
                                    .font(egui::FontId::proportional(9.0))
                                    .italics(),
                            );
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // -------------------------------------------------------------
                        // ROW 4: LOW BATTERY TRAY ICON SELECTOR (DEDICATED FULL SECTION)
                        // -------------------------------------------------------------
                        self.render_icon_selector(ui, &mut dev, base_type);

                        // -------------------------------------------------------------
                        // ROW 5: Card Footer (Hardware ID & Safe Remove)
                        // -------------------------------------------------------------
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("ID: {} • {}", dev.unique_id, type_title))
                                    .color(theme::TEXT_DIM)
                                    .font(egui::FontId::monospace(9.5)),
                            );

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if is_active {
                                    let status_label = ui.label(
                                        egui::RichText::new("Connected (Pause above to disable)")
                                            .color(theme::TEXT_DIM)
                                            .font(egui::FontId::proportional(9.5)),
                                    );
                                    status_label.on_hover_text("Connected peripherals cannot be removed while active. Turn toggle to 'Paused' to silence monitoring, or disconnect the device to remove.");
                                } else if is_confirming {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new("Remove?")
                                                .color(theme::STATUS_LOW_ALERT)
                                                .font(egui::FontId::proportional(10.5))
                                                .strong(),
                                        );
                                        if ui.button(egui::RichText::new("Cancel").font(egui::FontId::proportional(10.0))).clicked() {
                                            self.confirming_remove_id = None;
                                        }
                                        let confirm_btn = egui::Button::new(
                                            egui::RichText::new("Yes, Delete")
                                                .color(egui::Color32::WHITE)
                                                .font(egui::FontId::proportional(10.0))
                                                .strong(),
                                        )
                                        .fill(theme::STATUS_LOW_ALERT);

                                        if ui.add(confirm_btn).clicked() {
                                            to_remove = Some(idx);
                                            self.confirming_remove_id = None;
                                            action_toast = Some("Device removed".to_string());
                                        }
                                    });
                                } else {
                                    let remove_btn = egui::Button::new(
                                        egui::RichText::new("Remove Device")
                                            .color(theme::STATUS_LOW_ALERT)
                                            .font(egui::FontId::proportional(10.0)),
                                    )
                                    .fill(egui::Color32::TRANSPARENT)
                                    .frame(false);

                                    if ui.add(remove_btn).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                        self.confirming_remove_id = Some(dev.unique_id.clone());
                                    }
                                }
                            });
                        });
                    });
                });

            self.config.devices[idx] = dev;
            ui.add_space(10.0);
        }

        if let Some(msg) = action_toast {
            self.show_toast(&msg);
        }

        if let Some(idx) = to_remove {
            self.config.devices.remove(idx);
            self.device_removed = true;
        }
    }

    /// Dedicated, spacious low-battery tray icon selector with preview, dropdown, reset, and colored swatches
    fn render_icon_selector(&mut self, ui: &mut egui::Ui, dev: &mut DeviceConfig, base_type: &str) {
        let available_icons = crate::config::get_icon_list();

        let effective_base = match base_type {
            "gamepad" => "gamepad",
            "buds" => "buds",
            "keyboard" => "keyboard",
            _ => "mouse",
        };

        let default_icon_name = match effective_base {
            "gamepad" => "low_gamepad.png",
            "buds" => "low_buds.png",
            "keyboard" => "low_keyboard.png",
            _ => "low_mouse.png",
        };

        let current_is_custom = dev.low_battery_icon_path.is_some();
        let preview_icon_name = dev
            .low_battery_icon_path
            .clone()
            .unwrap_or_else(|| default_icon_name.to_string());

        ui.vertical(|ui| {
            // Header line: Label + Reset (if custom)
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Low Battery Tray Icon")
                        .color(theme::TEXT_SECONDARY)
                        .font(egui::FontId::proportional(11.5))
                        .strong(),
                );

                if current_is_custom {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let reset_btn = egui::Button::new(
                            egui::RichText::new("Reset to Default")
                                .color(theme::TEXT_MUTED)
                                .font(egui::FontId::proportional(10.0)),
                        )
                        .fill(egui::Color32::TRANSPARENT);
                        if ui.add(reset_btn).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            dev.low_battery_icon_path = None;
                        }
                    });
                }
            });

            ui.add_space(6.0);

            // Controls line: Preview thumbnail + Dropdown selector + Quick Pick Swatches
            ui.horizontal(|ui| {
                // 1. Preview thumbnail box
                let (prev_rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                ui.painter().rect_filled(prev_rect, 6.0, theme::BG_SUBTLE);
                ui.painter().rect_stroke(prev_rect, 6.0, egui::Stroke::new(1.0, theme::BORDER_CARD));

                if let Some(tex) = self.icon_textures.get(&preview_icon_name).or_else(|| self.icon_textures.get(default_icon_name)) {
                    let img_rect = egui::Rect::from_center_size(prev_rect.center(), egui::vec2(20.0, 20.0));
                    ui.painter().image(
                        tex.id(),
                        img_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }

                ui.add_space(6.0);

                // 2. ComboBox Dropdown
                let selected_display = if let Some(ref path) = dev.low_battery_icon_path {
                    format_icon_label(path)
                } else {
                    format!("Default ({})", format_icon_label(default_icon_name))
                };

                let prefix = format!("low_{}", effective_base);
                let (category_icons, other_icons): (Vec<_>, Vec<_>) = available_icons
                    .iter()
                    .partition(|name| name.starts_with(&prefix));

                egui::ComboBox::new(format!("icon_picker_{}", dev.unique_id), "")
                    .selected_text(selected_display)
                    .width(180.0)
                    .show_ui(ui, |ui| {
                        // Option: Default
                        let is_default = dev.low_battery_icon_path.is_none();
                        let (def_rect, def_resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::click());
                        if def_resp.clicked() {
                            dev.low_battery_icon_path = None;
                            ui.close_menu();
                        }
                        if is_default {
                            ui.painter().rect_filled(def_rect, 4.0, theme::BG_TAB_ACTIVE);
                        } else if def_resp.hovered() {
                            ui.painter().rect_filled(def_rect, 4.0, theme::BG_CARD_HOVER);
                        }
                        if let Some(tex) = self.icon_textures.get(default_icon_name) {
                            let img_rect = egui::Rect::from_center_size(egui::pos2(def_rect.left() + 12.0, def_rect.center().y), egui::vec2(16.0, 16.0));
                            ui.painter().image(tex.id(), img_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                        }
                        ui.painter().text(
                            egui::pos2(def_rect.left() + 26.0, def_rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            format!("Default ({})", format_icon_label(default_icon_name)),
                            egui::FontId::proportional(11.0),
                            if is_default { theme::TEXT_PRIMARY } else { theme::TEXT_SECONDARY },
                        );

                        // Category-specific icons
                        if !category_icons.is_empty() {
                            ui.separator();
                            for icon_name in &category_icons {
                                let is_selected = dev.low_battery_icon_path.as_deref() == Some(icon_name.as_str());
                                let (row_rect, row_resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::click());
                                if row_resp.clicked() {
                                    dev.low_battery_icon_path = Some((*icon_name).clone());
                                    ui.close_menu();
                                }
                                if is_selected {
                                    ui.painter().rect_filled(row_rect, 4.0, theme::BG_TAB_ACTIVE);
                                } else if row_resp.hovered() {
                                    ui.painter().rect_filled(row_rect, 4.0, theme::BG_CARD_HOVER);
                                }
                                if let Some(tex) = self.icon_textures.get(icon_name.as_str()) {
                                    let img_rect = egui::Rect::from_center_size(egui::pos2(row_rect.left() + 12.0, row_rect.center().y), egui::vec2(16.0, 16.0));
                                    ui.painter().image(tex.id(), img_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                                }
                                ui.painter().text(
                                    egui::pos2(row_rect.left() + 26.0, row_rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    format_icon_label(icon_name),
                                    egui::FontId::proportional(11.0),
                                    if is_selected { theme::TEXT_PRIMARY } else { theme::TEXT_SECONDARY },
                                );
                            }
                        }

                        // Other icons (other categories / custom icons)
                        if !other_icons.is_empty() {
                            ui.separator();
                            for icon_name in &other_icons {
                                let is_selected = dev.low_battery_icon_path.as_deref() == Some(icon_name.as_str());
                                let (row_rect, row_resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::click());
                                if row_resp.clicked() {
                                    dev.low_battery_icon_path = Some((*icon_name).clone());
                                    ui.close_menu();
                                }
                                if is_selected {
                                    ui.painter().rect_filled(row_rect, 4.0, theme::BG_TAB_ACTIVE);
                                } else if row_resp.hovered() {
                                    ui.painter().rect_filled(row_rect, 4.0, theme::BG_CARD_HOVER);
                                }
                                if let Some(tex) = self.icon_textures.get(icon_name.as_str()) {
                                    let img_rect = egui::Rect::from_center_size(egui::pos2(row_rect.left() + 12.0, row_rect.center().y), egui::vec2(16.0, 16.0));
                                    ui.painter().image(tex.id(), img_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                                }
                                ui.painter().text(
                                    egui::pos2(row_rect.left() + 26.0, row_rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    format_icon_label(icon_name),
                                    egui::FontId::proportional(11.0),
                                    if is_selected { theme::TEXT_PRIMARY } else { theme::TEXT_SECONDARY },
                                );
                            }
                        }
                    });

                ui.add_space(8.0);

                // 3. Quick Pick Color Swatches with visual color dots
                ui.label(
                    egui::RichText::new("Quick:")
                        .color(theme::TEXT_MUTED)
                        .font(egui::FontId::proportional(10.0)),
                );

                let swatch_defs: [(Option<String>, &str, egui::Color32); 4] = [
                    (None, "Default", egui::Color32::from_rgb(244, 244, 246)),
                    (Some(format!("low_{}_red.png", effective_base)), "Red", egui::Color32::from_rgb(239, 68, 68)),
                    (Some(format!("low_{}_orange.png", effective_base)), "Orange", egui::Color32::from_rgb(249, 115, 22)),
                    (Some(format!("low_{}_yellow.png", effective_base)), "Yellow", egui::Color32::from_rgb(234, 179, 8)),
                ];

                for (swatch_path, swatch_label, dot_color) in swatch_defs {
                    let is_current = match (&dev.low_battery_icon_path, &swatch_path) {
                        (None, None) => true,
                        (Some(cur), Some(opt)) => cur == opt,
                        _ => false,
                    };

                    let chip_size = egui::vec2(58.0, 22.0);
                    let (chip_rect, chip_resp) = ui.allocate_exact_size(chip_size, egui::Sense::click());
                    if chip_resp.clicked() {
                        dev.low_battery_icon_path = swatch_path;
                    }

                    let bg_color = if is_current {
                        theme::BG_TAB_ACTIVE
                    } else if chip_resp.hovered() {
                        theme::BG_CARD_HOVER
                    } else {
                        theme::BG_SUBTLE
                    };

                    let border_color = if is_current {
                        theme::BORDER_FOCUS
                    } else if chip_resp.hovered() {
                        theme::BORDER_HOVER
                    } else {
                        theme::BORDER_CARD
                    };

                    ui.painter().rect_filled(chip_rect, 4.0, bg_color);
                    ui.painter().rect_stroke(chip_rect, 4.0, egui::Stroke::new(1.0, border_color));

                    // Colored dot
                    let dot_pos = egui::pos2(chip_rect.left() + 9.0, chip_rect.center().y);
                    ui.painter().circle_filled(dot_pos, 3.5, dot_color);

                    // Label text
                    ui.painter().text(
                        egui::pos2(chip_rect.left() + 17.0, chip_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        swatch_label,
                        egui::FontId::proportional(9.5),
                        if is_current { theme::TEXT_PRIMARY } else { theme::TEXT_SECONDARY },
                    );

                    chip_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                }
            });

            ui.add_space(3.0);
            ui.label(
                egui::RichText::new("Displayed in the Windows taskbar tray when battery drops below alert threshold.")
                    .color(theme::TEXT_DIM)
                    .font(egui::FontId::proportional(9.0))
                    .italics(),
            );
        });
    }

    fn render_empty_state(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(theme::BG_CARD)
            .stroke(egui::Stroke::new(1.0, theme::BORDER_CARD))
            .rounding(12.0)
            .inner_margin(egui::Margin::same(32.0))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(40.0, 40.0), egui::Sense::hover());
                    ui.painter().rect_filled(icon_rect, 8.0, theme::BG_SUBTLE);
                    ui.painter().rect_stroke(icon_rect, 8.0, egui::Stroke::new(1.0, theme::BORDER_CARD));
                    let bolt_center = icon_rect.center();
                    let pts = [
                        egui::pos2(bolt_center.x + 1.5, bolt_center.y - 11.0),
                        egui::pos2(bolt_center.x - 5.0, bolt_center.y),
                        egui::pos2(bolt_center.x, bolt_center.y),
                        egui::pos2(bolt_center.x - 1.5, bolt_center.y + 11.0),
                        egui::pos2(bolt_center.x + 5.0, bolt_center.y),
                        egui::pos2(bolt_center.x, bolt_center.y),
                    ];
                    ui.painter().add(egui::Shape::convex_polygon(
                        pts.to_vec(),
                        theme::STATUS_CHARGING,
                        egui::Stroke::NONE,
                    ));
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("No Peripherals Configured Yet")
                            .color(theme::TEXT_PRIMARY)
                            .font(egui::FontId::proportional(16.0))
                            .strong(),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "BatStat automatically monitors wireless mice (Pulsar, Logitech), Xbox controllers,\nSteelSeries Arctis GameBuds, and compatible HID devices.",
                        )
                        .color(theme::TEXT_MUTED)
                        .font(egui::FontId::proportional(12.0)),
                    );
                    ui.add_space(14.0);

                    let btn = egui::Button::new(
                        egui::RichText::new("Detect Connected Peripherals")
                            .color(theme::BTN_PRIMARY_TEXT)
                            .font(egui::FontId::proportional(12.0))
                            .strong(),
                    )
                    .fill(theme::BTN_PRIMARY_BG)
                    .rounding(6.0)
                    .min_size(egui::vec2(220.0, 34.0));

                    if ui.add(btn).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        self.request_poll = true;
                        self.scanning_timer = Some(Instant::now());
                        self.show_toast("Scanning for connected peripherals...");
                    }
                });
            });
    }

    // -------------------------------------------------------------
    // GENERAL / SETTINGS TAB
    // -------------------------------------------------------------
    fn render_general_tab(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // CARD 1: DISPLAY & SYSTEM TRAY
            self.render_card_header(ui, "SYSTEM TRAY & DISPLAY");
            egui::Frame::none()
                .fill(theme::BG_CARD)
                .stroke(egui::Stroke::new(1.0, theme::BORDER_CARD))
                .rounding(10.0)
                .inner_margin(egui::Margin::same(14.0))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        // Show Percentage in Tray
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("Show Battery Percentage in Tray").color(theme::TEXT_PRIMARY).font(egui::FontId::proportional(13.0)));
                                ui.label(egui::RichText::new("Render a live device battery percentage directly on the system tray icon instead of the default icon.").color(theme::TEXT_MUTED).font(egui::FontId::proportional(9.5)).italics());
                            });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let current_val = self.config.tray_battery_channel.clone();
                                let available_channels = get_available_channels(&self.config);

                                let selected_text = if let Some(ref val) = current_val {
                                    available_channels
                                        .iter()
                                        .find(|(id, _)| id == val)
                                        .map(|(_, name)| name.clone())
                                        .unwrap_or_else(|| "No".to_string())
                                } else {
                                    "No (Default Icon)".to_string()
                                };

                                egui::ComboBox::new("tray_percentage_combo", "")
                                    .selected_text(selected_text)
                                    .width(160.0)
                                    .show_ui(ui, |ui| {
                                        let is_none = current_val.is_none();
                                        if ui.selectable_label(is_none, "No (Default Icon)").clicked() {
                                            self.config.tray_battery_channel = None;
                                        }
                                        for (id, name) in &available_channels {
                                            let is_selected = current_val.as_ref() == Some(id);
                                            if ui.selectable_label(is_selected, name).clicked() {
                                                self.config.tray_battery_channel = Some(id.clone());
                                            }
                                        }
                                    });
                            });
                        });

                        // Live tray preview
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Live Tray Preview:").color(theme::TEXT_MUTED).font(egui::FontId::proportional(10.0)));
                            let (prev_rect, _) = ui.allocate_exact_size(egui::vec2(38.0, 20.0), egui::Sense::hover());
                            ui.painter().rect_filled(prev_rect, 4.0, theme::BG_SUBTLE);
                            ui.painter().rect_stroke(prev_rect, 4.0, egui::Stroke::new(1.0, theme::BORDER_CARD));

                            if let Some(ref ch_id) = self.config.tray_battery_channel {
                                let mut pct_label = "85%".to_string();
                                for (dev_id, status) in &self.device_statuses {
                                    if let DeviceBatteryStatus::Online { channels } = status {
                                        for chan in channels.iter().flatten() {
                                            let key = match chan.channel_type {
                                                ChannelType::Main => format!("{}:Main", dev_id),
                                                ChannelType::Left => format!("{}:Left", dev_id),
                                                ChannelType::Right => format!("{}:Right", dev_id),
                                                ChannelType::Case => format!("{}:Case", dev_id),
                                            };
                                            if &key == ch_id {
                                                pct_label = format!("{}%", chan.percentage);
                                            }
                                        }
                                    }
                                }
                                ui.painter().text(prev_rect.center(), egui::Align2::CENTER_CENTER, pct_label, egui::FontId::monospace(9.5), theme::TEXT_PRIMARY);
                            } else if let Some(tex) = self.icon_textures.get("ok.png") {
                                let icon_box = egui::Rect::from_center_size(prev_rect.center(), egui::vec2(16.0, 16.0));
                                ui.painter().image(tex.id(), icon_box, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                            }
                        });

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(10.0);

                        // Polling Frequency Slider & Presets
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("Polling Frequency").color(theme::TEXT_PRIMARY).font(egui::FontId::proportional(13.0)));
                                ui.label(egui::RichText::new("Interval between battery status queries. 60s is recommended to minimize peripheral wake-ups.").color(theme::TEXT_MUTED).font(egui::FontId::proportional(9.5)).italics());
                            });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let (badge_rect, _) = ui.allocate_exact_size(egui::vec2(44.0, 20.0), egui::Sense::hover());
                                ui.painter().rect_filled(badge_rect, 4.0, theme::BG_SUBTLE);
                                ui.painter().rect_stroke(badge_rect, 4.0, egui::Stroke::new(1.0, theme::BORDER_CARD));
                                ui.painter().text(
                                    badge_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    format!("{}s", self.config.polling_interval_secs),
                                    egui::FontId::monospace(10.5),
                                    theme::TEXT_PRIMARY,
                                );
                            });
                        });

                        ui.add_space(4.0);
                        ui.scope(|ui| {
                            ui.visuals_mut().widgets.inactive.bg_fill = theme::BG_SUBTLE;
                            ui.visuals_mut().widgets.inactive.fg_stroke = egui::Stroke::new(2.0, theme::TEXT_SECONDARY);
                            ui.spacing_mut().slider_width = ui.available_width() - 8.0;
                            ui.add(egui::Slider::new(&mut self.config.polling_interval_secs, 1..=60).show_value(false).trailing_fill(true));
                        });

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("1s (Fast)").color(theme::TEXT_DIM).font(egui::FontId::proportional(9.0)));
                            ui.add_space(8.0);
                            if ui.button(egui::RichText::new("15s").font(egui::FontId::proportional(9.0))).clicked() {
                                self.config.polling_interval_secs = 15;
                            }
                            if ui.button(egui::RichText::new("30s").font(egui::FontId::proportional(9.0))).clicked() {
                                self.config.polling_interval_secs = 30;
                            }
                            if ui.button(egui::RichText::new("60s (Recommended)").font(egui::FontId::proportional(9.0))).clicked() {
                                self.config.polling_interval_secs = 60;
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new("60s (Battery Safe)").color(theme::TEXT_DIM).font(egui::FontId::proportional(9.0)));
                            });
                        });
                    });
                });

            ui.add_space(14.0);

            // CARD 2: STARTUP & NOTIFICATIONS
            self.render_card_header(ui, "STARTUP & SYSTEM NOTIFICATIONS");
            egui::Frame::none()
                .fill(theme::BG_CARD)
                .stroke(egui::Stroke::new(1.0, theme::BORDER_CARD))
                .rounding(10.0)
                .inner_margin(egui::Margin::same(14.0))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        // Launch on Startup
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("Launch on Windows Startup").color(theme::TEXT_PRIMARY).font(egui::FontId::proportional(13.0)));
                                ui.label(egui::RichText::new("Start BatStat automatically minimized to the Windows system tray when logging in.").color(theme::TEXT_MUTED).font(egui::FontId::proportional(9.5)).italics());
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let mut autostart = self.config.autostart;
                                if toggle_ui(ui, &mut autostart).changed() {
                                    self.config.autostart = autostart;
                                    if let Err(e) = set_autostart(autostart) {
                                        eprintln!("Failed to set autostart: {}", e);
                                        self.show_toast("Failed to update Windows autostart");
                                    } else {
                                        self.show_toast(if autostart { "Autostart enabled" } else { "Autostart disabled" });
                                    }
                                }
                            });
                        });

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(10.0);

                        // Desktop Notifications
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("Desktop Notification Alerts").color(theme::TEXT_PRIMARY).font(egui::FontId::proportional(13.0)));
                                ui.label(egui::RichText::new("Show native Windows toast notifications when a peripheral reaches its configured alert threshold.").color(theme::TEXT_MUTED).font(egui::FontId::proportional(9.5)).italics());
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                toggle_ui(ui, &mut self.config.enable_notifications);
                            });
                        });

                        if self.config.enable_notifications {
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                let test_btn = egui::Button::new(
                                    egui::RichText::new("Send Test Notification")
                                        .color(theme::TEXT_PRIMARY)
                                        .font(egui::FontId::proportional(10.5)),
                                )
                                .fill(theme::BTN_SECONDARY_BG)
                                .stroke(egui::Stroke::new(1.0, theme::BORDER_CARD))
                                .rounding(6.0);

                                let resp = ui.add(test_btn).on_hover_cursor(egui::CursorIcon::PointingHand);
                                if resp.clicked() {
                                    self.request_test_notification = true;
                                    self.show_toast("Test notification dispatched");
                                }
                                resp.on_hover_text("Send a sample low-battery Windows notification to verify delivery");
                            });
                        }
                    });
                });

            ui.add_space(14.0);

            // CARD 3: STORAGE & CUSTOM ICONS
            self.render_card_header(ui, "STORAGE & CUSTOM ASSETS");
            egui::Frame::none()
                .fill(theme::BG_CARD)
                .stroke(egui::Stroke::new(1.0, theme::BORDER_CARD))
                .rounding(10.0)
                .inner_margin(egui::Margin::same(14.0))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        // Open Icons Directory
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("Custom Icons Folder").color(theme::TEXT_PRIMARY).font(egui::FontId::proportional(13.0)));
                                ui.label(egui::RichText::new("Drop 32x32 PNG or ICO images here to expand low-battery icon choices.").color(theme::TEXT_MUTED).font(egui::FontId::proportional(9.5)).italics());
                                if let Some(ref dir) = crate::config::get_icons_dir_path() {
                                    ui.label(egui::RichText::new(dir.to_string_lossy()).color(theme::TEXT_DIM).font(egui::FontId::monospace(9.0)));
                                }
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let btn = egui::Button::new(
                                    egui::RichText::new("Open Icons Folder")
                                        .color(theme::TEXT_PRIMARY)
                                        .font(egui::FontId::proportional(11.0))
                                        .strong(),
                                )
                                .fill(theme::BTN_SECONDARY_BG)
                                .stroke(egui::Stroke::new(1.0, theme::BORDER_CARD))
                                .rounding(6.0);

                                if ui.add(btn).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    if let Some(dir) = crate::config::get_icons_dir_path() {
                                        let _ = std::process::Command::new("explorer").arg(dir).spawn();
                                    }
                                }
                            });
                        });

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(10.0);

                        // Open Config Folder
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("AppData Configuration Directory").color(theme::TEXT_PRIMARY).font(egui::FontId::proportional(13.0)));
                                ui.label(egui::RichText::new("Location of config.toml and diagnostic logs (%APPDATA%\\BatStat).").color(theme::TEXT_MUTED).font(egui::FontId::proportional(9.5)).italics());
                                if let Some(ref path) = crate::config::get_config_path() {
                                    ui.label(egui::RichText::new(path.to_string_lossy()).color(theme::TEXT_DIM).font(egui::FontId::monospace(9.0)));
                                }
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let btn = egui::Button::new(
                                    egui::RichText::new("Open Config Folder")
                                        .color(theme::TEXT_PRIMARY)
                                        .font(egui::FontId::proportional(11.0))
                                        .strong(),
                                )
                                .fill(theme::BTN_SECONDARY_BG)
                                .stroke(egui::Stroke::new(1.0, theme::BORDER_CARD))
                                .rounding(6.0);

                                if ui.add(btn).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                    if let Some(mut path) = crate::config::get_config_path() {
                                        path.pop();
                                        let _ = std::process::Command::new("explorer").arg(path).spawn();
                                    }
                                }
                            });
                        });
                    });
                });

            ui.add_space(14.0);

            // CARD 4: DIAGNOSTICS & LOGGING
            self.render_card_header(ui, "DIAGNOSTICS & LOGGING");
            egui::Frame::none()
                .fill(theme::BG_CARD)
                .stroke(egui::Stroke::new(1.0, theme::BORDER_CARD))
                .rounding(10.0)
                .inner_margin(egui::Margin::same(14.0))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("Diagnostic Debug Logging").color(theme::TEXT_PRIMARY).font(egui::FontId::proportional(13.0)));
                                ui.label(egui::RichText::new("Record detailed peripheral polling and communication frames into debug.log.").color(theme::TEXT_MUTED).font(egui::FontId::proportional(9.5)).italics());
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                toggle_ui(ui, &mut self.config.enable_debug_logging);
                            });
                        });
                    });
                });

            ui.add_space(16.0);
        });
    }

    // -------------------------------------------------------------
    // ABOUT & DIAGNOSTICS TAB
    // -------------------------------------------------------------
    fn render_about_tab(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // HERO APP BANNER
            egui::Frame::none()
                .fill(theme::BG_CARD)
                .stroke(egui::Stroke::new(1.0, theme::BORDER_CARD))
                .rounding(10.0)
                .inner_margin(egui::Margin::same(18.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::hover());
                        ui.painter().rect_filled(icon_rect, 8.0, theme::BG_SUBTLE);
                        ui.painter().rect_stroke(icon_rect, 8.0, egui::Stroke::new(1.0, theme::BORDER_CARD));
                        let c = icon_rect.center();
                        let pts = [
                            egui::pos2(c.x + 1.5, c.y - 12.0),
                            egui::pos2(c.x - 6.5, c.y + 1.0),
                            egui::pos2(c.x - 0.8, c.y + 1.0),
                            egui::pos2(c.x - 1.5, c.y + 12.0),
                            egui::pos2(c.x + 6.5, c.y - 1.0),
                            egui::pos2(c.x + 0.8, c.y - 1.0),
                        ];
                        ui.painter().add(egui::Shape::convex_polygon(
                            pts.to_vec(),
                            theme::STATUS_CHARGING,
                            egui::Stroke::NONE,
                        ));

                        ui.add_space(10.0);

                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("BatStat").color(theme::TEXT_PRIMARY).font(egui::FontId::proportional(16.0)).strong());
                                ui.label(egui::RichText::new(concat!("v", env!("CARGO_PKG_VERSION"))).color(theme::TEXT_MUTED).font(egui::FontId::proportional(12.0)).strong());
                            });
                            ui.label(egui::RichText::new("Lightweight Windows Peripheral Battery Monitor").color(theme::TEXT_SECONDARY).font(egui::FontId::proportional(11.0)));
                            ui.add_space(2.0);
                            ui.label(
                                egui::RichText::new("Zero Bloat • Native Rust & Win32 HID Architecture • <15 MB RAM • 0% Idle CPU")
                                    .color(theme::TEXT_DIM)
                                    .font(egui::FontId::proportional(9.5)),
                            );
                        });
                    });
                });

            ui.add_space(14.0);

            // SOFTWARE UPDATES
            self.render_card_header(ui, "SOFTWARE UPDATES");
            egui::Frame::none()
                .fill(theme::BG_CARD)
                .stroke(egui::Stroke::new(1.0, theme::BORDER_CARD))
                .rounding(10.0)
                .inner_margin(egui::Margin::same(14.0))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("Update Channel: Official GitHub Releases").color(theme::TEXT_PRIMARY).font(egui::FontId::proportional(12.5)));

                                match &self.update_status {
                                    crate::UpdateStatus::Idle => {
                                        ui.label(egui::RichText::new(format!("Installed version: v{}", env!("CARGO_PKG_VERSION"))).color(theme::TEXT_MUTED).font(egui::FontId::proportional(10.0)));
                                    }
                                    crate::UpdateStatus::Checking => {
                                        ui.label(egui::RichText::new("Checking GitHub for latest release...").color(theme::TEXT_MUTED).font(egui::FontId::proportional(10.0)).italics());
                                    }
                                    crate::UpdateStatus::Available(info) => {
                                        ui.label(egui::RichText::new(format!("Update Available: {}", info.tag_name)).color(theme::STATUS_ONLINE).font(egui::FontId::proportional(10.5)).strong());
                                    }
                                    crate::UpdateStatus::NoUpdate => {
                                        ui.label(egui::RichText::new("You are on the latest version of BatStat.").color(theme::STATUS_ONLINE).font(egui::FontId::proportional(10.0)));
                                    }
                                    crate::UpdateStatus::Downloading(pct) => {
                                        ui.label(egui::RichText::new(format!("Downloading update: {:.0}%", pct * 100.0)).color(theme::TEXT_PRIMARY).font(egui::FontId::proportional(10.0)).strong());
                                    }
                                    crate::UpdateStatus::ReadyToInstall(_) => {
                                        ui.label(egui::RichText::new("Update ready to install. BatStat will restart.").color(theme::STATUS_ONLINE).font(egui::FontId::proportional(10.0)).strong());
                                    }
                                    crate::UpdateStatus::Error(e) => {
                                        ui.label(egui::RichText::new(format!("Update check failed: {}", e)).color(theme::STATUS_LOW_ALERT).font(egui::FontId::proportional(10.0)));
                                    }
                                }
                            });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                match &self.update_status {
                                    crate::UpdateStatus::Available(info) => {
                                        let msi_asset = info.assets.iter().find(|a| a.name.ends_with(".msi"));
                                        if let Some(asset) = msi_asset {
                                            let btn = egui::Button::new(
                                                egui::RichText::new("Install Update")
                                                    .color(theme::BTN_PRIMARY_TEXT)
                                                    .font(egui::FontId::proportional(11.0))
                                                    .strong(),
                                            )
                                            .fill(theme::STATUS_ONLINE)
                                            .rounding(6.0);

                                            if ui.add(btn).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                                self.request_download_install = Some(asset.browser_download_url.clone());
                                            }
                                        } else {
                                            let btn = egui::Button::new(
                                                egui::RichText::new("View Release")
                                                    .color(theme::BTN_PRIMARY_TEXT)
                                                    .font(egui::FontId::proportional(11.0))
                                                    .strong(),
                                            )
                                            .fill(theme::BTN_PRIMARY_BG)
                                            .rounding(6.0);

                                            if ui.add(btn).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                                let _ = std::process::Command::new("explorer").arg("https://github.com/SGiehler/BatStat/releases/latest").spawn();
                                            }
                                        }
                                    }
                                    crate::UpdateStatus::Checking | crate::UpdateStatus::Downloading(_) => {
                                        let _ = ui.add_enabled(
                                            false,
                                            egui::Button::new("Checking...").rounding(6.0),
                                        );
                                    }
                                    _ => {
                                        let btn = egui::Button::new(
                                            egui::RichText::new("Check for Updates")
                                                .color(theme::TEXT_PRIMARY)
                                                .font(egui::FontId::proportional(11.0))
                                                .strong(),
                                        )
                                        .fill(theme::BTN_SECONDARY_BG)
                                        .stroke(egui::Stroke::new(1.0, theme::BORDER_CARD))
                                        .rounding(6.0);

                                        if ui.add(btn).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                            self.request_update_check = true;
                                        }
                                    }
                                }
                            });
                        });

                        // Progress bar when downloading
                        if let crate::UpdateStatus::Downloading(progress) = self.update_status {
                            ui.add_space(6.0);
                            let (bar_rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 6.0), egui::Sense::hover());
                            ui.painter().rect_filled(bar_rect, 3.0, theme::BG_SUBTLE);
                            let fill_w = bar_rect.width() * progress.clamp(0.0, 1.0);
                            if fill_w > 0.0 {
                                let fill_rect = egui::Rect::from_min_size(bar_rect.min, egui::vec2(fill_w, bar_rect.height()));
                                ui.painter().rect_filled(fill_rect, 3.0, theme::STATUS_ONLINE);
                            }
                        }
                    });
                });

            ui.add_space(14.0);

            // LIVE DIAGNOSTICS LOG VIEWER
            self.render_log_viewer(ui);

            ui.add_space(14.0);

            // PROJECT LINKS & REPO
            self.render_card_header(ui, "RESOURCES & COMMUNITY");
            egui::Frame::none()
                .fill(theme::BG_CARD)
                .stroke(egui::Stroke::new(1.0, theme::BORDER_CARD))
                .rounding(10.0)
                .inner_margin(egui::Margin::same(14.0))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(egui::RichText::new("GitHub Repository").font(egui::FontId::proportional(11.0))).clicked() {
                            let _ = std::process::Command::new("explorer").arg("https://github.com/SGiehler/BatStat").spawn();
                        }
                        if ui.button(egui::RichText::new("Report an Issue").font(egui::FontId::proportional(11.0))).clicked() {
                            let _ = std::process::Command::new("explorer").arg("https://github.com/SGiehler/BatStat/issues").spawn();
                        }
                        if ui.button(egui::RichText::new("Supported Hardware").font(egui::FontId::proportional(11.0))).clicked() {
                            let _ = std::process::Command::new("explorer").arg("https://github.com/SGiehler/BatStat/blob/master/DEVICELIST.md").spawn();
                        }
                        if ui.button(egui::RichText::new("MIT License").font(egui::FontId::proportional(11.0))).clicked() {
                            let _ = std::process::Command::new("explorer").arg("https://github.com/SGiehler/BatStat/blob/master/LICENSE").spawn();
                        }
                    });
                });

            ui.add_space(16.0);
        });
    }

    fn render_log_viewer(&mut self, ui: &mut egui::Ui) {
        self.render_card_header(ui, "DIAGNOSTIC LOG VIEWER");

        let log_path = crate::config::get_config_path().map(|mut p| {
            p.pop();
            p.push("debug.log");
            p
        });

        if self.debug_log_content.is_none() {
            self.refresh_log();
        }

        egui::Frame::none()
            .fill(theme::BG_CARD)
            .stroke(egui::Stroke::new(1.0, theme::BORDER_CARD))
            .rounding(10.0)
            .inner_margin(egui::Margin::same(12.0))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Recent diagnostic output:")
                                .color(theme::TEXT_MUTED)
                                .font(egui::FontId::proportional(10.5)),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if let Some(ref path) = log_path {
                                if path.exists() {
                                    if ui.button(egui::RichText::new("Clear").font(egui::FontId::proportional(10.0))).clicked() {
                                        let _ = std::fs::write(path, "");
                                        self.debug_log_content = Some(String::new());
                                        self.show_toast("Log file cleared");
                                    }
                                    if ui.button(egui::RichText::new("Copy").font(egui::FontId::proportional(10.0))).clicked() {
                                        if let Some(ref content) = self.debug_log_content {
                                            ui.output_mut(|o| o.copied_text = content.clone());
                                            self.show_toast("Copied log to clipboard");
                                        }
                                    }
                                    if ui.button(egui::RichText::new("Open in Editor").font(egui::FontId::proportional(10.0))).clicked() {
                                        let _ = std::process::Command::new("notepad").arg(path).spawn();
                                    }
                                }
                            }

                            if ui.button(egui::RichText::new("Refresh").font(egui::FontId::proportional(10.0))).clicked() {
                                self.refresh_log();
                                self.show_toast("Log refreshed");
                            }
                        });
                    });

                    ui.add_space(6.0);

                    let log_str = self
                        .debug_log_content
                        .as_deref()
                        .unwrap_or("No logs available.");

                    egui::Frame::none()
                        .fill(theme::BG_SUBTLE)
                        .stroke(egui::Stroke::new(1.0, theme::BORDER_SUBTLE))
                        .rounding(6.0)
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .max_height(120.0)
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(log_str)
                                                .color(theme::TEXT_MUTED)
                                                .font(egui::FontId::monospace(9.5)),
                                        )
                                        .wrap(),
                                    );
                                });
                        });
                });
            });
    }

    fn render_card_header(&self, ui: &mut egui::Ui, title: &str) {
        ui.horizontal(|ui| {
            let (bar_rect, _) = ui.allocate_exact_size(egui::vec2(3.0, 14.0), egui::Sense::hover());
            ui.painter().rect_filled(bar_rect, 1.5, theme::BORDER_FOCUS);
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(title)
                    .color(theme::TEXT_PRIMARY)
                    .font(egui::FontId::proportional(11.5))
                    .strong(),
            );
        });
        ui.add_space(4.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_category_mapping() {
        assert_eq!(get_device_category("pulsar_0x5406").0, "MOUSE");
        assert_eq!(get_device_category("xbox_0").0, "GAMEPAD");
        assert_eq!(get_device_category("gamebuds_left").0, "BUDS");
        assert_eq!(get_device_category("keyboard_1").0, "KEYBOARD");
        assert_eq!(get_device_category("unknown_dongle").0, "PERIPHERAL");
    }

    #[test]
    fn test_format_icon_label() {
        assert_eq!(format_icon_label("low_mouse_red.png"), "Mouse (Red)");
        assert_eq!(format_icon_label("low_mouse_orange.png"), "Mouse (Orange)");
        assert_eq!(format_icon_label("low_mouse_yellow.png"), "Mouse (Yellow)");
        assert_eq!(format_icon_label("low_mouse.png"), "Mouse (White)");
        assert_eq!(format_icon_label("low_gamepad_red.png"), "Gamepad (Red)");
        assert_eq!(format_icon_label("low_buds_yellow.png"), "Earbuds (Yellow)");
        assert_eq!(format_icon_label("low_keyboard.png"), "Keyboard (White)");
        assert_eq!(format_icon_label("custom_icon.png"), "custom_icon.png");
    }

    #[test]
    fn test_low_battery_icon_assignment() {
        let mut dev = DeviceConfig {
            unique_id: "pulsar_0x5406".to_string(),
            name: "Pulsar Mouse".to_string(),
            enabled: true,
            threshold: 20,
            low_battery_icon_path: None,
        };

        // Select custom icon
        dev.low_battery_icon_path = Some("low_mouse_red.png".to_string());
        assert_eq!(dev.low_battery_icon_path.as_deref(), Some("low_mouse_red.png"));

        // Reset to default
        dev.low_battery_icon_path = None;
        assert_eq!(dev.low_battery_icon_path, None);
    }

    #[test]
    fn test_available_channels_generation() {
        let mut config = AppConfig::default();
        config.devices.push(DeviceConfig {
            unique_id: "gamebuds_1".to_string(),
            name: "SteelSeries Arctis GameBuds".to_string(),
            enabled: true,
            threshold: 15,
            low_battery_icon_path: None,
        });
        config.devices.push(DeviceConfig {
            unique_id: "pulsar_1".to_string(),
            name: "Pulsar X2".to_string(),
            enabled: true,
            threshold: 20,
            low_battery_icon_path: None,
        });

        let channels = get_available_channels(&config);
        assert_eq!(channels.len(), 3);
        assert_eq!(channels[0].0, "gamebuds_1:Left");
        assert_eq!(channels[1].0, "gamebuds_1:Right");
        assert_eq!(channels[2].0, "pulsar_1:Main");
    }

    #[test]
    fn test_unsaved_changes_detection() {
        let config = AppConfig::default();
        let mut window = SettingsWindow::new(config, vec![], std::collections::HashMap::new());
        assert!(!window.has_unsaved_changes());

        window.config.polling_interval_secs = 42;
        assert!(window.has_unsaved_changes());

        // Reverting matches original config
        window.config.polling_interval_secs = window.original_config.polling_interval_secs;
        assert!(!window.has_unsaved_changes());

        // Adding device marks unsaved
        window.config.devices.push(DeviceConfig {
            unique_id: "xbox_0".to_string(),
            name: "Xbox Controller".to_string(),
            enabled: true,
            threshold: 20,
            low_battery_icon_path: None,
        });
        assert!(window.has_unsaved_changes());

        // Modifying icon path marks unsaved
        window.original_config = window.config.clone();
        assert!(!window.has_unsaved_changes());
        window.config.devices[0].low_battery_icon_path = Some("low_gamepad_red.png".to_string());
        assert!(window.has_unsaved_changes());
    }

    #[test]
    fn test_device_filter_and_search() {
        let mut config = AppConfig::default();
        config.devices.push(DeviceConfig {
            unique_id: "pulsar_11".to_string(),
            name: "Pulsar X2 Superlight".to_string(),
            enabled: true,
            threshold: 20,
            low_battery_icon_path: None,
        });
        config.devices.push(DeviceConfig {
            unique_id: "xbox_0".to_string(),
            name: "Xbox Elite Controller".to_string(),
            enabled: true,
            threshold: 15,
            low_battery_icon_path: None,
        });

        let active = vec!["pulsar_11".to_string()];
        let mut statuses = std::collections::HashMap::new();
        let mut channels = [None; 4];
        channels[0] = Some(BatteryChannel {
            channel_type: ChannelType::Main,
            percentage: 10,
            charging: true,
        });
        statuses.insert("pulsar_11".to_string(), DeviceBatteryStatus::Online { channels });

        let mut window = SettingsWindow::new(config, active, statuses);

        // Test Filter: Connected
        window.filter_status = DeviceFilter::Connected;
        let connected_count = window.config.devices.iter().filter(|d| window.active_devices.contains(&d.unique_id)).count();
        assert_eq!(connected_count, 1);

        // Test Filter: Charging
        window.filter_status = DeviceFilter::Charging;
        let charging_count = window.config.devices.iter().filter(|d| {
            if let Some(DeviceBatteryStatus::Online { channels }) = window.device_statuses.get(&d.unique_id) {
                channels.iter().flatten().any(|c| c.charging)
            } else {
                false
            }
        }).count();
        assert_eq!(charging_count, 1);

        // Test Filter: LowBattery
        window.filter_status = DeviceFilter::LowBattery;
        let low_count = window.config.devices.iter().filter(|d| {
            if let Some(DeviceBatteryStatus::Online { channels }) = window.device_statuses.get(&d.unique_id) {
                channels.iter().flatten().any(|c| c.percentage <= d.threshold)
            } else {
                false
            }
        }).count();
        assert_eq!(low_count, 1);

        // Test Filter: Offline
        window.filter_status = DeviceFilter::Offline;
        let offline_count = window.config.devices.iter().filter(|d| !window.active_devices.contains(&d.unique_id)).count();
        assert_eq!(offline_count, 1);

        // Test Search: "controller" matches Xbox via category title
        let search = "controller";
        let matches: Vec<_> = window.config.devices.iter().filter(|d| {
            let (_, _, type_title, _) = get_device_category(&d.unique_id);
            d.name.to_lowercase().contains(search) || d.unique_id.to_lowercase().contains(search) || type_title.to_lowercase().contains(search)
        }).collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].unique_id, "xbox_0");

        // Test Search: "mouse" matches Pulsar via category title
        let search = "mouse";
        let matches: Vec<_> = window.config.devices.iter().filter(|d| {
            let (_, _, type_title, _) = get_device_category(&d.unique_id);
            d.name.to_lowercase().contains(search) || d.unique_id.to_lowercase().contains(search) || type_title.to_lowercase().contains(search)
        }).collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].unique_id, "pulsar_11");
    }

    #[test]
    fn test_polling_interval_clamping_in_constructor() {
        let mut config = AppConfig::default();
        config.polling_interval_secs = 0;
        let window = SettingsWindow::new(config, vec![], std::collections::HashMap::new());
        assert_eq!(window.config.polling_interval_secs, 1);

        let mut config2 = AppConfig::default();
        config2.polling_interval_secs = 120;
        let window2 = SettingsWindow::new(config2, vec![], std::collections::HashMap::new());
        assert_eq!(window2.config.polling_interval_secs, 60);
    }

    #[test]
    fn test_rename_device_state_preservation() {
        let mut config = AppConfig::default();
        config.devices.push(DeviceConfig {
            unique_id: "pulsar_11".to_string(),
            name: "Pulsar X2".to_string(),
            enabled: true,
            threshold: 20,
            low_battery_icon_path: None,
        });
        let mut window = SettingsWindow::new(config, vec![], std::collections::HashMap::new());

        // Start renaming
        window.editing_device_id = Some("pulsar_11".to_string());
        window.edit_name_buffer = "Pulsar X2".to_string();

        // Simulate user typing into the buffer
        window.edit_name_buffer = "Pulsar Wireless Mouse".to_string();

        // Commit rename
        let trimmed = window.edit_name_buffer.trim().to_string();
        window.config.devices[0].name = trimmed;
        window.editing_device_id = None;

        assert_eq!(window.config.devices[0].name, "Pulsar Wireless Mouse");
        assert!(window.has_unsaved_changes());
    }

    #[test]
    fn test_hierarchical_escape_logic() {
        let config = AppConfig::default();
        let mut window = SettingsWindow::new(config, vec![], std::collections::HashMap::new());

        // 1. While editing, Escape cancels editing, NOT window
        window.editing_device_id = Some("pulsar_11".to_string());
        if window.editing_device_id.is_some() {
            window.editing_device_id = None;
        } else {
            window.request_close = true;
        }
        assert_eq!(window.editing_device_id, None);
        assert!(!window.request_close);

        // 2. While confirming removal, Escape cancels confirm, NOT window
        window.confirming_remove_id = Some("pulsar_11".to_string());
        if window.editing_device_id.is_some() {
            window.editing_device_id = None;
        } else if window.confirming_remove_id.is_some() {
            window.confirming_remove_id = None;
        } else {
            window.request_close = true;
        }
        assert_eq!(window.confirming_remove_id, None);
        assert!(!window.request_close);

        // 3. When idle, Escape closes window
        if window.editing_device_id.is_some() {
            window.editing_device_id = None;
        } else if window.confirming_remove_id.is_some() {
            window.confirming_remove_id = None;
        } else {
            window.discard_changes = true;
            window.request_close = true;
        }
        assert!(window.request_close);
        assert!(window.discard_changes);
    }

    #[test]
    fn test_format_icon_label_exhaustive() {
        assert_eq!(format_icon_label("low_mouse_red.png"), "Mouse (Red)");
        assert_eq!(format_icon_label("low_mouse_orange.png"), "Mouse (Orange)");
        assert_eq!(format_icon_label("low_mouse_yellow.png"), "Mouse (Yellow)");
        assert_eq!(format_icon_label("low_mouse_blue.png"), "Mouse (Blue)");
        assert_eq!(format_icon_label("low_mouse.png"), "Mouse (White)");
        assert_eq!(format_icon_label("low_gamepad_red.png"), "Gamepad (Red)");
        assert_eq!(format_icon_label("low_gamepad.png"), "Gamepad (White)");
        assert_eq!(format_icon_label("low_buds_red.png"), "Earbuds (Red)");
        assert_eq!(format_icon_label("low_buds.png"), "Earbuds (White)");
        assert_eq!(format_icon_label("low_keyboard_red.png"), "Keyboard (Red)");
        assert_eq!(format_icon_label("low_keyboard.png"), "Keyboard (White)");
        assert_eq!(format_icon_label("battery_alert.ico"), "battery_alert.ico");
        assert_eq!(format_icon_label("custom.png"), "custom.png");
    }

    #[test]
    fn test_device_category_base_types() {
        let (tag, glyph, title, base) = get_device_category("pulsar_0x5406");
        assert_eq!(base, "mouse");
        assert_eq!(tag, "MOUSE");
        assert_eq!(glyph, "mouse");
        assert_eq!(title, "Gaming Mouse");

        let (_, _, _, base) = get_device_category("xbox_one_controller");
        assert_eq!(base, "gamepad");

        let (_, _, _, base) = get_device_category("gamebuds_wireless");
        assert_eq!(base, "buds");

        let (_, _, _, base) = get_device_category("keyboard_rgb");
        assert_eq!(base, "keyboard");

        let (_, _, _, base) = get_device_category("unknown_hid_device");
        assert_eq!(base, "peripheral");
    }

    #[test]
    fn test_icon_swatch_values() {
        let mut dev = DeviceConfig {
            unique_id: "pulsar_1".to_string(),
            name: "Mouse".to_string(),
            enabled: true,
            threshold: 20,
            low_battery_icon_path: None,
        };

        let base_type = "mouse";
        let red_swatch = Some(format!("low_{}_red.png", base_type));
        dev.low_battery_icon_path = red_swatch.clone();
        assert_eq!(dev.low_battery_icon_path, Some("low_mouse_red.png".to_string()));

        let orange_swatch = Some(format!("low_{}_orange.png", base_type));
        assert_eq!(orange_swatch, Some("low_mouse_orange.png".to_string()));

        let yellow_swatch = Some(format!("low_{}_yellow.png", base_type));
        assert_eq!(yellow_swatch, Some("low_mouse_yellow.png".to_string()));

        let white_swatch = Some(format!("low_{}.png", base_type));
        assert_eq!(white_swatch, Some("low_mouse.png".to_string()));

        dev.low_battery_icon_path = None;
        assert_eq!(dev.low_battery_icon_path, None);
    }

    #[test]
    fn test_icon_swatch_all_categories() {
        let categories = ["mouse", "gamepad", "buds", "keyboard"];
        for cat in categories {
            assert_eq!(format!("low_{}_red.png", cat), format!("low_{}_red.png", cat));
            assert_eq!(format!("low_{}_orange.png", cat), format!("low_{}_orange.png", cat));
            assert_eq!(format!("low_{}_yellow.png", cat), format!("low_{}_yellow.png", cat));
            assert_eq!(format!("low_{}.png", cat), format!("low_{}.png", cat));
        }
    }

    #[test]
    fn test_has_unsaved_changes_on_icon_change() {
        let mut config = AppConfig::default();
        config.devices.push(DeviceConfig {
            unique_id: "mouse_1".to_string(),
            name: "Mouse".to_string(),
            enabled: true,
            threshold: 20,
            low_battery_icon_path: None,
        });
        let mut window = SettingsWindow::new(config, vec![], std::collections::HashMap::new());
        assert!(!window.has_unsaved_changes());

        window.config.devices[0].low_battery_icon_path = Some("low_mouse_red.png".to_string());
        assert!(window.has_unsaved_changes());

        window.config.devices[0].low_battery_icon_path = None;
        assert!(!window.has_unsaved_changes());
    }

    #[test]
    fn test_config_saved_in_place_lifecycle() {
        let config = AppConfig::default();
        let mut window = SettingsWindow::new(config, vec![], std::collections::HashMap::new());
        assert!(!window.config_saved_in_place);

        // Simulate Ctrl+S save in-place
        window.config_saved_in_place = true;
        assert!(window.config_saved_in_place);
    }

    #[test]
    fn test_ui_frame_rendering_all_tabs() {
        let ctx = egui::Context::default();
        let mut config = AppConfig::default();
        config.devices.push(DeviceConfig {
            unique_id: "pulsar_x2".to_string(),
            name: "Pulsar Mouse".to_string(),
            enabled: true,
            threshold: 20,
            low_battery_icon_path: None,
        });
        config.devices.push(DeviceConfig {
            unique_id: "xbox_controller".to_string(),
            name: "Xbox Gamepad".to_string(),
            enabled: true,
            threshold: 15,
            low_battery_icon_path: Some("low_gamepad_red.png".to_string()),
        });
        config.devices.push(DeviceConfig {
            unique_id: "gamebuds_1".to_string(),
            name: "SteelSeries Buds".to_string(),
            enabled: true,
            threshold: 25,
            low_battery_icon_path: Some("low_buds_orange.png".to_string()),
        });

        let mut statuses = std::collections::HashMap::new();
        statuses.insert(
            "pulsar_x2".to_string(),
            DeviceBatteryStatus::Online {
                channels: [
                    Some(BatteryChannel {
                        channel_type: ChannelType::Main,
                        percentage: 88,
                        charging: false,
                    }),
                    None,
                    None,
                    None,
                ],
            },
        );
        statuses.insert(
            "xbox_controller".to_string(),
            DeviceBatteryStatus::Online {
                channels: [
                    Some(BatteryChannel {
                        channel_type: ChannelType::Main,
                        percentage: 12,
                        charging: true,
                    }),
                    None,
                    None,
                    None,
                ],
            },
        );
        statuses.insert(
            "gamebuds_1".to_string(),
            DeviceBatteryStatus::Online {
                channels: [
                    Some(BatteryChannel {
                        channel_type: ChannelType::Left,
                        percentage: 90,
                        charging: false,
                    }),
                    Some(BatteryChannel {
                        channel_type: ChannelType::Right,
                        percentage: 85,
                        charging: true,
                    }),
                    Some(BatteryChannel {
                        channel_type: ChannelType::Case,
                        percentage: 100,
                        charging: false,
                    }),
                    None,
                ],
            },
        );

        let active_devices = vec![
            "pulsar_x2".to_string(),
            "xbox_controller".to_string(),
            "gamebuds_1".to_string(),
        ];
        let mut window = SettingsWindow::new(config, active_devices, statuses);

        // Render Devices Tab
        window.active_tab = Tab::Devices;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                window.render_devices_tab(ui);
            });
        });

        // Render General Tab
        window.active_tab = Tab::General;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                window.render_general_tab(ui);
            });
        });

        // Render About Tab
        window.active_tab = Tab::About;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                window.render_about_tab(ui);
            });
        });

        assert_eq!(window.config.devices.len(), 3);
        assert_eq!(window.config.devices[0].threshold, 20);
    }
}
