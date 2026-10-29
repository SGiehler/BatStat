use crate::ui::theme::{self, ActionIcon};
use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Sense, Stroke, Vec2};

/// Animated sleek toggle switch (`34 × 18 px`)
pub fn toggle_switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired = Vec2::new(34.0, 18.0);
    let (rect, mut response) = ui.allocate_exact_size(desired, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        let t = ui.ctx().animate_bool(response.id, *on);
        let track = if *on { theme::ACCENT_ONLINE } else { theme::BG_TRACK };
        let radius = rect.height() * 0.5;
        ui.painter().rect_filled(rect, radius, track);

        if response.hovered() {
            ui.painter().rect_stroke(
                rect,
                radius,
                Stroke::new(1.0, theme::BORDER_CARD_HOVER),
            );
        }

        let knob_r = radius - 2.5;
        let x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), t);
        ui.painter()
            .circle_filled(Pos2::new(x, rect.center().y), knob_r, Color32::WHITE);
    }

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Minimal rounded pill battery gauge (`6 px` or `4 px` height, no embedded text)
pub fn pill_battery_bar(
    ui: &mut egui::Ui,
    percentage: u8,
    charging: bool,
    threshold: u8,
    width: f32,
    height: f32,
) -> egui::Response {
    let w = width.max(24.0);
    let h = height.max(3.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());

    if ui.is_rect_visible(rect) {
        let r = Rounding::same(h * 0.5);
        ui.painter().rect_filled(rect, r, theme::BG_TRACK);

        let frac = (percentage.min(100) as f32) / 100.0;
        let fill_w = (rect.width() * frac).max(if percentage > 0 { h } else { 0.0 }).min(rect.width());

        let fill_color = if charging {
            theme::ACCENT_CHARGING
        } else if percentage <= threshold {
            theme::ACCENT_LOW
        } else {
            theme::ACCENT_ONLINE
        };

        if fill_w > 0.5 {
            let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, rect.height()));
            ui.painter().rect_filled(fill_rect, r, fill_color);
        }
    }

    response
}

/// Custom smooth threshold slider (`5%..=95%`, `1%` resolution) with `4 px` track & glowing thumb
pub fn smooth_threshold_slider(
    ui: &mut egui::Ui,
    value: &mut u8,
    width: f32,
) -> egui::Response {
    let w = width.max(80.0);
    let (rect, mut response) = ui.allocate_exact_size(Vec2::new(w, 20.0), Sense::click_and_drag());
    let pad = 7.0;
    let usable_left = rect.left() + pad;
    let usable_right = (rect.right() - pad).max(usable_left + 1.0);

    if let Some(pointer_pos) = response.interact_pointer_pos() {
        if response.clicked() || response.dragged() {
            let t = ((pointer_pos.x - usable_left) / (usable_right - usable_left)).clamp(0.0, 1.0);
            let new_val = (5.0 + t * 90.0).round() as u8;
            if new_val != *value {
                *value = new_val.clamp(5, 95);
                response.mark_changed();
            }
        }
    }

    *value = (*value).clamp(5, 95);

    if ui.is_rect_visible(rect) {
        let t = (*value - 5) as f32 / 90.0;
        let thumb_x = egui::lerp(usable_left..=usable_right, t);
        let cy = rect.center().y;

        let track_rect = Rect::from_min_max(
            Pos2::new(usable_left, cy - 2.0),
            Pos2::new(usable_right, cy + 2.0),
        );
        ui.painter().rect_filled(track_rect, 2.0, theme::BG_TRACK);

        let active_rect = Rect::from_min_max(
            Pos2::new(usable_left, cy - 2.0),
            Pos2::new(thumb_x, cy + 2.0),
        );
        ui.painter()
            .rect_filled(active_rect, 2.0, theme::ACCENT_ONLINE);

        let thumb_center = Pos2::new(thumb_x, cy);
        if response.hovered() || response.dragged() {
            ui.painter().circle_filled(
                thumb_center,
                8.5,
                theme::ACCENT_ONLINE.linear_multiply(0.22),
            );
        }
        ui.painter().circle_filled(thumb_center, 6.0, Color32::WHITE);
    }

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Compact 3-way segmented poll selector (`15s` · `30s` · `60s`), returns `true` when changed
pub fn segmented_poll_selector(ui: &mut egui::Ui, secs: &mut u64) -> bool {
    let presets: [(u64, &str); 3] = [(15, "15s"), (30, "30s"), (60, "60s")];
    let mut changed = false;

    let total_size = Vec2::new(126.0, 26.0);
    let (outer, _) = ui.allocate_exact_size(total_size, Sense::hover());

    if ui.is_rect_visible(outer) {
        ui.painter().rect_filled(outer, 7.0, theme::BG_DRAWER);
        ui.painter()
            .rect_stroke(outer, 7.0, Stroke::new(1.0, theme::BORDER_CARD));
    }

    let seg_w = (total_size.x - 6.0) / 3.0;
    for (idx, (preset_val, label)) in presets.iter().enumerate() {
        let seg_rect = Rect::from_min_size(
            outer.min + Vec2::new(3.0 + idx as f32 * seg_w, 3.0),
            Vec2::new(seg_w, total_size.y - 6.0),
        );
        let resp = ui
            .interact(
                seg_rect,
                ui.id().with(("poll_seg", idx)),
                Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand);

        if resp.clicked() && *secs != *preset_val {
            *secs = *preset_val;
            changed = true;
        }

        let is_sel = *secs == *preset_val;
        if ui.is_rect_visible(seg_rect) {
            if is_sel {
                ui.painter().rect_filled(seg_rect, 5.0, theme::BG_ACTIVE_PILL);
            } else if resp.hovered() {
                ui.painter().rect_filled(seg_rect, 5.0, theme::BG_CARD_HOVER);
            }
            ui.painter().text(
                seg_rect.center(),
                egui::Align2::CENTER_CENTER,
                *label,
                egui::FontId::proportional(11.5),
                if is_sel {
                    theme::TEXT_PRIMARY
                } else {
                    theme::TEXT_MUTED
                },
            );
        }
    }

    changed
}

/// Header action button (`28 × 28 px`) with vector glyph, active state, and optional update badge dot
pub fn header_icon_button(
    ui: &mut egui::Ui,
    icon: ActionIcon,
    active: bool,
    badge_dot: bool,
    spin_angle: f32,
    tooltip: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(28.0, 28.0), Sense::click());

    if ui.is_rect_visible(rect) {
        if active {
            ui.painter()
                .rect_filled(rect, 7.0, theme::ACCENT_ONLINE_BG);
            ui.painter().rect_stroke(
                rect,
                7.0,
                Stroke::new(1.0, theme::ACCENT_ONLINE.linear_multiply(0.45)),
            );
        } else if response.hovered() {
            ui.painter().rect_filled(rect, 7.0, theme::BG_CARD_HOVER);
        }

        let icon_color = if active {
            theme::ACCENT_ONLINE
        } else if response.hovered() {
            theme::TEXT_PRIMARY
        } else {
            theme::TEXT_MUTED
        };

        let glyph_rect = Rect::from_center_size(rect.center(), Vec2::new(15.0, 15.0));
        theme::draw_action_icon(ui.painter(), glyph_rect, icon, icon_color, spin_angle);

        if badge_dot {
            ui.painter().circle_filled(
                Pos2::new(rect.right() - 5.0, rect.top() + 5.0),
                3.0,
                theme::ACCENT_ONLINE,
            );
        }
    }

    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(tooltip)
}

/// Circular color swatch dot (`20 × 20 px`) for low-battery icon quick tinting
pub fn color_swatch_dot(
    ui: &mut egui::Ui,
    color: Color32,
    selected: bool,
    tooltip: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(20.0, 20.0), Sense::click());

    if ui.is_rect_visible(rect) {
        let c = rect.center();
        if selected {
            ui.painter()
                .circle_stroke(c, 8.5, Stroke::new(1.5, theme::TEXT_PRIMARY));
        } else if response.hovered() {
            ui.painter()
                .circle_stroke(c, 8.5, Stroke::new(1.0, theme::BORDER_CARD_HOVER));
        }
        ui.painter().circle_filled(c, 5.5, color);
    }

    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(tooltip)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::{draw_device_icon, ActionIcon, DeviceKind};

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
                let _ = header_icon_button(ui, ActionIcon::Pin { pinned: true }, true, true, 0.0, "Pin");
                let _ = color_swatch_dot(ui, egui::Color32::RED, true, "Red");

                let (r, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
                for kind in [
                    DeviceKind::Mouse,
                    DeviceKind::Gamepad,
                    DeviceKind::Buds,
                    DeviceKind::Keyboard,
                    DeviceKind::Peripheral,
                ] {
                    draw_device_icon(ui.painter(), r, kind, egui::Color32::WHITE);
                }
            });
        });

        assert!((5..=95).contains(&threshold));
    }
}
