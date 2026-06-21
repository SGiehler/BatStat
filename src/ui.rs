use crate::config::{AppConfig, save_config};
use crate::autostart::set_autostart;
use crate::plugins::{DeviceBatteryStatus, BatteryChannel, ChannelType};
use eframe::egui;

pub struct SettingsWindow {
    pub config: AppConfig,
    pub active_devices: Vec<String>, // list of active unique_ids
    pub device_statuses: std::collections::HashMap<String, DeviceBatteryStatus>,
    pub request_close: bool,
    pub request_poll: bool,
    pub device_removed: bool,
    pub icon_textures: std::collections::HashMap<String, egui::TextureHandle>,
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
            request_close: false,
            request_poll: false,
            device_removed: false,
            icon_textures: std::collections::HashMap::new(),
        }
    }
}

// Custom animated toggle switch widget to match the premium mockup feel
fn toggle_ui(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
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
            egui::Color32::from_rgb(0x4c, 0xc9, 0xf0) // Electric Blue
        } else {
            egui::Color32::from_rgb(0x39, 0x39, 0x52) // outline-variant dark gray
        };
        
        let rounding = rect.height() / 2.0;
        ui.painter().rect_filled(rect, rounding, track_color);
        
        // Knob
        let knob_radius = rect.height() / 2.0 - 2.0;
        let min_x = rect.left() + knob_radius + 2.0;
        let max_x = rect.right() - knob_radius - 2.0;
        let knob_x = min_x + how_on * (max_x - min_x);
        let knob_center = egui::pos2(knob_x, rect.center().y);
        let knob_color = egui::Color32::WHITE;
        ui.painter().circle_filled(knob_center, knob_radius, knob_color);
    }

    response
}

impl eframe::App for SettingsWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.icon_textures.is_empty() {
            if let Some(icons_dir) = crate::config::get_icons_dir_path() {
                if let Ok(entries) = std::fs::read_dir(&icons_dir) {
                    for entry in entries.flatten() {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_file() {
                                let name = entry.file_name().to_string_lossy().into_owned();
                                if name.ends_with(".png") || name.ends_with(".ico") {
                                    let path = icons_dir.join(&name);
                                    if let Ok(img) = image::open(&path) {
                                        let size = [img.width() as _, img.height() as _];
                                        let img_rgba = img.to_rgba8();
                                        let pixels = img_rgba.as_flat_samples();
                                        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                                        let texture = ctx.load_texture(
                                            &name,
                                            color_image,
                                            egui::TextureOptions::default()
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

        // Set the custom dark-theme styling inspired by the MCP Stitch mockup
        let mut visuals = egui::Visuals::dark();
        
        visuals.panel_fill = egui::Color32::from_rgb(0x1a, 0x1a, 0x2e); // Deep space background
        visuals.window_fill = egui::Color32::from_rgb(0x1a, 0x1a, 0x2e);
        
        // Setup card / container look (non-interactive widgets)
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(0x25, 0x25, 0x40); // Card color
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0x39, 0x39, 0x52)); // Outline border
        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0x8d, 0x8d, 0x8d)); // Secondary text
        visuals.widgets.noninteractive.rounding = egui::Rounding::same(4.0);
        
        // Setup interactive elements (inactive buttons, sliders, etc.)
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(0x25, 0x25, 0x40);
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0x39, 0x39, 0x52));
        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
        
        // Hover state
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(0x2c, 0x2c, 0x4d); // card highlight
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0x4c, 0xc9, 0xf0)); // Electric blue border
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
        
        // Active/Pressed state
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0x4c, 0xc9, 0xf0);
        visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0x4c, 0xc9, 0xf0));
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0x16, 0x16, 0x16));
        visuals.widgets.active.rounding = egui::Rounding::same(4.0);
        
        // Slider background track - must be lighter than panel_fill to show the horizontal line!
        visuals.extreme_bg_color = egui::Color32::from_rgb(0x4e, 0x4e, 0x70);
        visuals.selection.bg_fill = egui::Color32::from_rgb(0x4c, 0xc9, 0xf0);
        
        ctx.set_visuals(visuals);

        // 1. TOP PANEL: Header
        egui::TopBottomPanel::top("header_panel")
            .frame(egui::Frame::none()
                .fill(egui::Color32::from_rgb(0x1a, 0x1a, 0x2e))
                .inner_margin(egui::Margin { left: 16.0, right: 16.0, top: 12.0, bottom: 12.0 })
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(0x39, 0x39, 0x52))) // bottom border
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("⚡").color(egui::Color32::from_rgb(0x4c, 0xc9, 0xf0)).font(egui::FontId::proportional(16.0)));
                    ui.label(egui::RichText::new("BatStat").strong().color(egui::Color32::WHITE).font(egui::FontId::proportional(15.0)));
                    
                    ui.add_space(4.0);
                    
                    // Version badge
                    let (badge_rect, _) = ui.allocate_exact_size(egui::vec2(44.0, 18.0), egui::Sense::hover());
                    ui.painter().rect_filled(badge_rect, 4.0, egui::Color32::from_rgb(0x25, 0x25, 0x40));
                    ui.painter().rect_stroke(badge_rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(0x39, 0x39, 0x52)));
                    ui.painter().text(
                        badge_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "v1.2.0",
                        egui::FontId::proportional(10.0),
                        egui::Color32::from_rgb(0x8d, 0x8d, 0x8d)
                    );
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new("⚙").color(egui::Color32::from_rgb(0x8d, 0x8d, 0x8d)).font(egui::FontId::proportional(16.0)));
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("❓").color(egui::Color32::from_rgb(0x8d, 0x8d, 0x8d)).font(egui::FontId::proportional(16.0)));
                    });
                });
            });

        // 2. BOTTOM PANEL: Footer Buttons (Centered perfectly via Right-to-Left vertical alignment)
        egui::TopBottomPanel::bottom("footer_panel")
            .frame(egui::Frame::none()
                .fill(egui::Color32::from_rgb(0x1a, 0x1a, 0x2e))
                .inner_margin(egui::Margin { left: 16.0, right: 16.0, top: 12.0, bottom: 12.0 })
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(0x39, 0x39, 0x52))) // top border
            )
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Save & Close button (electric blue)
                    let save_btn = egui::Button::new(
                        egui::RichText::new("Save & Close")
                            .color(egui::Color32::from_rgb(0x16, 0x16, 0x16)) // Dark contrast text
                            .strong()
                            .font(egui::FontId::proportional(12.0))
                    ).fill(egui::Color32::from_rgb(0x4c, 0xc9, 0xf0))
                     .rounding(4.0)
                     .min_size(egui::vec2(100.0, 28.0));
                    
                    if ui.add(save_btn).clicked() {
                        if let Err(e) = save_config(&self.config) {
                            eprintln!("Failed to save config: {}", e);
                        }
                        self.request_close = true;
                    }
                    
                    ui.add_space(8.0);
                    
                    // Cancel button (ghost style, perfectly aligned vertically)
                    let cancel_btn = egui::Button::new(
                        egui::RichText::new("Cancel")
                            .color(egui::Color32::from_rgb(0x8d, 0x8d, 0x8d))
                            .font(egui::FontId::proportional(12.0))
                    ).fill(egui::Color32::TRANSPARENT).frame(false);
                    
                    if ui.add(cancel_btn).clicked() {
                        self.request_close = true;
                    }
                });
            });

        // 3. CENTRAL PANEL: Scrollable content
        egui::CentralPanel::default()
            .frame(egui::Frame::none()
                .fill(egui::Color32::from_rgb(0x1a, 0x1a, 0x2e))
                .inner_margin(egui::Margin { left: 16.0, right: 16.0, top: 12.0, bottom: 12.0 })
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .show(ui, |ui| {
                        // Section A: Global Configuration
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new("⚙  GLOBAL CONFIGURATION")
                                    .color(egui::Color32::from_rgb(0x4c, 0xc9, 0xf0))
                                    .font(egui::FontId::proportional(10.0))
                                    .strong()
                            );
                            ui.add_space(6.0);
                            
                            // Card
                            egui::Frame::none()
                                .fill(egui::Color32::from_rgb(0x25, 0x25, 0x40))
                                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(0x39, 0x39, 0x52)))
                                .rounding(4.0)
                                .inner_margin(egui::Margin { left: 12.0, right: 12.0, top: 12.0, bottom: 12.0 })
                                .show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        // Polling Interval title + current badge
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("Polling Interval").color(egui::Color32::WHITE).font(egui::FontId::proportional(13.0)));
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                let text = format!("{}s", self.config.polling_interval_secs);
                                                let (badge_rect, _) = ui.allocate_exact_size(egui::vec2(40.0, 18.0), egui::Sense::hover());
                                                ui.painter().rect_filled(badge_rect, 4.0, egui::Color32::from_rgb(0x1a, 0x1a, 0x2e));
                                                ui.painter().text(
                                                    badge_rect.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    &text,
                                                    egui::FontId::monospace(10.0),
                                                    egui::Color32::from_rgb(0x4c, 0xc9, 0xf0)
                                                );
                                            });
                                        });
                                        ui.add_space(4.0);
                                        
                                        // Slider
                                        ui.scope(|ui| {
                                            ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_rgb(0x39, 0x39, 0x52);
                                            ui.visuals_mut().widgets.inactive.fg_stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(0x4c, 0xc9, 0xf0));
                                            ui.spacing_mut().slider_width = ui.available_width() - 8.0;
                                            ui.add(egui::Slider::new(&mut self.config.polling_interval_secs, 1..=60).show_value(false).trailing_fill(true));
                                        });
                                        
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("1s").color(egui::Color32::from_rgb(0x8d, 0x8d, 0x8d)).font(egui::FontId::proportional(9.0)));
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.label(egui::RichText::new("60s").color(egui::Color32::from_rgb(0x8d, 0x8d, 0x8d)).font(egui::FontId::proportional(9.0)));
                                            });
                                        });
                                        
                                        ui.add_space(10.0);
                                        ui.separator();
                                        ui.add_space(10.0);
                                        
                                        // Startup launch
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.label(egui::RichText::new("Launch on Startup").color(egui::Color32::WHITE).font(egui::FontId::proportional(13.0)));
                                                ui.label(egui::RichText::new("Start BatStat automatically with Windows").color(egui::Color32::from_rgb(0x8d, 0x8d, 0x8d)).font(egui::FontId::proportional(9.0)).italics());
                                            });
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                let mut autostart = self.config.autostart;
                                                if toggle_ui(ui, &mut autostart).changed() {
                                                    self.config.autostart = autostart;
                                                    if let Err(e) = set_autostart(autostart) {
                                                        eprintln!("Failed to set autostart: {}", e);
                                                    }
                                                }
                                            });
                                        });
                                        
                                        ui.add_space(10.0);
                                        ui.separator();
                                        ui.add_space(10.0);
                                        
                                        // Windows Notification Alerts
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.label(egui::RichText::new("Windows Notification Alerts").color(egui::Color32::WHITE).font(egui::FontId::proportional(13.0)));
                                                ui.label(egui::RichText::new("Show desktop alerts when battery levels are low.").color(egui::Color32::from_rgb(0x8d, 0x8d, 0x8d)).font(egui::FontId::proportional(9.0)).italics());
                                            });
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                toggle_ui(ui, &mut self.config.enable_notifications);
                                            });
                                        });

                                        ui.add_space(10.0);
                                        ui.separator();
                                        ui.add_space(10.0);

                                        // Custom Icons Folder Shortcut
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.label(egui::RichText::new("Custom Icons Folder").color(egui::Color32::WHITE).font(egui::FontId::proportional(13.0)));
                                                ui.label(egui::RichText::new("Open directory where custom low battery icons are stored.").color(egui::Color32::from_rgb(0x8d, 0x8d, 0x8d)).font(egui::FontId::proportional(9.0)).italics());
                                            });
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                let btn = egui::Button::new(
                                                    egui::RichText::new("📁 Open Folder")
                                                        .color(egui::Color32::WHITE)
                                                        .font(egui::FontId::proportional(11.0))
                                                        .strong()
                                                ).fill(egui::Color32::from_rgb(0x2c, 0x2c, 0x4d)).rounding(4.0);

                                                if ui.add(btn).clicked() {
                                                    if let Some(dir) = crate::config::get_icons_dir_path() {
                                                        let _ = std::process::Command::new("explorer").arg(dir).spawn();
                                                    }
                                                }
                                            });
                                        });
                                    });
                                });
                        });
                        
                        ui.add_space(16.0);
                        
                        // Section B: Peripherals (With dynamic detect devices button)
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("📱  CONNECTED PERIPHERALS")
                                        .color(egui::Color32::from_rgb(0x4c, 0xc9, 0xf0))
                                        .font(egui::FontId::proportional(10.0))
                                        .strong()
                                );
                                
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let btn = egui::Button::new(
                                        egui::RichText::new("🔄 Detect Devices")
                                            .color(egui::Color32::WHITE)
                                            .font(egui::FontId::proportional(10.0))
                                            .strong()
                                    ).fill(egui::Color32::from_rgb(0x2c, 0x2c, 0x4d)).rounding(4.0);
                                    
                                    if ui.add(btn).clicked() {
                                        self.request_poll = true;
                                    }
                                });
                            });
                            ui.add_space(6.0);
                            
                            let mut to_remove = None;
                            
                            for (idx, dev) in self.config.devices.iter_mut().enumerate() {
                                let is_active = self.active_devices.contains(&dev.unique_id);
                                
                                // Render card
                                egui::Frame::none()
                                    .fill(egui::Color32::from_rgb(0x25, 0x25, 0x40))
                                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(0x39, 0x39, 0x52)))
                                    .rounding(4.0)
                                    .inner_margin(egui::Margin { left: 12.0, right: 12.0, top: 12.0, bottom: 12.0 })
                                    .show(ui, |ui| {
                                        ui.vertical(|ui| {
                                            // Row 1: Type badge + Name/Status + Enabled checkbox
                                            ui.horizontal(|ui| {
                                                // Type badge (standard letters, 100% robust rendering)
                                                let tag = if dev.unique_id.starts_with("pulsar_") {
                                                    "MOUSE"
                                                } else if dev.unique_id.starts_with("xbox_") {
                                                    "GAMEPAD"
                                                } else if dev.unique_id.starts_with("gamebuds") {
                                                    "BUDS"
                                                } else if dev.unique_id.starts_with("keyboard") || dev.unique_id.contains("keyboard") {
                                                    "KEYBOARD"
                                                } else {
                                                    "DEV"
                                                };
                                                
                                                let (tag_rect, _) = ui.allocate_exact_size(egui::vec2(52.0, 20.0), egui::Sense::hover());
                                                ui.painter().rect_filled(tag_rect, 4.0, egui::Color32::from_rgb(0x1a, 0x1a, 0x2e));
                                                ui.painter().rect_stroke(tag_rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(0x39, 0x39, 0x52)));
                                                ui.painter().text(
                                                    tag_rect.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    tag,
                                                    egui::FontId::proportional(9.0),
                                                    egui::Color32::from_rgb(0x4c, 0xc9, 0xf0) // Electric Blue text
                                                );
                                                
                                                ui.add_space(6.0);
                                                
                                                // Title and status dot with battery levels
                                                ui.vertical(|ui| {
                                                    ui.label(egui::RichText::new(&dev.name).color(egui::Color32::WHITE).font(egui::FontId::proportional(13.0)).strong());
                                                    
                                                    ui.horizontal(|ui| {
                                                        if let Some(status) = self.device_statuses.get(&dev.unique_id) {
                                                            match status {
                                                                DeviceBatteryStatus::Online { channels } => {
                                                                    let active_channels: Vec<&BatteryChannel> = channels.iter().flatten().collect();
                                                                    if active_channels.is_empty() {
                                                                        let dot_color = egui::Color32::from_rgb(0x8d, 0x8d, 0x8d);
                                                                        let (rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                                                                        ui.painter().circle_filled(rect.center(), 3.0, dot_color);
                                                                        ui.label(egui::RichText::new("DISCONNECTED").color(dot_color).font(egui::FontId::proportional(10.0)).strong());
                                                                    } else {
                                                                        let dot_color = egui::Color32::from_rgb(0x00, 0xe6, 0x76);
                                                                        let (rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                                                                        ui.painter().circle_filled(rect.center(), 3.0, dot_color);
                                                                        
                                                                        for (c_idx, chan) in active_channels.iter().enumerate() {
                                                                            if c_idx > 0 {
                                                                                ui.label(egui::RichText::new("|").color(egui::Color32::from_rgb(0x39, 0x39, 0x52)));
                                                                            }
                                                                            let prefix = match chan.channel_type {
                                                                                ChannelType::Main => "",
                                                                                ChannelType::Left => "L: ",
                                                                                ChannelType::Right => "R: ",
                                                                                ChannelType::Case => "Case: ",
                                                                            };
                                                                            let chan_str = format!("{}{}%{}", prefix, chan.percentage, if chan.charging { " ⚡" } else { "" });
                                                                            ui.label(egui::RichText::new(chan_str).color(dot_color).font(egui::FontId::proportional(10.0)).strong());
                                                                        }
                                                                    }
                                                                }
                                                                DeviceBatteryStatus::Offline => {
                                                                    let dot_color = egui::Color32::from_rgb(0x8d, 0x8d, 0x8d);
                                                                    let (rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                                                                    ui.painter().circle_filled(rect.center(), 3.0, dot_color);
                                                                    ui.label(egui::RichText::new("DISCONNECTED").color(dot_color).font(egui::FontId::proportional(10.0)).strong());
                                                                }
                                                            }
                                                        } else {
                                                            let dot_color = egui::Color32::from_rgb(0x8d, 0x8d, 0x8d);
                                                            let (rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                                                            ui.painter().circle_filled(rect.center(), 3.0, dot_color);
                                                            ui.label(egui::RichText::new("DISCONNECTED").color(dot_color).font(egui::FontId::proportional(10.0)).strong());
                                                        }
                                                    });
                                                });
                                                
                                                // Enabled toggle
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    ui.checkbox(&mut dev.enabled, "");
                                                    ui.label(egui::RichText::new("Enabled").color(egui::Color32::from_rgb(0x8d, 0x8d, 0x8d)).font(egui::FontId::proportional(10.0)));
                                                });
                                            });
                                            
                                            ui.add_space(6.0);
                                            ui.separator();
                                            ui.add_space(6.0);
                                            
                                            // Row 2: Custom Low Icon
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new("LOW BATTERY ICON").color(egui::Color32::from_rgb(0x8d, 0x8d, 0x8d)).font(egui::FontId::proportional(9.0)).strong());
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    // Reset button
                                                    if dev.low_battery_icon_path.is_some() {
                                                        if ui.button(egui::RichText::new("Reset").font(egui::FontId::proportional(10.0))).clicked() {
                                                            dev.low_battery_icon_path = None;
                                                        }
                                                    }
                                                    
                                                    // Dropdown combo box
                                                    let available_icons = crate::config::get_icon_list();
                                                    let current_selected = dev.low_battery_icon_path.as_deref().unwrap_or("Default");
                                                    
                                                    let default_icon_name = if dev.unique_id.starts_with("pulsar_") {
                                                        "low_mouse.png"
                                                    } else if dev.unique_id.starts_with("xbox_") {
                                                        "low_gamepad.png"
                                                    } else if dev.unique_id.starts_with("gamebuds") {
                                                        "low_buds.png"
                                                    } else if dev.unique_id.starts_with("keyboard") || dev.unique_id.contains("keyboard") {
                                                        "low_keyboard.png"
                                                    } else {
                                                        "ok.png"
                                                    };
                                                    let preview_icon_name = dev.low_battery_icon_path.clone().unwrap_or_else(|| default_icon_name.to_string());
                                                    
                                                    egui::ComboBox::new(format!("icon_combo_{}", dev.unique_id), "")
                                                        .selected_text(current_selected)
                                                        .width(160.0)
                                                        .show_ui(ui, |ui| {
                                                            // Default selection
                                                            {
                                                                let is_default = dev.low_battery_icon_path.is_none();
                                                                let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 20.0), egui::Sense::click());
                                                                if response.clicked() {
                                                                    dev.low_battery_icon_path = None;
                                                                }
                                                                let visual = ui.style().interact(&response);
                                                                if is_default || response.hovered() {
                                                                    ui.painter().rect_filled(rect, 2.0, visual.bg_fill);
                                                                }
                                                                let image_rect = egui::Rect::from_min_size(rect.min + egui::vec2(4.0, 2.0), egui::vec2(16.0, 16.0));
                                                                let text_pos = rect.min + egui::vec2(24.0, 4.0);
                                                                if let Some(texture) = self.icon_textures.get(default_icon_name) {
                                                                    ui.painter().image(texture.id(), image_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                                                                }
                                                                ui.painter().text(text_pos, egui::Align2::LEFT_TOP, "Default", egui::FontId::proportional(11.0), visual.fg_stroke.color);
                                                            }
                                                            
                                                            // Custom options selection
                                                            for icon_name in &available_icons {
                                                                let is_selected = dev.low_battery_icon_path.as_deref() == Some(icon_name);
                                                                let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 20.0), egui::Sense::click());
                                                                if response.clicked() {
                                                                    dev.low_battery_icon_path = Some(icon_name.clone());
                                                                }
                                                                let visual = ui.style().interact(&response);
                                                                if is_selected || response.hovered() {
                                                                    ui.painter().rect_filled(rect, 2.0, visual.bg_fill);
                                                                }
                                                                let image_rect = egui::Rect::from_min_size(rect.min + egui::vec2(4.0, 2.0), egui::vec2(16.0, 16.0));
                                                                let text_pos = rect.min + egui::vec2(24.0, 4.0);
                                                                if let Some(texture) = self.icon_textures.get(icon_name) {
                                                                    ui.painter().image(texture.id(), image_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                                                                }
                                                                ui.painter().text(text_pos, egui::Align2::LEFT_TOP, icon_name, egui::FontId::proportional(11.0), visual.fg_stroke.color);
                                                            }
                                                        });
                                                    
                                                    // Show preview next to the combobox
                                                    if let Some(texture) = self.icon_textures.get(&preview_icon_name) {
                                                        ui.image((texture.id(), egui::vec2(20.0, 20.0)));
                                                        ui.add_space(4.0);
                                                    }
                                                });
                                            });
                                            
                                            ui.add_space(6.0);
                                            
                                            // Row 3: Alert Threshold Slider
                                            ui.vertical(|ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(egui::RichText::new("Alert Threshold").color(egui::Color32::from_rgb(0x8d, 0x8d, 0x8d)).font(egui::FontId::proportional(11.0)));
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        ui.label(egui::RichText::new(format!("{}%", dev.threshold)).color(egui::Color32::from_rgb(0x4c, 0xc9, 0xf0)).font(egui::FontId::monospace(11.0)).strong());
                                                    });
                                                });
                                                ui.add_space(2.0);
                                                ui.scope(|ui| {
                                                    ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_rgb(0x39, 0x39, 0x52);
                                                    ui.visuals_mut().widgets.inactive.fg_stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(0x4c, 0xc9, 0xf0));
                                                    ui.spacing_mut().slider_width = ui.available_width() - 8.0;
                                                    ui.add(egui::Slider::new(&mut dev.threshold, 5..=95).show_value(false).trailing_fill(true));
                                                });
                                            });
                                            
                                            // Row 4: Remove Button (only shown for offline/disconnected devices)
                                            if !is_active {
                                                ui.add_space(4.0);
                                                ui.horizontal(|ui| {
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        let remove_btn = egui::Button::new(
                                                            egui::RichText::new("🗑 Remove Device")
                                                                .color(egui::Color32::from_rgb(0xda, 0x1e, 0x28)) // error red
                                                                .font(egui::FontId::proportional(10.0))
                                                        ).fill(egui::Color32::TRANSPARENT);
                                                        
                                                        if ui.add(remove_btn).clicked() {
                                                            to_remove = Some(idx);
                                                        }
                                                    });
                                                });
                                            }
                                        });
                                    });
                                ui.add_space(10.0);
                            }
                            
                            if let Some(idx) = to_remove {
                                self.config.devices.remove(idx);
                                self.device_removed = true;
                            }
                        });
                    });
            });
    }
}
