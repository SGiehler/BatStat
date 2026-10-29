pub mod devices_view;
pub mod preferences_view;
pub mod theme;
pub mod widgets;

use crate::config::{get_icon_list_for, resolve_icons_dir, AppConfig, DeviceConfig};
use crate::plugins::DeviceBatteryStatus;
use crate::ui::theme::{ActionIcon, DeviceKind};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlyoutView {
    Devices,
    Preferences,
}

/// Reorders `devices` by moving `from_idx` to `to_idx`. Returns `true` if order changed.
pub fn reorder_devices(devices: &mut Vec<DeviceConfig>, from_idx: usize, to_idx: usize) -> bool {
    if from_idx >= devices.len() || to_idx >= devices.len() || from_idx == to_idx {
        return false;
    }
    let item = devices.remove(from_idx);
    devices.insert(to_idx, item);
    true
}

/// Formats an icon filename into a clean human-readable label
pub fn format_icon_label(filename: &str) -> String {
    let name = filename
        .strip_suffix(".png")
        .or_else(|| filename.strip_suffix(".PNG"))
        .or_else(|| filename.strip_suffix(".ico"))
        .or_else(|| filename.strip_suffix(".ICO"))
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

/// Returns `(Type Tag, DeviceKind, Friendly Title, Base Type)` for a device `unique_id`
pub fn get_device_category(
    unique_id: &str,
) -> (&'static str, DeviceKind, &'static str, &'static str) {
    if unique_id.starts_with("pulsar_")
        || unique_id.starts_with("logitech_")
        || unique_id.contains("mouse")
    {
        ("MOUSE", DeviceKind::Mouse, "Gaming Mouse", "mouse")
    } else if unique_id.starts_with("xbox_")
        || unique_id.contains("gamepad")
        || unique_id.contains("controller")
    {
        ("GAMEPAD", DeviceKind::Gamepad, "Controller", "gamepad")
    } else if unique_id.starts_with("gamebuds")
        || unique_id.contains("buds")
        || unique_id.contains("headset")
    {
        ("BUDS", DeviceKind::Buds, "Wireless Earbuds", "buds")
    } else if unique_id.starts_with("keyboard") || unique_id.contains("keyboard") {
        ("KEYBOARD", DeviceKind::Keyboard, "Keyboard", "keyboard")
    } else {
        ("PERIPHERAL", DeviceKind::Peripheral, "Peripheral", "peripheral")
    }
}

pub fn get_available_channels(config: &AppConfig) -> Vec<(String, String)> {
    let mut options = Vec::new();
    for dev in &config.devices {
        if dev.unique_id.starts_with("gamebuds") {
            options.push((
                format!("{}:Left", dev.unique_id),
                format!("{} (Left)", dev.name),
            ));
            options.push((
                format!("{}:Right", dev.unique_id),
                format!("{} (Right)", dev.name),
            ));
        } else {
            options.push((format!("{}:Main", dev.unique_id), dev.name.clone()));
        }
    }
    options
}

pub struct SettingsWindow {
    pub config: AppConfig,
    pub active_devices: Vec<String>,
    pub device_statuses: std::collections::HashMap<String, DeviceBatteryStatus>,
    pub update_status: crate::UpdateStatus,

    // Flyout navigation & progressive disclosure state
    pub view: FlyoutView,
    pub expanded_device_id: Option<String>,
    pub dragging_device_id: Option<String>,
    pub drag_reordered: bool,

    // Window pin / drag / modal dialog guards
    pub is_pinned: bool,
    pub has_custom_position: bool,
    pub suppress_focus_hide: bool,

    // 100% Auto-save coordinator
    pub save_immediately: bool,
    pub save_on_release: bool,
    pub config_saved_in_place: bool,
    pub custom_config_path: Option<PathBuf>,

    // Host signals
    pub device_removed: bool,
    pub request_close: bool,
    pub request_poll: bool,
    pub request_test_notification: bool,
    pub request_update_check: bool,
    pub request_download_install: Option<String>,

    // Inline editing & diagnostics state
    pub editing_device_id: Option<String>,
    pub edit_name_buffer: String,
    pub focus_rename: bool,
    pub confirming_remove_id: Option<String>,
    pub logs_expanded: bool,
    pub debug_log_content: Option<String>,

    // Textures & visual polish
    pub icon_textures: std::collections::HashMap<String, egui::TextureHandle>,
    pub loaded_icons_dir: Option<PathBuf>,
    pub scanning_timer: Option<Instant>,
    pub toast: Option<(String, Instant)>,
    pub fonts_initialized: bool,
}

impl SettingsWindow {
    pub fn new(
        mut config: AppConfig,
        active_devices: Vec<String>,
        device_statuses: std::collections::HashMap<String, DeviceBatteryStatus>,
    ) -> Self {
        config.polling_interval_secs = config.polling_interval_secs.clamp(1, 60);
        Self {
            config,
            active_devices,
            device_statuses,
            update_status: crate::UpdateStatus::Idle,

            view: FlyoutView::Devices,
            expanded_device_id: None,
            dragging_device_id: None,
            drag_reordered: false,

            is_pinned: false,
            has_custom_position: false,
            suppress_focus_hide: false,

            save_immediately: false,
            save_on_release: false,
            config_saved_in_place: false,
            custom_config_path: None,

            device_removed: false,
            request_close: false,
            request_poll: false,
            request_test_notification: false,
            request_update_check: false,
            request_download_install: None,

            editing_device_id: None,
            edit_name_buffer: String::new(),
            focus_rename: false,
            confirming_remove_id: None,
            logs_expanded: false,
            debug_log_content: None,

            icon_textures: std::collections::HashMap::new(),
            loaded_icons_dir: None,
            scanning_timer: None,
            toast: None,
            fonts_initialized: false,
        }
    }

    pub fn mark_dirty_immediate(&mut self) {
        self.save_immediately = true;
    }

    pub fn mark_dirty_on_release(&mut self) {
        self.save_on_release = true;
    }

    fn persist_config(&self) -> Result<(), String> {
        if let Some(ref custom_path) = self.custom_config_path {
            if let Some(parent) = custom_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create config dir: {}", e))?;
            }
            let content = toml::to_string_pretty(&self.config)
                .map_err(|e| format!("Failed to serialize config: {}", e))?;
            std::fs::write(custom_path, content)
                .map_err(|e| format!("Failed to write config: {}", e))?;
            return Ok(());
        }

        #[cfg(not(test))]
        {
            crate::config::save_config(&self.config)
        }

        #[cfg(test)]
        {
            let _ = toml::to_string_pretty(&self.config)
                .map_err(|e| format!("Failed to serialize config: {}", e))?;
            Ok(())
        }
    }

    /// Immediately persists any pending discrete or slider/reorder edits
    pub fn flush_autosave(&mut self) {
        if !self.save_immediately && !self.save_on_release {
            return;
        }
        self.save_immediately = false;
        self.save_on_release = false;

        match self.persist_config() {
            Ok(()) => {
                self.config_saved_in_place = true;
            }
            Err(e) => {
                eprintln!("Auto-save failed: {}", e);
                self.show_toast("Failed to save settings");
            }
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
                    "No logs recorded yet.\nToggle debug logging on to capture live polling events."
                        .to_string(),
                );
            }
        }
    }

    /// Hierarchical Escape key handler:
    /// 1. Cancel inline rename -> 2. Cancel remove confirm -> 3. Exit Preferences -> 4. Collapse drawer -> 5. Close flyout
    pub fn handle_escape_press(&mut self) {
        if self.editing_device_id.is_some() {
            self.editing_device_id = None;
        } else if self.confirming_remove_id.is_some() {
            self.confirming_remove_id = None;
        } else if self.view == FlyoutView::Preferences {
            self.flush_autosave();
            self.view = FlyoutView::Devices;
        } else if self.expanded_device_id.is_some() {
            self.flush_autosave();
            self.expanded_device_id = None;
        } else {
            self.flush_autosave();
            self.request_close = true;
        }
    }

    pub fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.handle_escape_press();
        }
        if ctx.input(|i| {
            i.key_pressed(egui::Key::F5) || (i.modifiers.command && i.key_pressed(egui::Key::R))
        }) {
            self.request_poll = true;
            self.scanning_timer = Some(Instant::now());
            self.show_toast("Refreshing batteries…");
        }
    }

    pub fn ensure_textures(&mut self, ctx: &egui::Context) {
        let active_dir = resolve_icons_dir(&self.config);
        if self.loaded_icons_dir != active_dir {
            self.icon_textures.clear();
            self.loaded_icons_dir = active_dir.clone();
        }

        let icon_list = get_icon_list_for(&self.config);
        let needs_load = self.icon_textures.is_empty()
            || icon_list
                .iter()
                .any(|name| !self.icon_textures.contains_key(name));

        if !needs_load {
            return;
        }

        #[cfg(not(test))]
        if let Some(ref dir) = active_dir {
            crate::config::setup_icons_folder_at(dir);
        }

        // Load files from active icons directory first so customized files take priority
        if let Some(ref icons_dir) = active_dir {
            if let Ok(entries) = std::fs::read_dir(icons_dir) {
                for entry in entries.flatten() {
                    if let Ok(ft) = entry.file_type() {
                        if ft.is_file() {
                            let name = entry.file_name().to_string_lossy().into_owned();
                            let lower = name.to_ascii_lowercase();
                            if !self.icon_textures.contains_key(&name)
                                && (lower.ends_with(".png") || lower.ends_with(".ico"))
                            {
                                let path = icons_dir.join(&name);
                                if let Ok(img) = image::open(&path) {
                                    let size = [img.width() as usize, img.height() as usize];
                                    let rgba = img.to_rgba8();
                                    let color_img = egui::ColorImage::from_rgba_unmultiplied(
                                        size,
                                        rgba.as_flat_samples().as_slice(),
                                    );
                                    let tex = ctx.load_texture(
                                        &name,
                                        color_img,
                                        egui::TextureOptions::default(),
                                    );
                                    self.icon_textures.insert(name, tex);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Built-in fallbacks
        let builtins: [(&str, &[u8]); 5] = [
            ("ok.png", include_bytes!("../icons/ok.png")),
            ("low_mouse.png", include_bytes!("../icons/low_mouse.png")),
            ("low_gamepad.png", include_bytes!("../icons/low_gamepad.png")),
            ("low_buds.png", include_bytes!("../icons/low_buds.png")),
            (
                "low_keyboard.png",
                include_bytes!("../icons/low_keyboard.png"),
            ),
        ];

        for (name, bytes) in builtins {
            if !self.icon_textures.contains_key(name) {
                if let Ok(img) = image::load_from_memory(bytes) {
                    let size = [img.width() as usize, img.height() as usize];
                    let rgba = img.to_rgba8();
                    let color_img = egui::ColorImage::from_rgba_unmultiplied(
                        size,
                        rgba.as_flat_samples().as_slice(),
                    );
                    let tex = ctx.load_texture(name, color_img, egui::TextureOptions::default());
                    self.icon_textures.insert(name.to_string(), tex);
                }
            }
        }
    }

    pub fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render_flyout(ctx);
    }

    pub fn render_flyout(&mut self, ctx: &egui::Context) {
        if !self.fonts_initialized {
            theme::configure_flyout_fonts(ctx);
            theme::apply_flyout_visuals(ctx);
            self.fonts_initialized = true;
        }

        self.ensure_textures(ctx);
        self.handle_shortcuts(ctx);

        // Compute spin animation angle while manual refresh is active
        let mut spin_angle = 0.0_f32;
        if let Some(start) = self.scanning_timer {
            let elapsed = start.elapsed().as_secs_f32();
            if elapsed < 1.2 {
                spin_angle = elapsed * std::f32::consts::TAU * 1.6;
                ctx.request_repaint();
            } else {
                self.scanning_timer = None;
            }
        }

        let online_count = self
            .config
            .devices
            .iter()
            .filter(|d| {
                d.enabled
                    && self.active_devices.contains(&d.unique_id)
                    && self
                        .device_statuses
                        .get(&d.unique_id)
                        .map(|s| s.is_online())
                        .unwrap_or(false)
            })
            .count();

        let low_count = self
            .config
            .devices
            .iter()
            .filter(|d| {
                d.enabled
                    && self
                        .device_statuses
                        .get(&d.unique_id)
                        .map(|s| s.is_online() && s.effective_percentage() <= d.threshold)
                        .unwrap_or(false)
            })
            .count();

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(theme::BG_FLYOUT)
                    .stroke(Stroke::new(1.0, theme::BORDER_FLYOUT))
                    .inner_margin(egui::Margin::same(0.0)),
            )
            .show(ctx, |ui| {
                // =========================================================
                // 48px Draggable Flyout Header
                // =========================================================
                let full_w = ui.available_width();
                let (header_rect, _) =
                    ui.allocate_exact_size(Vec2::new(full_w, 48.0), Sense::hover());

                ui.painter().rect_filled(header_rect, 0.0, theme::BG_HEADER);
                ui.painter().line_segment(
                    [
                        Pos2::new(header_rect.left(), header_rect.bottom()),
                        Pos2::new(header_rect.right(), header_rect.bottom()),
                    ],
                    Stroke::new(1.0, theme::BORDER_FLYOUT),
                );

                // Left drag region (leaves 138px on right for the 4 action buttons)
                let drag_right = (header_rect.right() - 138.0).max(header_rect.left() + 40.0);
                let drag_rect =
                    Rect::from_min_max(header_rect.min, Pos2::new(drag_right, header_rect.bottom()));
                let drag_resp = ui.interact(
                    drag_rect,
                    ui.id().with("flyout_header_drag"),
                    Sense::click_and_drag(),
                );
                if drag_resp.drag_started() {
                    self.is_pinned = true;
                    self.has_custom_position = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if drag_resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }

                // Brand bolt badge + Title + Live status capsule
                let badge_rect = Rect::from_center_size(
                    Pos2::new(header_rect.left() + 24.0, header_rect.center().y),
                    Vec2::new(24.0, 24.0),
                );
                ui.painter()
                    .rect_filled(badge_rect, 6.0, theme::ACCENT_ONLINE_BG);
                ui.painter().rect_stroke(
                    badge_rect,
                    6.0,
                    Stroke::new(1.0, theme::ACCENT_ONLINE.linear_multiply(0.35)),
                );
                theme::draw_bolt(ui.painter(), badge_rect.center(), 1.0, theme::ACCENT_ONLINE);

                let header_title = match self.view {
                    FlyoutView::Devices => "BatStat",
                    FlyoutView::Preferences => "Preferences",
                };
                let title_galley = ui.painter().layout_no_wrap(
                    header_title.to_string(),
                    egui::FontId::proportional(14.5),
                    theme::TEXT_PRIMARY,
                );
                let title_pos = Pos2::new(
                    badge_rect.right() + 9.0,
                    header_rect.center().y - title_galley.size().y * 0.5,
                );
                let title_w = title_galley.size().x;
                ui.painter()
                    .galley(title_pos, title_galley, theme::TEXT_PRIMARY);

                if self.view == FlyoutView::Devices {
                    let (pill_text, pill_fg, pill_bg) = if low_count > 0 {
                        (
                            format!("{} low", low_count),
                            theme::ACCENT_LOW,
                            theme::ACCENT_LOW_BG,
                        )
                    } else {
                        (
                            format!("{} online", online_count),
                            theme::ACCENT_ONLINE,
                            theme::ACCENT_ONLINE_BG,
                        )
                    };
                    let pill_galley = ui.painter().layout_no_wrap(
                        pill_text,
                        egui::FontId::proportional(10.0),
                        pill_fg,
                    );
                    let pw = pill_galley.size().x + 14.0;
                    let pill_rect = Rect::from_center_size(
                        Pos2::new(
                            title_pos.x + title_w + 8.0 + pw * 0.5,
                            header_rect.center().y,
                        ),
                        Vec2::new(pw, 18.0),
                    );
                    if pill_rect.right() < drag_right - 4.0 {
                        ui.painter().rect_filled(pill_rect, 9.0, pill_bg);
                        ui.painter().galley(
                            Pos2::new(
                                pill_rect.left() + 7.0,
                                pill_rect.center().y - pill_galley.size().y * 0.5,
                            ),
                            pill_galley,
                            pill_fg,
                        );
                    }
                }

                // Right-side 4 action buttons (`Refresh`, `Pin`, `Gear`/`Back`, `Close`)
                let actions_rect = Rect::from_min_max(
                    Pos2::new(drag_right, header_rect.top()),
                    Pos2::new(header_rect.right() - 10.0, header_rect.bottom()),
                );
                let mut actions_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(actions_rect)
                        .layout(egui::Layout::right_to_left(egui::Align::Center)),
                );
                actions_ui.spacing_mut().item_spacing.x = 3.0;

                if widgets::header_icon_button(
                    &mut actions_ui,
                    ActionIcon::Close,
                    false,
                    false,
                    0.0,
                    "Close flyout (Esc)",
                )
                .clicked()
                {
                    self.flush_autosave();
                    self.request_close = true;
                }

                let has_update = matches!(self.update_status, crate::UpdateStatus::Available(_));
                match self.view {
                    FlyoutView::Devices => {
                        if widgets::header_icon_button(
                            &mut actions_ui,
                            ActionIcon::Gear,
                            false,
                            has_update,
                            0.0,
                            if has_update {
                                "Preferences (Update available)"
                            } else {
                                "Preferences"
                            },
                        )
                        .clicked()
                        {
                            self.flush_autosave();
                            self.view = FlyoutView::Preferences;
                        }
                    }
                    FlyoutView::Preferences => {
                        if widgets::header_icon_button(
                            &mut actions_ui,
                            ActionIcon::Back,
                            true,
                            false,
                            0.0,
                            "Back to devices (Esc)",
                        )
                        .clicked()
                        {
                            self.flush_autosave();
                            self.view = FlyoutView::Devices;
                        }
                    }
                }

                let pin_tip = if self.is_pinned {
                    "Pinned open (click to unpin)"
                } else {
                    "Pin flyout open"
                };
                if widgets::header_icon_button(
                    &mut actions_ui,
                    ActionIcon::Pin {
                        pinned: self.is_pinned,
                    },
                    self.is_pinned,
                    false,
                    0.0,
                    pin_tip,
                )
                .clicked()
                {
                    self.is_pinned = !self.is_pinned;
                    if !self.is_pinned {
                        self.has_custom_position = false;
                        self.show_toast("Unpinned");
                    } else {
                        self.show_toast("Pinned open");
                    }
                }

                if widgets::header_icon_button(
                    &mut actions_ui,
                    ActionIcon::Refresh,
                    self.scanning_timer.is_some(),
                    false,
                    spin_angle,
                    "Refresh batteries (F5)",
                )
                .clicked()
                {
                    self.request_poll = true;
                    self.scanning_timer = Some(Instant::now());
                    self.show_toast("Refreshing batteries…");
                }

                // =========================================================
                // Scrollable Body Area
                // =========================================================
                egui::Frame::none()
                    .inner_margin(egui::Margin::symmetric(12.0, 10.0))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt(("flyout_body_scroll", self.view == FlyoutView::Preferences))
                            .auto_shrink([false, false])
                            .show(ui, |ui| match self.view {
                                FlyoutView::Devices => {
                                    devices_view::render_devices_view(self, ui);
                                }
                                FlyoutView::Preferences => {
                                    preferences_view::render_preferences_view(self, ui);
                                }
                            });
                    });
            });

        // Auto-save coordinator at end of frame
        if self.save_immediately {
            self.flush_autosave();
        } else if self.save_on_release && !ctx.input(|i| i.pointer.any_down()) {
            self.flush_autosave();
            self.show_toast("Saved");
        }

        // Subtle floating 2.5s bottom toast pill
        if let Some((ref msg, created_at)) = self.toast.clone() {
            let elapsed = created_at.elapsed();
            if elapsed < Duration::from_millis(2500) {
                ctx.request_repaint_after(Duration::from_millis(80));
                let fade = if elapsed > Duration::from_millis(2000) {
                    1.0 - ((elapsed.as_secs_f32() - 2.0) / 0.5).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                let screen = ctx.screen_rect();
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Foreground,
                    egui::Id::new("flyout_toast_layer"),
                ));
                let text_col = theme::TEXT_PRIMARY.linear_multiply(fade);
                let galley =
                    painter.layout_no_wrap(msg.clone(), egui::FontId::proportional(11.0), text_col);
                let pill_size = Vec2::new(galley.size().x + 22.0, 26.0);
                let pill_rect = Rect::from_center_size(
                    Pos2::new(screen.center().x, screen.bottom() - 22.0),
                    pill_size,
                );
                painter.rect_filled(
                    pill_rect,
                    13.0,
                    Color32::from_rgb(32, 35, 45).linear_multiply(fade),
                );
                painter.rect_stroke(
                    pill_rect,
                    13.0,
                    Stroke::new(1.0, theme::ACCENT_ONLINE.linear_multiply(0.45 * fade)),
                );
                painter.galley(
                    Pos2::new(
                        pill_rect.center().x - galley.size().x * 0.5,
                        pill_rect.center().y - galley.size().y * 0.5,
                    ),
                    galley,
                    text_col,
                );
            } else {
                self.toast = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::{BatteryChannel, ChannelType};
    use crate::ui::theme::DeviceKind;

    #[test]
    fn test_device_category_mapping() {
        assert_eq!(
            get_device_category("pulsar_0x5406"),
            ("MOUSE", DeviceKind::Mouse, "Gaming Mouse", "mouse")
        );
        assert_eq!(
            get_device_category("xbox_0"),
            ("GAMEPAD", DeviceKind::Gamepad, "Controller", "gamepad")
        );
        assert_eq!(
            get_device_category("gamebuds_left"),
            ("BUDS", DeviceKind::Buds, "Wireless Earbuds", "buds")
        );
        assert_eq!(
            get_device_category("keyboard_1"),
            ("KEYBOARD", DeviceKind::Keyboard, "Keyboard", "keyboard")
        );
        assert_eq!(
            get_device_category("unknown_dongle"),
            ("PERIPHERAL", DeviceKind::Peripheral, "Peripheral", "peripheral")
        );
    }

    #[test]
    fn test_format_icon_label() {
        assert_eq!(format_icon_label("low_mouse_red.png"), "Mouse (Red)");
        assert_eq!(format_icon_label("low_mouse_orange.png"), "Mouse (Orange)");
        assert_eq!(format_icon_label("low_mouse_yellow.png"), "Mouse (Yellow)");
        assert_eq!(format_icon_label("low_mouse_blue.png"), "Mouse (Blue)");
        assert_eq!(format_icon_label("low_mouse.png"), "Mouse (White)");
        assert_eq!(format_icon_label("low_gamepad_red.png"), "Gamepad (Red)");
        assert_eq!(format_icon_label("low_buds_yellow.png"), "Earbuds (Yellow)");
        assert_eq!(format_icon_label("low_keyboard.png"), "Keyboard (White)");
        assert_eq!(format_icon_label("custom_icon.png"), "custom_icon.png");
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
    fn test_reorder_devices_preserves_elements_and_updates_priority() {
        let mut devs = vec![
            DeviceConfig {
                unique_id: "a".into(),
                name: "A".into(),
                enabled: true,
                threshold: 20,
                low_battery_icon_path: None,
            },
            DeviceConfig {
                unique_id: "b".into(),
                name: "B".into(),
                enabled: true,
                threshold: 20,
                low_battery_icon_path: None,
            },
            DeviceConfig {
                unique_id: "c".into(),
                name: "C".into(),
                enabled: true,
                threshold: 20,
                low_battery_icon_path: None,
            },
        ];
        assert!(reorder_devices(&mut devs, 2, 0));
        let ids: Vec<&str> = devs.iter().map(|d| d.unique_id.as_str()).collect();
        assert_eq!(ids, vec!["c", "a", "b"]);
        assert!(!reorder_devices(&mut devs, 1, 1));
        assert!(!reorder_devices(&mut devs, 5, 0));
    }

    #[test]
    fn test_autosave_flushes_pending_slider_change_on_close() {
        let temp_cfg_path = std::env::temp_dir()
            .join(format!("batstat_autosave_test_{}.toml", std::process::id()));
        let _ = std::fs::remove_file(&temp_cfg_path);

        let mut cfg = AppConfig::default();
        cfg.devices.push(DeviceConfig {
            unique_id: "pulsar_1".into(),
            name: "Pulsar X2".into(),
            enabled: true,
            threshold: 20,
            low_battery_icon_path: None,
        });
        let mut win = SettingsWindow::new(cfg, vec![], std::collections::HashMap::new());
        win.custom_config_path = Some(temp_cfg_path.clone());
        win.config.devices[0].threshold = 35;
        win.save_on_release = true;
        win.flush_autosave();
        assert!(!win.save_on_release);
        assert!(win.config_saved_in_place);

        let saved_toml = std::fs::read_to_string(&temp_cfg_path).unwrap();
        let parsed: AppConfig = toml::from_str(&saved_toml).unwrap();
        assert_eq!(parsed.devices[0].threshold, 35);
        let _ = std::fs::remove_file(&temp_cfg_path);
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
            unique_id: "pulsar_1".into(),
            name: "Pulsar X2".into(),
            enabled: true,
            threshold: 20,
            low_battery_icon_path: Some("low_mouse_red.png".into()),
        });
        cfg.devices.push(DeviceConfig {
            unique_id: "keyboard_1".into(),
            name: "Keychron K2".into(),
            enabled: true,
            threshold: 20,
            low_battery_icon_path: None,
        });
        let mut statuses = std::collections::HashMap::new();
        statuses.insert(
            "gamebuds".into(),
            DeviceBatteryStatus::Online {
                channels: [
                    Some(BatteryChannel {
                        channel_type: ChannelType::Left,
                        percentage: 92,
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
        statuses.insert(
            "pulsar_1".into(),
            DeviceBatteryStatus::Online {
                channels: [
                    Some(BatteryChannel {
                        channel_type: ChannelType::Main,
                        percentage: 14,
                        charging: false,
                    }),
                    None,
                    None,
                    None,
                ],
            },
        );
        let mut win =
            SettingsWindow::new(cfg, vec!["gamebuds".into(), "pulsar_1".into()], statuses);
        win.expanded_device_id = Some("gamebuds".into());

        let _ = ctx.run(egui::RawInput::default(), |ctx| win.render_flyout(ctx));
        win.view = FlyoutView::Preferences;
        win.logs_expanded = true;
        let _ = ctx.run(egui::RawInput::default(), |ctx| win.render_flyout(ctx));
    }
}
