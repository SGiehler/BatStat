use crate::autostart::set_autostart;
use crate::config::{get_config_path, resolve_icons_dir, setup_icons_folder_at};
use crate::plugins::{ChannelType, DeviceBatteryStatus};
use crate::ui::theme::{self, ActionIcon};
use crate::ui::widgets;
use crate::ui::{get_available_channels, SettingsWindow};
use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Sense, Stroke, Vec2};

fn section_label(ui: &mut egui::Ui, label: &str) {
    ui.label(
        egui::RichText::new(label)
            .color(theme::TEXT_DIM)
            .font(egui::FontId::proportional(10.0))
            .strong(),
    );
    ui.add_space(4.0);
}

fn compact_path_display(path: &std::path::Path, max_chars: usize) -> String {
    let raw = path.to_string_lossy();
    let count = raw.chars().count();
    if count <= max_chars {
        raw.into_owned()
    } else {
        let tail: String = raw
            .chars()
            .skip(count.saturating_sub(max_chars.saturating_sub(1)))
            .collect();
        format!("…{}", tail)
    }
}

pub fn render_preferences_view(win: &mut SettingsWindow, ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        // =========================================================
        // 1. TASKBAR & POLLING
        // =========================================================
        section_label(ui, "TASKBAR & POLLING");
        egui::Frame::none()
            .fill(theme::BG_CARD)
            .stroke(Stroke::new(1.0, theme::BORDER_CARD))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::symmetric(12.0, 10.0))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Taskbar `%` Readout row + 20×20 live preview
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Tray readout")
                                .color(theme::TEXT_PRIMARY)
                                .font(egui::FontId::proportional(12.5)),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Live 26×20 tray icon preview pill
                            let (prev_rect, _) =
                                ui.allocate_exact_size(Vec2::new(30.0, 22.0), Sense::hover());
                            ui.painter().rect_filled(prev_rect, 5.0, theme::BG_DRAWER);
                            ui.painter().rect_stroke(
                                prev_rect,
                                5.0,
                                Stroke::new(1.0, theme::BORDER_CARD),
                            );

                            if let Some(ref ch_id) = win.config.tray_battery_channel {
                                let mut pct_label = "85".to_string();
                                let mut pct_col = theme::TEXT_PRIMARY;
                                for (dev_id, status) in &win.device_statuses {
                                    if let DeviceBatteryStatus::Online { channels } = status {
                                        for chan in channels.iter().flatten() {
                                            let key = match chan.channel_type {
                                                ChannelType::Main => format!("{}:Main", dev_id),
                                                ChannelType::Left => format!("{}:Left", dev_id),
                                                ChannelType::Right => format!("{}:Right", dev_id),
                                                ChannelType::Case => format!("{}:Case", dev_id),
                                            };
                                            if &key == ch_id {
                                                pct_label = format!("{}", chan.percentage);
                                                if chan.charging {
                                                    pct_col = theme::ACCENT_ONLINE;
                                                }
                                            }
                                        }
                                    }
                                }
                                ui.painter().text(
                                    prev_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    pct_label,
                                    egui::FontId::monospace(10.5),
                                    pct_col,
                                );
                            } else if let Some(tex) = win.icon_textures.get("ok.png") {
                                let icon_rect =
                                    Rect::from_center_size(prev_rect.center(), Vec2::new(14.0, 14.0));
                                ui.painter().image(
                                    tex.id(),
                                    icon_rect,
                                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                            }

                            let channels = get_available_channels(&win.config);
                            let cur = win.config.tray_battery_channel.clone();
                            let sel_label = cur
                                .as_ref()
                                .and_then(|id| {
                                    channels
                                        .iter()
                                        .find(|(cid, _)| cid == id)
                                        .map(|(_, label)| label.clone())
                                })
                                .unwrap_or_else(|| "Default icon".to_string());

                            egui::ComboBox::from_id_salt("flyout_tray_channel_combo")
                                .selected_text(
                                    egui::RichText::new(sel_label)
                                        .font(egui::FontId::proportional(11.5))
                                        .color(theme::TEXT_PRIMARY),
                                )
                                .width(152.0)
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_label(cur.is_none(), "Default icon")
                                        .clicked()
                                        && cur.is_some()
                                    {
                                        win.config.tray_battery_channel = None;
                                        win.mark_dirty_immediate();
                                    }
                                    for (cid, label) in &channels {
                                        let is_sel = cur.as_ref() == Some(cid);
                                        if ui.selectable_label(is_sel, label).clicked() && !is_sel {
                                            win.config.tray_battery_channel = Some(cid.clone());
                                            win.mark_dirty_immediate();
                                        }
                                    }
                                });
                        });
                    });

                    ui.add_space(8.0);
                    let avail = ui.available_width();
                    let (sep, _) = ui.allocate_exact_size(Vec2::new(avail, 1.0), Sense::hover());
                    ui.painter().line_segment(
                        [sep.left_center(), sep.right_center()],
                        Stroke::new(1.0, theme::BORDER_CARD),
                    );
                    ui.add_space(8.0);

                    // Poll interval segmented selector
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Poll interval")
                                .color(theme::TEXT_PRIMARY)
                                .font(egui::FontId::proportional(12.5)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if widgets::segmented_poll_selector(
                                ui,
                                &mut win.config.polling_interval_secs,
                            ) {
                                win.mark_dirty_immediate();
                                win.request_poll = true;
                                win.show_toast(&format!(
                                    "Polling every {}s",
                                    win.config.polling_interval_secs
                                ));
                            }
                        });
                    });
                });
            });

        ui.add_space(12.0);

        // =========================================================
        // 2. SYSTEM BEHAVIOR
        // =========================================================
        section_label(ui, "SYSTEM BEHAVIOR");
        egui::Frame::none()
            .fill(theme::BG_CARD)
            .stroke(Stroke::new(1.0, theme::BORDER_CARD))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::symmetric(12.0, 10.0))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Launch at Login
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Launch at login")
                                .color(theme::TEXT_PRIMARY)
                                .font(egui::FontId::proportional(12.5)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let mut autostart = win.config.autostart;
                            if widgets::toggle_switch(ui, &mut autostart).changed() {
                                win.config.autostart = autostart;
                                win.mark_dirty_immediate();
                                if let Err(e) = set_autostart(autostart) {
                                    eprintln!("Failed to set autostart: {}", e);
                                    win.show_toast("Failed to update startup registry");
                                } else {
                                    win.show_toast(if autostart {
                                        "Launch at login enabled"
                                    } else {
                                        "Launch at login disabled"
                                    });
                                }
                            }
                        });
                    });

                    ui.add_space(8.0);
                    let avail = ui.available_width();
                    let (sep, _) = ui.allocate_exact_size(Vec2::new(avail, 1.0), Sense::hover());
                    ui.painter().line_segment(
                        [sep.left_center(), sep.right_center()],
                        Stroke::new(1.0, theme::BORDER_CARD),
                    );
                    ui.add_space(8.0);

                    // Low Battery Notifications + Test
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Low-battery alerts")
                                .color(theme::TEXT_PRIMARY)
                                .font(egui::FontId::proportional(12.5)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let mut notif = win.config.enable_notifications;
                            if widgets::toggle_switch(ui, &mut notif).changed() {
                                win.config.enable_notifications = notif;
                                win.mark_dirty_immediate();
                            }
                            if win.config.enable_notifications {
                                let test_btn = ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Test")
                                                .color(theme::TEXT_SECONDARY)
                                                .font(egui::FontId::proportional(10.5)),
                                        )
                                        .fill(theme::BG_SUBTLE)
                                        .stroke(Stroke::new(1.0, theme::BORDER_CARD))
                                        .rounding(Rounding::same(6.0)),
                                    )
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .on_hover_text("Send sample low-battery toast notification");
                                if test_btn.clicked() {
                                    win.request_test_notification = true;
                                    win.show_toast("Test notification sent");
                                }
                            }
                        });
                    });
                });
            });

        ui.add_space(12.0);

        // =========================================================
        // 3. ICONS & STORAGE (Configurable Icons Folder)
        // =========================================================
        section_label(ui, "ICONS & STORAGE");
        egui::Frame::none()
            .fill(theme::BG_CARD)
            .stroke(Stroke::new(1.0, theme::BORDER_CARD))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::symmetric(12.0, 10.0))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    let resolved_dir = resolve_icons_dir(&win.config);
                    let is_custom = win
                        .config
                        .custom_icons_dir
                        .as_deref()
                        .map(|s| !s.trim().is_empty())
                        .unwrap_or(false);

                    ui.horizontal(|ui| {
                        let (f_rect, _) =
                            ui.allocate_exact_size(Vec2::new(15.0, 15.0), Sense::hover());
                        theme::draw_action_icon(
                            ui.painter(),
                            f_rect,
                            ActionIcon::Folder,
                            theme::ACCENT_ONLINE,
                            0.0,
                        );
                        ui.label(
                            egui::RichText::new("Icons folder")
                                .color(theme::TEXT_PRIMARY)
                                .font(egui::FontId::proportional(12.5)),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let badge_text = if is_custom { "Custom" } else { "Default" };
                            let badge_col = if is_custom {
                                theme::ACCENT_ONLINE
                            } else {
                                theme::TEXT_DIM
                            };
                            ui.label(
                                egui::RichText::new(badge_text)
                                    .color(badge_col)
                                    .font(egui::FontId::proportional(10.0))
                                    .strong(),
                            );
                        });
                    });

                    if let Some(ref dir) = resolved_dir {
                        ui.add_space(4.0);
                        let short_path = compact_path_display(dir, 44);
                        egui::Frame::none()
                            .fill(theme::BG_DRAWER)
                            .stroke(Stroke::new(1.0, theme::BORDER_CARD))
                            .rounding(Rounding::same(6.0))
                            .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                            .show(ui, |ui| {
                                ui.set_width((ui.available_width()).max(80.0));
                                ui.label(
                                    egui::RichText::new(short_path)
                                        .color(theme::TEXT_MUTED)
                                        .font(egui::FontId::monospace(10.0)),
                                )
                                .on_hover_text(dir.to_string_lossy().as_ref());
                            });
                    }

                    ui.add_space(7.0);

                    let mut pick_icons_folder = false;
                    ui.horizontal(|ui| {
                        let change_btn = ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Change…")
                                        .color(theme::TEXT_PRIMARY)
                                        .font(egui::FontId::proportional(11.0))
                                        .strong(),
                                )
                                .fill(theme::BG_SUBTLE)
                                .stroke(Stroke::new(1.0, theme::BORDER_CARD))
                                .rounding(Rounding::same(6.0)),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text("Choose a custom folder for tray icons");
                        if change_btn.clicked() {
                            pick_icons_folder = true;
                        }

                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Open")
                                        .color(theme::TEXT_SECONDARY)
                                        .font(egui::FontId::proportional(11.0)),
                                )
                                .fill(theme::BG_SUBTLE)
                                .stroke(Stroke::new(1.0, theme::BORDER_CARD))
                                .rounding(Rounding::same(6.0)),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text("Open active icons folder in Explorer")
                            .clicked()
                        {
                            if let Some(ref dir) = resolved_dir {
                                let _ = std::process::Command::new("explorer").arg(dir).spawn();
                            }
                        }

                        if is_custom
                            && ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Reset")
                                            .color(theme::TEXT_MUTED)
                                            .font(egui::FontId::proportional(11.0)),
                                    )
                                    .fill(theme::BG_SUBTLE)
                                    .stroke(Stroke::new(1.0, theme::BORDER_CARD))
                                    .rounding(Rounding::same(6.0)),
                                )
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .on_hover_text("Reset to %APPDATA%\\BatStat\\icons")
                                .clicked()
                        {
                            win.config.custom_icons_dir = None;
                            win.icon_textures.clear();
                            win.loaded_icons_dir = None;
                            win.mark_dirty_immediate();
                            win.show_toast("Reset to default icons folder");
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Config Dir")
                                            .color(theme::TEXT_MUTED)
                                            .font(egui::FontId::proportional(10.5)),
                                    )
                                    .fill(theme::BG_SUBTLE)
                                    .stroke(Stroke::new(1.0, theme::BORDER_CARD))
                                    .rounding(Rounding::same(6.0)),
                                )
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .on_hover_text("Open config.toml folder in Explorer")
                                .clicked()
                            {
                                if let Some(mut cfg_path) = get_config_path() {
                                    cfg_path.pop();
                                    let _ = std::process::Command::new("explorer")
                                        .arg(cfg_path)
                                        .spawn();
                                }
                            }
                        });
                    });

                    if pick_icons_folder {
                        win.suppress_focus_hide = true;
                        let mut dialog =
                            rfd::FileDialog::new().set_title("Select BatStat Icons Folder");
                        if let Some(ref dir) = resolved_dir {
                            if dir.exists() {
                                dialog = dialog.set_directory(dir);
                            }
                        }
                        let chosen = dialog.pick_folder();
                        win.suppress_focus_hide = false;

                        if let Some(folder) = chosen {
                            setup_icons_folder_at(&folder);
                            win.config.custom_icons_dir =
                                Some(folder.to_string_lossy().into_owned());
                            win.icon_textures.clear();
                            win.loaded_icons_dir = None;
                            win.mark_dirty_immediate();
                            win.show_toast("Icons folder updated");
                        }
                    }
                });
            });

        ui.add_space(12.0);

        // =========================================================
        // 4. UPDATES & DIAGNOSTICS
        // =========================================================
        section_label(ui, "UPDATES & DIAGNOSTICS");
        egui::Frame::none()
            .fill(theme::BG_CARD)
            .stroke(Stroke::new(1.0, theme::BORDER_CARD))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::symmetric(12.0, 10.0))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Version + Updater row
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(concat!(
                                    "BatStat v",
                                    env!("CARGO_PKG_VERSION")
                                ))
                                .color(theme::TEXT_PRIMARY)
                                .font(egui::FontId::proportional(12.5))
                                .strong(),
                            );
                            let (status_txt, status_col) = match &win.update_status {
                                crate::UpdateStatus::Idle => {
                                    ("GitHub releases".to_string(), theme::TEXT_DIM)
                                }
                                crate::UpdateStatus::Checking => {
                                    ("Checking for updates…".to_string(), theme::TEXT_MUTED)
                                }
                                crate::UpdateStatus::Available(info) => (
                                    format!("{} available", info.tag_name),
                                    theme::ACCENT_ONLINE,
                                ),
                                crate::UpdateStatus::NoUpdate => {
                                    ("Up to date".to_string(), theme::ACCENT_ONLINE)
                                }
                                crate::UpdateStatus::Downloading(p) => (
                                    format!("Downloading {:.0}%…", p * 100.0),
                                    theme::ACCENT_CHARGING,
                                ),
                                crate::UpdateStatus::ReadyToInstall(_) => {
                                    ("Restarting to install…".to_string(), theme::ACCENT_ONLINE)
                                }
                                crate::UpdateStatus::Error(_) => {
                                    ("Update check failed".to_string(), theme::ACCENT_LOW)
                                }
                            };
                            ui.label(
                                egui::RichText::new(status_txt)
                                    .color(status_col)
                                    .font(egui::FontId::proportional(10.5)),
                            );
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            match &win.update_status {
                                crate::UpdateStatus::Available(info) => {
                                    let msi = info
                                        .assets
                                        .iter()
                                        .find(|a| a.name.ends_with(".msi"))
                                        .map(|a| a.browser_download_url.clone());
                                    if let Some(url) = msi {
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    egui::RichText::new("Install")
                                                        .color(theme::BG_FLYOUT)
                                                        .font(egui::FontId::proportional(11.0))
                                                        .strong(),
                                                )
                                                .fill(theme::ACCENT_ONLINE)
                                                .rounding(Rounding::same(6.0)),
                                            )
                                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                                            .clicked()
                                        {
                                            win.request_download_install = Some(url);
                                        }
                                    } else if ui
                                        .button(
                                            egui::RichText::new("Release")
                                                .font(egui::FontId::proportional(11.0)),
                                        )
                                        .clicked()
                                    {
                                        let _ = std::process::Command::new("explorer")
                                            .arg("https://github.com/SGiehler/BatStat/releases/latest")
                                            .spawn();
                                    }
                                }
                                crate::UpdateStatus::Checking
                                | crate::UpdateStatus::Downloading(_) => {
                                    ui.add_enabled(
                                        false,
                                        egui::Button::new(
                                            egui::RichText::new("Busy…")
                                                .font(egui::FontId::proportional(10.5)),
                                        )
                                        .rounding(Rounding::same(6.0)),
                                    );
                                }
                                _ => {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                egui::RichText::new("Check")
                                                    .color(theme::TEXT_SECONDARY)
                                                    .font(egui::FontId::proportional(11.0)),
                                            )
                                            .fill(theme::BG_SUBTLE)
                                            .stroke(Stroke::new(1.0, theme::BORDER_CARD))
                                            .rounding(Rounding::same(6.0)),
                                        )
                                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                                        .clicked()
                                    {
                                        win.request_update_check = true;
                                    }
                                }
                            }
                        });
                    });

                    if let crate::UpdateStatus::Downloading(progress) = win.update_status {
                        ui.add_space(6.0);
                        let avail = ui.available_width();
                        let (bar, _) = ui.allocate_exact_size(Vec2::new(avail, 4.0), Sense::hover());
                        ui.painter().rect_filled(bar, 2.0, theme::BG_DRAWER);
                        let fw = bar.width() * progress.clamp(0.0, 1.0);
                        if fw > 0.5 {
                            ui.painter().rect_filled(
                                Rect::from_min_size(bar.min, Vec2::new(fw, bar.height())),
                                2.0,
                                theme::ACCENT_ONLINE,
                            );
                        }
                    }

                    ui.add_space(8.0);
                    let avail = ui.available_width();
                    let (sep, _) = ui.allocate_exact_size(Vec2::new(avail, 1.0), Sense::hover());
                    ui.painter().line_segment(
                        [sep.left_center(), sep.right_center()],
                        Stroke::new(1.0, theme::BORDER_CARD),
                    );
                    ui.add_space(6.0);

                    // Collapsible Debug Logs header
                    let log_toggle_resp = ui.horizontal(|ui| {
                        let (chev_rect, chev_click) =
                            ui.allocate_exact_size(Vec2::new(12.0, 16.0), Sense::click());
                        let cc = chev_rect.center();
                        let stroke = Stroke::new(1.4, theme::TEXT_MUTED);
                        if win.logs_expanded {
                            ui.painter().line_segment(
                                [Pos2::new(cc.x - 3.2, cc.y - 1.5), Pos2::new(cc.x, cc.y + 1.8)],
                                stroke,
                            );
                            ui.painter().line_segment(
                                [Pos2::new(cc.x, cc.y + 1.8), Pos2::new(cc.x + 3.2, cc.y - 1.5)],
                                stroke,
                            );
                        } else {
                            ui.painter().line_segment(
                                [Pos2::new(cc.x - 1.5, cc.y - 3.2), Pos2::new(cc.x + 1.8, cc.y)],
                                stroke,
                            );
                            ui.painter().line_segment(
                                [Pos2::new(cc.x + 1.8, cc.y), Pos2::new(cc.x - 1.5, cc.y + 3.2)],
                                stroke,
                            );
                        }

                        let btn = ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Debug logs")
                                        .color(theme::TEXT_SECONDARY)
                                        .font(egui::FontId::proportional(11.5)),
                                )
                                .frame(false),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if btn.clicked() || chev_click.clicked() {
                            win.logs_expanded = !win.logs_expanded;
                            if win.logs_expanded {
                                win.refresh_log();
                            }
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let mut dbg = win.config.enable_debug_logging;
                            if widgets::toggle_switch(ui, &mut dbg).changed() {
                                win.config.enable_debug_logging = dbg;
                                win.mark_dirty_immediate();
                                win.show_toast(if dbg {
                                    "Debug logging enabled"
                                } else {
                                    "Debug logging disabled"
                                });
                            }
                        });
                    });
                    let _ = log_toggle_resp;

                    if win.logs_expanded {
                        if win.debug_log_content.is_none() {
                            win.refresh_log();
                        }
                        ui.add_space(6.0);

                        let log_path = get_config_path().map(|mut p| {
                            p.pop();
                            p.push("debug.log");
                            p
                        });

                        ui.horizontal(|ui| {
                            if ui
                                .button(
                                    egui::RichText::new("Refresh")
                                        .font(egui::FontId::proportional(10.0)),
                                )
                                .clicked()
                            {
                                win.refresh_log();
                                win.show_toast("Log refreshed");
                            }
                            if let Some(ref lp) = log_path {
                                if lp.exists() {
                                    if ui
                                        .button(
                                            egui::RichText::new("Copy")
                                                .font(egui::FontId::proportional(10.0)),
                                        )
                                        .clicked()
                                    {
                                        if let Some(ref content) = win.debug_log_content {
                                            ui.output_mut(|o| o.copied_text = content.clone());
                                            win.show_toast("Copied log");
                                        }
                                    }
                                    if ui
                                        .button(
                                            egui::RichText::new("Clear")
                                                .font(egui::FontId::proportional(10.0)),
                                        )
                                        .clicked()
                                    {
                                        let _ = std::fs::write(lp, "");
                                        win.debug_log_content = Some(String::new());
                                        win.show_toast("Log cleared");
                                    }
                                    if ui
                                        .button(
                                            egui::RichText::new("Open")
                                                .font(egui::FontId::proportional(10.0)),
                                        )
                                        .clicked()
                                    {
                                        let _ =
                                            std::process::Command::new("notepad").arg(lp).spawn();
                                    }
                                }
                            }
                        });

                        ui.add_space(4.0);
                        let log_text = win
                            .debug_log_content
                            .as_deref()
                            .unwrap_or("No logs recorded yet.");
                        egui::Frame::none()
                            .fill(theme::BG_DRAWER)
                            .stroke(Stroke::new(1.0, theme::BORDER_CARD))
                            .rounding(Rounding::same(6.0))
                            .inner_margin(egui::Margin::same(7.0))
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("flyout_debug_logs_scroll")
                                    .max_height(100.0)
                                    .stick_to_bottom(true)
                                    .show(ui, |ui| {
                                        ui.set_width((ui.available_width()).max(80.0));
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(log_text)
                                                    .color(theme::TEXT_MUTED)
                                                    .font(egui::FontId::monospace(9.5)),
                                            )
                                            .wrap(),
                                        );
                                    });
                            });
                    }
                });
            });

        ui.add_space(8.0);
    });
}
