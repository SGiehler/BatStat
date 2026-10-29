use crate::config::{get_icon_list_for, import_icon_file, resolve_icons_dir};
use crate::plugins::{BatteryChannel, ChannelType, DeviceBatteryStatus};
use crate::ui::theme::{self, ActionIcon, DeviceKind};
use crate::ui::widgets;
use crate::ui::{format_icon_label, get_device_category, reorder_devices, SettingsWindow};
use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use std::time::Instant;

/// Draws a crisp vector chevron (`expanded == false` -> down `▾`, `expanded == true` -> up `▴`)
fn draw_chevron(painter: &egui::Painter, center: Pos2, expanded: bool, color: Color32) {
    let w = 3.8;
    let h = 2.2;
    let stroke = Stroke::new(1.5, color);
    if expanded {
        painter.line_segment(
            [Pos2::new(center.x - w, center.y + h * 0.5), Pos2::new(center.x, center.y - h * 0.5)],
            stroke,
        );
        painter.line_segment(
            [Pos2::new(center.x, center.y - h * 0.5), Pos2::new(center.x + w, center.y + h * 0.5)],
            stroke,
        );
    } else {
        painter.line_segment(
            [Pos2::new(center.x - w, center.y - h * 0.5), Pos2::new(center.x, center.y + h * 0.5)],
            stroke,
        );
        painter.line_segment(
            [Pos2::new(center.x, center.y + h * 0.5), Pos2::new(center.x + w, center.y - h * 0.5)],
            stroke,
        );
    }
}

pub fn render_devices_view(win: &mut SettingsWindow, ui: &mut egui::Ui) {
    if win.config.devices.is_empty() {
        render_empty_state(win, ui);
        return;
    }

    // Handle dropped icon files when a device drawer is expanded
    if let Some(expanded_id) = win.expanded_device_id.clone() {
        let dropped_paths: Vec<std::path::PathBuf> = ui.ctx().input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        if let Some(src_path) = dropped_paths.first() {
            if let Some(icons_dir) = resolve_icons_dir(&win.config) {
                match import_icon_file(src_path, &icons_dir) {
                    Ok(filename) => {
                        if let Some(dev) = win
                            .config
                            .devices
                            .iter_mut()
                            .find(|d| d.unique_id == expanded_id)
                        {
                            dev.low_battery_icon_path = Some(filename.clone());
                            win.icon_textures.clear();
                            win.loaded_icons_dir = None;
                            win.mark_dirty_immediate();
                            win.show_toast(&format!("Alert icon set to {}", filename));
                        }
                    }
                    Err(err) => {
                        win.show_toast(&err);
                    }
                }
            }
        }
    }

    // Partition indices into Online+Enabled cards first, then Offline/Paused compact rows,
    // preserving `win.config.devices` priority order within each group.
    let mut online_indices = Vec::new();
    let mut offline_indices = Vec::new();
    for (idx, dev) in win.config.devices.iter().enumerate() {
        let is_online = dev.enabled
            && win.active_devices.contains(&dev.unique_id)
            && win
                .device_statuses
                .get(&dev.unique_id)
                .map(|s| s.is_online())
                .unwrap_or(false);
        if is_online {
            online_indices.push(idx);
        } else {
            offline_indices.push(idx);
        }
    }

    let mut item_rects: Vec<(usize, Rect)> = Vec::with_capacity(win.config.devices.len());
    let mut remove_idx: Option<usize> = None;

    for (pos, &idx) in online_indices.iter().enumerate() {
        if pos > 0 {
            ui.add_space(7.0);
        }
        let rect = render_online_device_card(win, ui, idx, &mut remove_idx);
        item_rects.push((idx, rect));
    }

    if !offline_indices.is_empty() {
        if !online_indices.is_empty() {
            ui.add_space(12.0);
        }
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("OFFLINE & PAUSED")
                    .color(theme::TEXT_DIM)
                    .font(egui::FontId::proportional(10.0))
                    .strong(),
            );
            let avail = ui.available_width();
            if avail > 12.0 {
                let (line_rect, _) = ui.allocate_exact_size(Vec2::new(avail, 10.0), Sense::hover());
                ui.painter().line_segment(
                    [
                        Pos2::new(line_rect.left() + 4.0, line_rect.center().y),
                        Pos2::new(line_rect.right(), line_rect.center().y),
                    ],
                    Stroke::new(1.0, theme::BORDER_CARD),
                );
            }
        });
        ui.add_space(5.0);

        for (pos, &idx) in offline_indices.iter().enumerate() {
            if pos > 0 {
                ui.add_space(5.0);
            }
            let rect = render_offline_device_row(win, ui, idx, &mut remove_idx);
            item_rects.push((idx, rect));
        }
    }

    if let Some(idx) = remove_idx {
        if idx < win.config.devices.len() {
            let removed = win.config.devices.remove(idx);
            if win.expanded_device_id.as_deref() == Some(&removed.unique_id) {
                win.expanded_device_id = None;
            }
            win.confirming_remove_id = None;
            win.device_removed = true;
            win.mark_dirty_immediate();
            win.show_toast("Device removed");
        }
    }

    // Live drag-to-reorder across `win.config.devices`
    if let Some(drag_id) = win.dragging_device_id.clone() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        if let Some(pointer_pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
            if let Some(from_idx) = win
                .config
                .devices
                .iter()
                .position(|d| d.unique_id == drag_id)
            {
                let mut target_swap: Option<usize> = None;
                for &(other_idx, rect) in &item_rects {
                    if other_idx == from_idx {
                        continue;
                    }
                    if from_idx < other_idx && pointer_pos.y > rect.center().y {
                        target_swap = Some(other_idx);
                    } else if from_idx > other_idx && pointer_pos.y < rect.center().y {
                        target_swap = Some(other_idx);
                        break;
                    }
                }
                if let Some(to_idx) = target_swap {
                    if reorder_devices(&mut win.config.devices, from_idx, to_idx) {
                        win.drag_reordered = true;
                        ui.ctx().request_repaint();
                    }
                }
            }
        }

        if !ui.ctx().input(|i| i.pointer.any_down()) {
            win.dragging_device_id = None;
            if win.drag_reordered {
                win.drag_reordered = false;
                win.mark_dirty_immediate();
                win.show_toast("Priority order saved");
            }
        }
    }
}

fn render_empty_state(win: &mut SettingsWindow, ui: &mut egui::Ui) {
    ui.add_space(36.0);
    egui::Frame::none()
        .fill(theme::BG_CARD)
        .stroke(Stroke::new(1.0, theme::BORDER_CARD))
        .rounding(Rounding::same(12.0))
        .inner_margin(egui::Margin::symmetric(20.0, 28.0))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                let (badge, _) = ui.allocate_exact_size(Vec2::new(46.0, 46.0), Sense::hover());
                ui.painter().circle_filled(badge.center(), 23.0, theme::BG_SUBTLE);
                ui.painter().circle_stroke(
                    badge.center(),
                    23.0,
                    Stroke::new(1.0, theme::BORDER_CARD_HOVER),
                );
                let icon_rect = Rect::from_center_size(badge.center(), Vec2::new(22.0, 22.0));
                theme::draw_device_icon(
                    ui.painter(),
                    icon_rect,
                    DeviceKind::Mouse,
                    theme::ACCENT_ONLINE,
                );

                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new("No wireless peripherals detected")
                        .color(theme::TEXT_PRIMARY)
                        .font(egui::FontId::proportional(13.5))
                        .strong(),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(
                        "Connect a supported wireless mouse, headset, keyboard, or controller.",
                    )
                    .color(theme::TEXT_MUTED)
                    .font(egui::FontId::proportional(11.5)),
                );
                ui.add_space(14.0);

                let scan_btn = egui::Button::new(
                    egui::RichText::new("Scan Peripherals")
                        .color(theme::ACCENT_ONLINE)
                        .font(egui::FontId::proportional(11.5))
                        .strong(),
                )
                .fill(theme::ACCENT_ONLINE_BG)
                .stroke(Stroke::new(1.0, theme::ACCENT_ONLINE.linear_multiply(0.5)))
                .rounding(Rounding::same(7.0));

                if ui
                    .add(scan_btn)
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    win.request_poll = true;
                    win.scanning_timer = Some(Instant::now());
                    win.show_toast("Scanning peripherals…");
                }
            });
        });
}

fn render_online_device_card(
    win: &mut SettingsWindow,
    ui: &mut egui::Ui,
    idx: usize,
    remove_idx: &mut Option<usize>,
) -> Rect {
    let dev_snapshot = win.config.devices[idx].clone();
    let (_, dev_kind, _, base_type) = get_device_category(&dev_snapshot.unique_id);
    let status = win.device_statuses.get(&dev_snapshot.unique_id).copied();

    let active_channels: Vec<BatteryChannel> = match status {
        Some(DeviceBatteryStatus::Online { channels }) => {
            channels.into_iter().flatten().collect()
        }
        _ => Vec::new(),
    };

    let effective_pct = status.map(|s| s.effective_percentage()).unwrap_or(0);
    let any_charging = active_channels.iter().any(|c| c.charging);
    let is_low = !any_charging && effective_pct <= dev_snapshot.threshold;
    let is_expanded = win.expanded_device_id.as_deref() == Some(&dev_snapshot.unique_id);
    let is_dragging = win.dragging_device_id.as_deref() == Some(&dev_snapshot.unique_id);

    let card_fill = if is_low {
        theme::BG_LOW_CARD
    } else {
        theme::BG_CARD
    };
    let card_border = if is_dragging || is_expanded {
        Stroke::new(1.0, theme::ACCENT_ONLINE.linear_multiply(0.65))
    } else if is_low {
        Stroke::new(1.0, theme::BORDER_LOW_CARD)
    } else {
        Stroke::new(1.0, theme::BORDER_CARD)
    };

    let frame_resp = egui::Frame::none()
        .fill(card_fill)
        .stroke(card_border)
        .rounding(Rounding::same(10.0))
        .inner_margin(egui::Margin::same(0.0))
        .show(ui, |ui| {
            // Card main summary padding
            egui::Frame::none()
                .inner_margin(egui::Margin::symmetric(10.0, 10.0))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        let header_resp = ui.horizontal(|ui| {
                            // Drag grip handle (`⋮⋮`)
                            let (grip_rect, grip_resp) = ui.allocate_exact_size(
                                Vec2::new(16.0, 28.0),
                                Sense::drag(),
                            );
                            let grip_color = if grip_resp.hovered() || is_dragging {
                                theme::TEXT_PRIMARY
                            } else {
                                theme::TEXT_DIM
                            };
                            theme::draw_action_icon(
                                ui.painter(),
                                Rect::from_center_size(grip_rect.center(), Vec2::new(13.0, 13.0)),
                                ActionIcon::Grip,
                                grip_color,
                                0.0,
                            );
                            if grip_resp.drag_started() {
                                win.dragging_device_id = Some(dev_snapshot.unique_id.clone());
                            }
                            grip_resp
                                .on_hover_cursor(egui::CursorIcon::Grab)
                                .on_hover_text("Drag to reorder priority");

                            // Device category badge
                            let (icon_box, _) =
                                ui.allocate_exact_size(Vec2::new(30.0, 30.0), Sense::hover());
                            let badge_bg = if is_low {
                                theme::ACCENT_LOW_BG
                            } else if any_charging {
                                theme::ACCENT_CHARGING_BG
                            } else {
                                theme::BG_SUBTLE
                            };
                            let badge_fg = if is_low {
                                theme::ACCENT_LOW
                            } else if any_charging {
                                theme::ACCENT_CHARGING
                            } else {
                                theme::ACCENT_ONLINE
                            };
                            ui.painter().rect_filled(icon_box, 8.0, badge_bg);
                            theme::draw_device_icon(
                                ui.painter(),
                                Rect::from_center_size(icon_box.center(), Vec2::new(18.0, 18.0)),
                                dev_kind,
                                badge_fg,
                            );

                            ui.add_space(4.0);

                            // Clickable title & status subtitle area
                            let right_reserve = if active_channels.len() <= 1 { 68.0 } else { 22.0 };
                            let title_w = (ui.available_width() - right_reserve).max(60.0);
                            let (title_rect, title_resp) = ui.allocate_exact_size(
                                Vec2::new(title_w, 30.0),
                                Sense::click(),
                            );

                            if is_low || any_charging {
                                ui.painter().text(
                                    Pos2::new(title_rect.left(), title_rect.top() + 7.5),
                                    egui::Align2::LEFT_CENTER,
                                    &dev_snapshot.name,
                                    egui::FontId::proportional(13.0),
                                    theme::TEXT_PRIMARY,
                                );
                                let (sub_text, sub_col) = if any_charging {
                                    ("Charging", theme::ACCENT_CHARGING)
                                } else {
                                    ("Low battery", theme::ACCENT_LOW)
                                };
                                let mut text_x = title_rect.left();
                                if any_charging {
                                    theme::draw_bolt(
                                        ui.painter(),
                                        Pos2::new(text_x + 3.5, title_rect.bottom() - 6.0),
                                        0.85,
                                        theme::ACCENT_CHARGING,
                                    );
                                    text_x += 10.0;
                                }
                                ui.painter().text(
                                    Pos2::new(text_x, title_rect.bottom() - 6.0),
                                    egui::Align2::LEFT_CENTER,
                                    sub_text,
                                    egui::FontId::proportional(10.5),
                                    sub_col,
                                );
                            } else {
                                ui.painter().text(
                                    Pos2::new(title_rect.left(), title_rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    &dev_snapshot.name,
                                    egui::FontId::proportional(13.0),
                                    theme::TEXT_PRIMARY,
                                );
                            }

                            // Right side: Hero `%` (for single-channel) + expand chevron
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let (chev_rect, chev_resp) =
                                    ui.allocate_exact_size(Vec2::new(18.0, 28.0), Sense::click());
                                draw_chevron(
                                    ui.painter(),
                                    chev_rect.center(),
                                    is_expanded,
                                    if chev_resp.hovered() || title_resp.hovered() {
                                        theme::TEXT_PRIMARY
                                    } else {
                                        theme::TEXT_DIM
                                    },
                                );

                                if active_channels.len() <= 1 {
                                    let pct_color = if any_charging {
                                        theme::ACCENT_CHARGING
                                    } else if is_low {
                                        theme::ACCENT_LOW
                                    } else {
                                        theme::TEXT_PRIMARY
                                    };
                                    let pct_resp = ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(format!("{}%", effective_pct))
                                                .color(pct_color)
                                                .font(egui::FontId::monospace(15.5))
                                                .strong(),
                                        )
                                        .sense(Sense::click()),
                                    );
                                    if pct_resp.clicked() {
                                        toggle_drawer(win, &dev_snapshot.unique_id);
                                    }
                                    pct_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                                }

                                if chev_resp.clicked() {
                                    toggle_drawer(win, &dev_snapshot.unique_id);
                                }
                                chev_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                            });

                            if title_resp.clicked() {
                                toggle_drawer(win, &dev_snapshot.unique_id);
                            }
                            title_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        });
                        let _ = header_resp;

                        ui.add_space(7.0);

                        // Single-channel pill gauge OR Multi-channel L / R / Case mini-grid
                        if active_channels.len() <= 1 {
                            let avail_w = ui.available_width();
                            let bar_resp = widgets::pill_battery_bar(
                                ui,
                                effective_pct,
                                any_charging,
                                dev_snapshot.threshold,
                                avail_w,
                                6.0,
                            );
                            let click_bar = ui.interact(
                                bar_resp.rect,
                                ui.id().with(("bar_click", &dev_snapshot.unique_id)),
                                Sense::click(),
                            );
                            if click_bar.clicked() {
                                toggle_drawer(win, &dev_snapshot.unique_id);
                            }
                            click_bar.on_hover_cursor(egui::CursorIcon::PointingHand);
                        } else {
                            let cols = active_channels.len();
                            let gap = 6.0;
                            let total_w = ui.available_width();
                            let col_w = ((total_w - gap * (cols.saturating_sub(1) as f32))
                                / (cols as f32))
                                .max(68.0);

                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = gap;
                                for ch in &active_channels {
                                    let ch_label = match ch.channel_type {
                                        ChannelType::Left => "L",
                                        ChannelType::Right => "R",
                                        ChannelType::Case => "Case",
                                        ChannelType::Main => "Main",
                                    };
                                    let ch_low = !ch.charging && ch.percentage <= dev_snapshot.threshold;
                                    egui::Frame::none()
                                        .fill(theme::BG_SUBTLE)
                                        .stroke(Stroke::new(
                                            1.0,
                                            if ch_low {
                                                theme::BORDER_LOW_CARD
                                            } else {
                                                theme::BORDER_CARD
                                            },
                                        ))
                                        .rounding(Rounding::same(7.0))
                                        .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                                        .show(ui, |ui| {
                                            let inner_w = (col_w - 18.0).max(44.0);
                                            ui.set_width(inner_w);
                                            ui.vertical(|ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        egui::RichText::new(ch_label)
                                                            .color(theme::TEXT_MUTED)
                                                            .font(egui::FontId::proportional(10.5))
                                                            .strong(),
                                                    );
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            let pct_col = if ch.charging {
                                                                theme::ACCENT_CHARGING
                                                            } else if ch_low {
                                                                theme::ACCENT_LOW
                                                            } else {
                                                                theme::TEXT_PRIMARY
                                                            };
                                                            ui.label(
                                                                egui::RichText::new(format!(
                                                                    "{}%",
                                                                    ch.percentage
                                                                ))
                                                                .color(pct_col)
                                                                .font(egui::FontId::monospace(11.5))
                                                                .strong(),
                                                            );
                                                            if ch.charging {
                                                                let (b_rect, _) =
                                                                    ui.allocate_exact_size(
                                                                        Vec2::new(8.0, 11.0),
                                                                        Sense::hover(),
                                                                    );
                                                                theme::draw_bolt(
                                                                    ui.painter(),
                                                                    b_rect.center(),
                                                                    0.75,
                                                                    theme::ACCENT_CHARGING,
                                                                );
                                                            }
                                                        },
                                                    );
                                                });
                                                ui.add_space(4.0);
                                                let _ = widgets::pill_battery_bar(
                                                    ui,
                                                    ch.percentage,
                                                    ch.charging,
                                                    dev_snapshot.threshold,
                                                    inner_w,
                                                    4.0,
                                                );
                                            });
                                        });
                                }
                            });
                        }
                    });
                });

            if is_expanded {
                render_device_drawer(win, ui, idx, base_type, true, remove_idx);
            }
        });

    frame_resp.response.rect
}

fn render_offline_device_row(
    win: &mut SettingsWindow,
    ui: &mut egui::Ui,
    idx: usize,
    remove_idx: &mut Option<usize>,
) -> Rect {
    let dev_snapshot = win.config.devices[idx].clone();
    let (_, dev_kind, _, base_type) = get_device_category(&dev_snapshot.unique_id);
    let is_expanded = win.expanded_device_id.as_deref() == Some(&dev_snapshot.unique_id);
    let is_dragging = win.dragging_device_id.as_deref() == Some(&dev_snapshot.unique_id);

    let border = if is_dragging || is_expanded {
        Stroke::new(1.0, theme::ACCENT_ONLINE.linear_multiply(0.55))
    } else {
        Stroke::new(1.0, theme::BORDER_CARD)
    };

    let frame_resp = egui::Frame::none()
        .fill(theme::BG_SUBTLE)
        .stroke(border)
        .rounding(Rounding::same(8.0))
        .inner_margin(egui::Margin::same(0.0))
        .show(ui, |ui| {
            egui::Frame::none()
                .inner_margin(egui::Margin::symmetric(10.0, 7.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (grip_rect, grip_resp) =
                            ui.allocate_exact_size(Vec2::new(16.0, 22.0), Sense::drag());
                        theme::draw_action_icon(
                            ui.painter(),
                            Rect::from_center_size(grip_rect.center(), Vec2::new(12.0, 12.0)),
                            ActionIcon::Grip,
                            if grip_resp.hovered() || is_dragging {
                                theme::TEXT_SECONDARY
                            } else {
                                theme::TEXT_DIM
                            },
                            0.0,
                        );
                        if grip_resp.drag_started() {
                            win.dragging_device_id = Some(dev_snapshot.unique_id.clone());
                        }
                        grip_resp
                            .on_hover_cursor(egui::CursorIcon::Grab)
                            .on_hover_text("Drag to reorder priority");

                        let (icon_rect, _) =
                            ui.allocate_exact_size(Vec2::new(20.0, 20.0), Sense::hover());
                        theme::draw_device_icon(
                            ui.painter(),
                            icon_rect,
                            dev_kind,
                            theme::TEXT_DIM,
                        );

                        let row_w = (ui.available_width() - 78.0).max(60.0);
                        let (name_rect, name_resp) =
                            ui.allocate_exact_size(Vec2::new(row_w, 22.0), Sense::click());
                        ui.painter().text(
                            Pos2::new(name_rect.left() + 2.0, name_rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            &dev_snapshot.name,
                            egui::FontId::proportional(12.5),
                            theme::TEXT_MUTED,
                        );
                        if name_resp.clicked() {
                            toggle_drawer(win, &dev_snapshot.unique_id);
                        }
                        let name_hovered = name_resp.hovered();
                        name_resp.on_hover_cursor(egui::CursorIcon::PointingHand);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let (chev_rect, chev_resp) =
                                ui.allocate_exact_size(Vec2::new(18.0, 22.0), Sense::click());
                            draw_chevron(
                                ui.painter(),
                                chev_rect.center(),
                                is_expanded,
                                if chev_resp.hovered() || name_hovered {
                                    theme::TEXT_SECONDARY
                                } else {
                                    theme::TEXT_DIM
                                },
                            );
                            if chev_resp.clicked() {
                                toggle_drawer(win, &dev_snapshot.unique_id);
                            }
                            chev_resp.on_hover_cursor(egui::CursorIcon::PointingHand);

                            let tag_text = if !dev_snapshot.enabled {
                                "Paused"
                            } else {
                                "Offline"
                            };
                            let tag_resp = ui.add(
                                egui::Label::new(
                                    egui::RichText::new(tag_text)
                                        .color(theme::TEXT_DIM)
                                        .font(egui::FontId::proportional(10.5)),
                                )
                                .sense(Sense::click()),
                            );
                            if tag_resp.clicked() {
                                toggle_drawer(win, &dev_snapshot.unique_id);
                            }
                            tag_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                        });
                    });
                });

            if is_expanded {
                let is_hw_online = win.active_devices.contains(&dev_snapshot.unique_id);
                render_device_drawer(win, ui, idx, base_type, is_hw_online, remove_idx);
            }
        });

    frame_resp.response.rect
}

fn toggle_drawer(win: &mut SettingsWindow, unique_id: &str) {
    win.flush_autosave();
    win.editing_device_id = None;
    win.confirming_remove_id = None;
    if win.expanded_device_id.as_deref() == Some(unique_id) {
        win.expanded_device_id = None;
    } else {
        win.expanded_device_id = Some(unique_id.to_string());
    }
}

fn render_device_drawer(
    win: &mut SettingsWindow,
    ui: &mut egui::Ui,
    idx: usize,
    base_type: &'static str,
    is_hw_online: bool,
    remove_idx: &mut Option<usize>,
) {
    // Subtle top divider separating card telemetry from recessed drawer
    let avail = ui.available_width();
    let (sep_rect, _) = ui.allocate_exact_size(Vec2::new(avail, 1.0), Sense::hover());
    ui.painter().line_segment(
        [sep_rect.left_center(), sep_rect.right_center()],
        Stroke::new(1.0, theme::BORDER_CARD),
    );

    egui::Frame::none()
        .fill(theme::BG_DRAWER)
        .rounding(Rounding {
            nw: 0.0,
            ne: 0.0,
            sw: 10.0,
            se: 10.0,
        })
        .inner_margin(egui::Margin::symmetric(12.0, 10.0))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                // Drop hover banner for custom icons
                let hovering_file = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
                if hovering_file {
                    egui::Frame::none()
                        .fill(theme::ACCENT_ONLINE_BG)
                        .stroke(Stroke::new(1.0, theme::ACCENT_ONLINE))
                        .rounding(Rounding::same(6.0))
                        .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("Drop .png or .ico to assign alert tray icon")
                                    .color(theme::ACCENT_ONLINE)
                                    .font(egui::FontId::proportional(11.0))
                                    .strong(),
                            );
                        });
                    ui.add_space(6.0);
                }

                // 1. Nickname Rename + Monitoring Switch
                let unique_id = win.config.devices[idx].unique_id.clone();
                let is_editing = win.editing_device_id.as_deref() == Some(&unique_id);

                ui.horizontal(|ui| {
                    if is_editing {
                        let edit_resp = ui.add(
                            egui::TextEdit::singleline(&mut win.edit_name_buffer)
                                .desired_width(175.0)
                                .font(egui::FontId::proportional(12.0)),
                        );
                        if win.focus_rename {
                            edit_resp.request_focus();
                            win.focus_rename = false;
                        }
                        let enter = edit_resp.has_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let save_click = ui
                            .button(
                                egui::RichText::new("Done")
                                    .color(theme::ACCENT_ONLINE)
                                    .font(egui::FontId::proportional(11.0)),
                            )
                            .clicked();

                        if enter || save_click || edit_resp.lost_focus() {
                            if !ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                let trimmed = win.edit_name_buffer.trim().to_string();
                                if !trimmed.is_empty() && trimmed != win.config.devices[idx].name {
                                    win.config.devices[idx].name = trimmed;
                                    win.mark_dirty_immediate();
                                    win.show_toast("Name saved");
                                }
                            }
                            win.editing_device_id = None;
                        }
                    } else {
                        let rename_btn = ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Rename")
                                        .color(theme::TEXT_SECONDARY)
                                        .font(egui::FontId::proportional(11.0)),
                                )
                                .fill(theme::BG_CARD)
                                .stroke(Stroke::new(1.0, theme::BORDER_CARD))
                                .rounding(Rounding::same(6.0)),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if rename_btn.clicked() {
                            win.editing_device_id = Some(unique_id.clone());
                            win.edit_name_buffer = win.config.devices[idx].name.clone();
                            win.focus_rename = true;
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut enabled = win.config.devices[idx].enabled;
                        if widgets::toggle_switch(ui, &mut enabled).changed() {
                            win.config.devices[idx].enabled = enabled;
                            win.mark_dirty_immediate();
                            win.show_toast(if enabled {
                                "Monitoring enabled"
                            } else {
                                "Monitoring paused"
                            });
                        }
                        ui.label(
                            egui::RichText::new("Monitor")
                                .color(theme::TEXT_MUTED)
                                .font(egui::FontId::proportional(11.0)),
                        );
                    });
                });

                ui.add_space(9.0);

                // 2. Smooth Low-Battery Alert Threshold Slider
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Low-battery alert")
                            .color(theme::TEXT_MUTED)
                            .font(egui::FontId::proportional(11.0)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("{}%", win.config.devices[idx].threshold))
                                .color(theme::ACCENT_ONLINE)
                                .font(egui::FontId::monospace(11.5))
                                .strong(),
                        );
                    });
                });
                ui.add_space(2.0);
                let slider_w = ui.available_width();
                let mut threshold = win.config.devices[idx].threshold;
                if widgets::smooth_threshold_slider(ui, &mut threshold, slider_w).changed() {
                    win.config.devices[idx].threshold = threshold;
                    win.mark_dirty_on_release();
                }

                ui.add_space(9.0);

                // 3. Alert Tray Icon Picker + Preview + Swatches + Browse
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Alert tray icon")
                            .color(theme::TEXT_MUTED)
                            .font(egui::FontId::proportional(11.0)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("Drop .png/.ico or browse")
                                .color(theme::TEXT_DIM)
                                .font(egui::FontId::proportional(9.5)),
                        );
                    });
                });
                ui.add_space(4.0);

                let icon_list = get_icon_list_for(&win.config);
                let current_icon = win.config.devices[idx].low_battery_icon_path.clone();
                let default_preview_key = match base_type {
                    "mouse" | "gamepad" | "buds" | "keyboard" => format!("low_{}.png", base_type),
                    _ => "ok.png".to_string(),
                };
                let preview_key = current_icon
                    .as_deref()
                    .and_then(|p| {
                        std::path::Path::new(p)
                            .file_name()
                            .map(|f| f.to_string_lossy().into_owned())
                    })
                    .unwrap_or_else(|| default_preview_key.clone());

                let mut picked_new_icon: Option<Option<String>> = None;
                let mut trigger_browse = false;

                ui.horizontal(|ui| {
                    // 26×26 crisp icon preview box
                    let (prev_box, _) =
                        ui.allocate_exact_size(Vec2::new(26.0, 26.0), Sense::hover());
                    ui.painter().rect_filled(prev_box, 6.0, theme::BG_CARD);
                    ui.painter()
                        .rect_stroke(prev_box, 6.0, Stroke::new(1.0, theme::BORDER_CARD));
                    if let Some(tex) = win
                        .icon_textures
                        .get(&preview_key)
                        .or_else(|| win.icon_textures.get(&default_preview_key))
                    {
                        let img_rect =
                            Rect::from_center_size(prev_box.center(), Vec2::new(16.0, 16.0));
                        ui.painter().image(
                            tex.id(),
                            img_rect,
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }

                    // Thumbnail ComboBox
                    let selected_label = match current_icon.as_deref() {
                        None => "Default".to_string(),
                        Some(p) => {
                            let fname = std::path::Path::new(p)
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_else(|| p.to_string());
                            format_icon_label(&fname)
                        }
                    };

                    let combo_w = (ui.available_width() - 74.0).max(110.0);
                    egui::ComboBox::from_id_salt(("alert_icon_combo", &unique_id))
                        .selected_text(
                            egui::RichText::new(selected_label)
                                .font(egui::FontId::proportional(11.5))
                                .color(theme::TEXT_PRIMARY),
                        )
                        .width(combo_w)
                        .show_ui(ui, |ui| {
                            let is_default = current_icon.is_none();
                            if ui.selectable_label(is_default, "Default").clicked() {
                                picked_new_icon = Some(None);
                            }
                            for icon_name in &icon_list {
                                let is_sel = current_icon.as_deref() == Some(icon_name.as_str());
                                let row_resp = ui.horizontal(|ui| {
                                    if let Some(tex) = win.icon_textures.get(icon_name) {
                                        ui.add(
                                            egui::Image::new(tex)
                                                .fit_to_exact_size(Vec2::new(14.0, 14.0)),
                                        );
                                    }
                                    ui.selectable_label(is_sel, format_icon_label(icon_name))
                                });
                                if row_resp.inner.clicked() {
                                    picked_new_icon = Some(Some(icon_name.clone()));
                                }
                            }
                        });

                    // Browse… native file picker button
                    let browse_btn = ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Browse…")
                                    .color(theme::TEXT_SECONDARY)
                                    .font(egui::FontId::proportional(11.0)),
                            )
                            .fill(theme::BG_CARD)
                            .stroke(Stroke::new(1.0, theme::BORDER_CARD))
                            .rounding(Rounding::same(6.0)),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text("Import a custom .png or .ico alert icon");

                    if browse_btn.clicked() {
                        trigger_browse = true;
                    }
                });

                // 5 Quick Color Swatches for standard categories
                if matches!(base_type, "mouse" | "gamepad" | "buds" | "keyboard") {
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Tint:")
                                .color(theme::TEXT_DIM)
                                .font(egui::FontId::proportional(10.0)),
                        );
                        let swatches: [(Option<String>, Color32, &str); 5] = [
                            (None, Color32::from_rgb(240, 242, 245), "Default (White)"),
                            (
                                Some(format!("low_{}_red.png", base_type)),
                                Color32::from_rgb(239, 68, 68),
                                "Red alert",
                            ),
                            (
                                Some(format!("low_{}_orange.png", base_type)),
                                Color32::from_rgb(249, 115, 22),
                                "Orange alert",
                            ),
                            (
                                Some(format!("low_{}_yellow.png", base_type)),
                                Color32::from_rgb(250, 204, 21),
                                "Yellow alert",
                            ),
                            (
                                Some(format!("low_{}_blue.png", base_type)),
                                Color32::from_rgb(56, 189, 248),
                                "Blue alert",
                            ),
                        ];

                        for (target_val, swatch_col, tip) in swatches {
                            let is_sel = match (&current_icon, &target_val) {
                                (None, None) => true,
                                (Some(cur), None) => cur == &format!("low_{}.png", base_type),
                                (Some(cur), Some(t)) => cur == t,
                                _ => false,
                            };
                            if widgets::color_swatch_dot(ui, swatch_col, is_sel, tip).clicked() {
                                picked_new_icon = Some(target_val);
                            }
                        }
                    });
                }

                if let Some(choice) = picked_new_icon {
                    win.config.devices[idx].low_battery_icon_path = choice;
                    win.mark_dirty_immediate();
                    win.show_toast("Alert icon saved");
                }

                if trigger_browse {
                    win.suppress_focus_hide = true;
                    let picked = rfd::FileDialog::new()
                        .set_title("Choose Low-Battery Tray Icon")
                        .add_filter("Icon Images", &["png", "ico"])
                        .pick_file();
                    win.suppress_focus_hide = false;

                    if let Some(src_path) = picked {
                        if let Some(icons_dir) = resolve_icons_dir(&win.config) {
                            match import_icon_file(&src_path, &icons_dir) {
                                Ok(imported_name) => {
                                    win.config.devices[idx].low_battery_icon_path =
                                        Some(imported_name.clone());
                                    win.icon_textures.clear();
                                    win.loaded_icons_dir = None;
                                    win.mark_dirty_immediate();
                                    win.show_toast(&format!("Imported {}", imported_name));
                                }
                                Err(err) => {
                                    win.show_toast(&err);
                                }
                            }
                        }
                    }
                }

                // 4. Offline-only 2-step Remove Device
                if !is_hw_online {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let is_confirming =
                                win.confirming_remove_id.as_deref() == Some(&unique_id);
                            if is_confirming {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Confirm Remove")
                                                .color(theme::ACCENT_LOW)
                                                .font(egui::FontId::proportional(10.5))
                                                .strong(),
                                        )
                                        .fill(theme::ACCENT_LOW_BG)
                                        .stroke(Stroke::new(1.0, theme::BORDER_LOW_CARD))
                                        .rounding(Rounding::same(6.0)),
                                    )
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    *remove_idx = Some(idx);
                                }
                                if ui
                                    .button(
                                        egui::RichText::new("Cancel")
                                            .color(theme::TEXT_MUTED)
                                            .font(egui::FontId::proportional(10.5)),
                                    )
                                    .clicked()
                                {
                                    win.confirming_remove_id = None;
                                }
                            } else if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Remove device")
                                            .color(theme::TEXT_DIM)
                                            .font(egui::FontId::proportional(10.5)),
                                    )
                                    .frame(false),
                                )
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                win.confirming_remove_id = Some(unique_id.clone());
                            }
                        });
                    });
                }
            });
        });
}

