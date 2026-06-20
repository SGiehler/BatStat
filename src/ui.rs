use crate::config::{AppConfig, save_config};
use crate::autostart::set_autostart;
use eframe::egui;

pub struct SettingsWindow {
    pub config: AppConfig,
    pub active_devices: Vec<String>, // list of active unique_ids
    pub request_close: bool,
}

impl SettingsWindow {
    pub fn new(config: AppConfig, active_devices: Vec<String>) -> Self {
        Self {
            config,
            active_devices,
            request_close: false,
        }
    }
}

impl eframe::App for SettingsWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("BatStat Settings");
            ui.add_space(8.0);

            // Global Config Section
            ui.group(|ui| {
                ui.label("Global Settings");
                ui.add_space(4.0);

                // Polling Interval
                ui.horizontal(|ui| {
                    ui.label("Polling Interval:");
                    ui.add(egui::Slider::new(&mut self.config.polling_interval_secs, 10..=3600).suffix("s"));
                });

                // Autostart
                let mut autostart = self.config.autostart;
                if ui.checkbox(&mut autostart, "Start BatStat with Windows").changed() {
                    self.config.autostart = autostart;
                    if let Err(e) = set_autostart(autostart) {
                        eprintln!("Failed to set autostart: {}", e);
                    }
                }
            });

            ui.add_space(12.0);
            ui.label("Peripherals & Battery Thresholds:");
            ui.add_space(4.0);

            // Scroll Area for Devices List
            egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                let mut to_remove = None;

                for (idx, dev) in self.config.devices.iter_mut().enumerate() {
                    let is_active = self.active_devices.contains(&dev.unique_id);
                    
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            // Online/Offline status indicator dot
                            let dot_color = if is_active { egui::Color32::GREEN } else { egui::Color32::GRAY };
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                            ui.painter().circle_filled(rect.center(), 4.0, dot_color);
                            ui.add_space(4.0);

                            ui.checkbox(&mut dev.enabled, "");
                            ui.text_edit_singleline(&mut dev.name);

                            if !is_active {
                                ui.weak("(Disconnected)");
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("Remove").clicked() {
                                    to_remove = Some(idx);
                                }
                            });
                        });

                        ui.horizontal(|ui| {
                            ui.label("Alert Threshold:");
                            ui.add(egui::Slider::new(&mut dev.threshold, 5..=95).suffix("%"));
                        });

                        ui.horizontal(|ui| {
                            ui.label("Custom Low Icon:");
                            let path_str = dev.low_battery_icon_path.as_deref().unwrap_or("");
                            ui.weak(if path_str.is_empty() { "Default" } else { path_str });
                            
                            if ui.button("Choose...").clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("Image", &["png", "ico"])
                                    .pick_file() 
                                {
                                    dev.low_battery_icon_path = Some(path.to_string_lossy().into_owned());
                                }
                            }
                            if dev.low_battery_icon_path.is_some() {
                                if ui.button("Reset").clicked() {
                                    dev.low_battery_icon_path = None;
                                }
                            }
                        });
                    });
                    ui.add_space(4.0);
                }

                if let Some(idx) = to_remove {
                    self.config.devices.remove(idx);
                }
            });

            ui.add_space(12.0);

            // Bottom Buttons
            ui.horizontal(|ui| {
                if ui.button("Save & Close").clicked() {
                    if let Err(e) = save_config(&self.config) {
                        eprintln!("Failed to save config: {}", e);
                    }
                    self.request_close = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }

                if ui.button("Cancel").clicked() {
                    self.request_close = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }
}
