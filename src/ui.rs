use crate::config::{AppConfig, save_config};
use crate::autostart::set_autostart;
use crate::plugins::DeviceBatteryStatus;
use eframe::egui;

pub struct SettingsWindow {
    pub config: AppConfig,
    pub active_devices: Vec<String>, // list of active unique_ids
    pub device_statuses: std::collections::HashMap<String, DeviceBatteryStatus>,
    pub request_close: bool,
    pub request_poll: bool,
}

impl SettingsWindow {
    pub fn new(
        config: AppConfig,
        active_devices: Vec<String>,
        device_statuses: std::collections::HashMap<String, DeviceBatteryStatus>,
    ) -> Self {
        Self {
            config,
            active_devices,
            device_statuses,
            request_close: false,
            request_poll: false,
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
                                            ui.add(egui::Slider::new(&mut self.config.polling_interval_secs, 10..=3600).show_value(false).trailing_fill(true));
                                        });
                                        
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("10s").color(egui::Color32::from_rgb(0x8d, 0x8d, 0x8d)).font(egui::FontId::proportional(9.0)));
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.label(egui::RichText::new("3600s").color(egui::Color32::from_rgb(0x8d, 0x8d, 0x8d)).font(egui::FontId::proportional(9.0)));
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
                                                    
                                                    if dev.unique_id == "gamebuds" {
                                                        // Unified SteelSeries GameBuds Left/Right battery displays
                                                        if let Some(status) = self.device_statuses.get(&dev.unique_id) {
                                                            ui.horizontal(|ui| {
                                                                // Left Bud status
                                                                let l_on = status.left_online.unwrap_or(false);
                                                                let l_pct = status.left_percentage.unwrap_or(0);
                                                                let l_chg = status.left_charging.unwrap_or(false);
                                                                
                                                                let l_dot_color = if l_on { egui::Color32::from_rgb(0x00, 0xe6, 0x76) } else { egui::Color32::from_rgb(0x8d, 0x8d, 0x8d) };
                                                                let (l_rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                                                                ui.painter().circle_filled(l_rect.center(), 3.0, l_dot_color);
                                                                
                                                                let l_text = if l_on {
                                                                    format!("L: {}%{}", l_pct, if l_chg { " ⚡" } else { "" })
                                                                } else {
                                                                    "L: OFF".to_string()
                                                                };
                                                                ui.label(egui::RichText::new(l_text).color(l_dot_color).font(egui::FontId::proportional(10.0)).strong());
                                                                
                                                                ui.add_space(6.0);
                                                                ui.label(egui::RichText::new("|").color(egui::Color32::from_rgb(0x39, 0x39, 0x52)));
                                                                ui.add_space(6.0);
                                                                
                                                                // Right Bud status
                                                                let r_on = status.right_online.unwrap_or(false);
                                                                let r_pct = status.right_percentage.unwrap_or(0);
                                                                let r_chg = status.right_charging.unwrap_or(false);
                                                                
                                                                let r_dot_color = if r_on { egui::Color32::from_rgb(0x00, 0xe6, 0x76) } else { egui::Color32::from_rgb(0x8d, 0x8d, 0x8d) };
                                                                let (r_rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                                                                ui.painter().circle_filled(r_rect.center(), 3.0, r_dot_color);
                                                                
                                                                let r_text = if r_on {
                                                                    format!("R: {}%{}", r_pct, if r_chg { " ⚡" } else { "" })
                                                                } else {
                                                                    "R: OFF".to_string()
                                                                };
                                                                ui.label(egui::RichText::new(r_text).color(r_dot_color).font(egui::FontId::proportional(10.0)).strong());
                                                            });
                                                        } else {
                                                            ui.horizontal(|ui| {
                                                                let dot_color = egui::Color32::from_rgb(0x8d, 0x8d, 0x8d);
                                                                let (rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                                                                ui.painter().circle_filled(rect.center(), 3.0, dot_color);
                                                                ui.label(egui::RichText::new("DISCONNECTED").color(dot_color).font(egui::FontId::proportional(10.0)).strong());
                                                            });
                                                        }
                                                    } else {
                                                        // Standard Device battery display
                                                        ui.horizontal(|ui| {
                                                            let dot_color = if is_active { egui::Color32::from_rgb(0x00, 0xe6, 0x76) } else { egui::Color32::from_rgb(0x8d, 0x8d, 0x8d) };
                                                            let (rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                                                            ui.painter().circle_filled(rect.center(), 3.0, dot_color);
                                                            
                                                            let status_str = if is_active {
                                                                if let Some(status) = self.device_statuses.get(&dev.unique_id) {
                                                                    format!("{}%{}", status.percentage, if status.charging { " ⚡" } else { "" })
                                                                } else {
                                                                    "ONLINE".to_string()
                                                                }
                                                            } else {
                                                                "DISCONNECTED".to_string()
                                                            };
                                                            ui.label(egui::RichText::new(status_str).color(dot_color).font(egui::FontId::proportional(10.0)).strong());
                                                        });
                                                    }
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
                                                ui.label(egui::RichText::new("DEVICE ICON").color(egui::Color32::from_rgb(0x8d, 0x8d, 0x8d)).font(egui::FontId::proportional(9.0)).strong());
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    if ui.button(egui::RichText::new("Choose...").font(egui::FontId::proportional(10.0))).clicked() {
                                                        if let Some(path) = rfd::FileDialog::new()
                                                            .add_filter("Image", &["png", "ico"])
                                                            .pick_file() 
                                                        {
                                                            dev.low_battery_icon_path = Some(path.to_string_lossy().into_owned());
                                                        }
                                                    }
                                                    
                                                    if dev.low_battery_icon_path.is_some() {
                                                        if ui.button(egui::RichText::new("Reset").font(egui::FontId::proportional(10.0))).clicked() {
                                                            dev.low_battery_icon_path = None;
                                                        }
                                                    }
                                                    
                                                    let path_str = dev.low_battery_icon_path.as_deref().unwrap_or("Default");
                                                    let preview_text = if path_str.len() > 18 {
                                                        format!("...{}", &path_str[path_str.len()-15..])
                                                    } else {
                                                        path_str.to_string()
                                                    };
                                                    ui.label(egui::RichText::new(preview_text).color(egui::Color32::from_rgb(0x8d, 0x8d, 0x8d)).font(egui::FontId::monospace(10.0)));
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
                            }
                        });
                    });
            });
    }
}
