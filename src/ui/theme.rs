#![allow(dead_code)]

use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Stroke, Vec2};

// Obsidian Flyout Surface Palette
pub const BG_FLYOUT: Color32 = Color32::from_rgb(20, 21, 26);          // #14151a
pub const BG_HEADER: Color32 = Color32::from_rgb(22, 24, 30);          // #16181e
pub const BORDER_FLYOUT: Color32 = Color32::from_rgb(42, 45, 56);      // #2a2d38

pub const BG_CARD: Color32 = Color32::from_rgb(28, 30, 38);            // #1c1e26
pub const BG_CARD_HOVER: Color32 = Color32::from_rgb(35, 38, 49);      // #232631
pub const BORDER_CARD: Color32 = Color32::from_rgb(40, 43, 54);        // #282b36
pub const BORDER_CARD_HOVER: Color32 = Color32::from_rgb(60, 65, 82);  // #3c4152

pub const BG_DRAWER: Color32 = Color32::from_rgb(16, 17, 22);          // #101116
pub const BG_SUBTLE: Color32 = Color32::from_rgb(23, 25, 32);          // #171920
pub const BG_TRACK: Color32 = Color32::from_rgb(38, 41, 54);           // #262936
pub const BG_ACTIVE_PILL: Color32 = Color32::from_rgb(46, 50, 66);     // #2e3242

pub const BG_LOW_CARD: Color32 = Color32::from_rgb(36, 21, 24);        // #241518
pub const BG_LOW_CARD_HOVER: Color32 = Color32::from_rgb(44, 25, 29);  // #2c191d
pub const BORDER_LOW_CARD: Color32 = Color32::from_rgb(92, 35, 42);    // #5c232a

// Semantic Accents
pub const ACCENT_ONLINE: Color32 = Color32::from_rgb(52, 211, 153);    // #34d399 Emerald
pub const ACCENT_ONLINE_BG: Color32 = Color32::from_rgb(19, 44, 36);
pub const ACCENT_CHARGING: Color32 = Color32::from_rgb(251, 191, 36);  // #fbbf24 Amber
pub const ACCENT_CHARGING_BG: Color32 = Color32::from_rgb(50, 38, 17);
pub const ACCENT_LOW: Color32 = Color32::from_rgb(248, 113, 113);      // #f87171 Coral
pub const ACCENT_LOW_BG: Color32 = Color32::from_rgb(56, 22, 26);

// Typography Hierarchy
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(244, 244, 247);    // #f4f4f7
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(209, 213, 219);  // #d1d5db
pub const TEXT_MUTED: Color32 = Color32::from_rgb(148, 163, 184);      // #94a3b8
pub const TEXT_DIM: Color32 = Color32::from_rgb(100, 116, 139);        // #64748b

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Mouse,
    Gamepad,
    Buds,
    Keyboard,
    Peripheral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionIcon {
    Refresh,
    Pin { pinned: bool },
    Gear,
    Back,
    Close,
    Folder,
    Grip,
}

/// Loads Windows 11 Segoe UI Variable (or Segoe UI) + Cascadia Mono (or Consolas)
pub fn configure_flyout_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let mut changed = false;

    for (name, path) in [
        ("win_sans", "C:\\Windows\\Fonts\\SegUIVar.ttf"),
        ("win_sans", "C:\\Windows\\Fonts\\segoeui.ttf"),
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(name.to_owned(), egui::FontData::from_owned(bytes));
            if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                fam.insert(0, name.to_owned());
                changed = true;
            }
            break;
        }
    }

    if let Ok(sym_bytes) = std::fs::read("C:\\Windows\\Fonts\\seguisym.ttf") {
        fonts.font_data.insert("segoe_sym".to_owned(), egui::FontData::from_owned(sym_bytes));
        if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            fam.push("segoe_sym".to_owned());
            changed = true;
        }
    }

    for (name, path) in [
        ("win_mono", "C:\\Windows\\Fonts\\CascadiaMono.ttf"),
        ("win_mono", "C:\\Windows\\Fonts\\consola.ttf"),
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(name.to_owned(), egui::FontData::from_owned(bytes));
            if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                fam.insert(0, name.to_owned());
                changed = true;
            }
            break;
        }
    }

    if changed {
        ctx.set_fonts(fonts);
    }
}

/// Applies clean dark Obsidian visuals to egui popups, inputs, and scrollbars
pub fn apply_flyout_visuals(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG_FLYOUT;
    visuals.window_fill = BG_CARD;
    visuals.window_stroke = Stroke::new(1.0, BORDER_CARD_HOVER);
    visuals.window_rounding = Rounding::same(10.0);
    visuals.popup_shadow = egui::epaint::Shadow {
        offset: Vec2::new(0.0, 4.0),
        blur: 12.0,
        spread: 0.0,
        color: Color32::from_black_alpha(120),
    };

    visuals.extreme_bg_color = BG_DRAWER;
    visuals.widgets.noninteractive.bg_fill = BG_CARD;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_CARD);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_MUTED);
    visuals.widgets.noninteractive.rounding = Rounding::same(8.0);

    visuals.widgets.inactive.bg_fill = BG_SUBTLE;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_CARD);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_SECONDARY);
    visuals.widgets.inactive.rounding = Rounding::same(7.0);

    visuals.widgets.hovered.bg_fill = BG_CARD_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, BORDER_CARD_HOVER);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.hovered.rounding = Rounding::same(7.0);

    visuals.widgets.active.bg_fill = BG_ACTIVE_PILL;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT_ONLINE);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.active.rounding = Rounding::same(7.0);

    visuals.selection.bg_fill = ACCENT_ONLINE_BG;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT_ONLINE);

    let mut style = (*ctx.style()).clone();
    style.spacing.scroll = egui::style::ScrollStyle::floating();
    style.spacing.scroll.bar_width = 5.0;
    style.visuals = visuals;
    ctx.set_style(style);
}

/// Draws a crisp vector peripheral category icon centered inside `rect`
pub fn draw_device_icon(painter: &egui::Painter, rect: Rect, kind: DeviceKind, color: Color32) {
    let c = rect.center();
    let s = (rect.width().min(rect.height()) / 20.0).clamp(0.6, 2.0);
    let stroke = Stroke::new(1.5 * s, color);

    match kind {
        DeviceKind::Mouse => {
            let body = Rect::from_center_size(c, Vec2::new(10.5 * s, 15.5 * s));
            painter.rect_stroke(body, Rounding::same(5.2 * s), stroke);
            painter.line_segment(
                [
                    Pos2::new(c.x, body.top() + 3.2 * s),
                    Pos2::new(c.x, body.top() + 6.5 * s),
                ],
                Stroke::new(1.5 * s, color),
            );
        }
        DeviceKind::Gamepad => {
            let body = Rect::from_center_size(c, Vec2::new(16.5 * s, 10.5 * s));
            painter.rect_stroke(body, Rounding::same(4.5 * s), stroke);
            // D-pad cross on left
            let dx = c.x - 3.8 * s;
            painter.line_segment([Pos2::new(dx - 2.0 * s, c.y), Pos2::new(dx + 2.0 * s, c.y)], Stroke::new(1.3 * s, color));
            painter.line_segment([Pos2::new(dx, c.y - 2.0 * s), Pos2::new(dx, c.y + 2.0 * s)], Stroke::new(1.3 * s, color));
            // Action buttons on right
            painter.circle_filled(Pos2::new(c.x + 3.2 * s, c.y + 1.0 * s), 1.0 * s, color);
            painter.circle_filled(Pos2::new(c.x + 5.0 * s, c.y - 1.0 * s), 1.0 * s, color);
        }
        DeviceKind::Buds => {
            // Headband arc + twin ear cushions
            let r = 6.2 * s;
            let mut arc_pts = Vec::with_capacity(13);
            for i in 0..=12 {
                let t = std::f32::consts::PI + (i as f32 / 12.0) * std::f32::consts::PI;
                arc_pts.push(Pos2::new(c.x + r * t.cos(), c.y + 1.0 * s + r * t.sin()));
            }
            painter.add(egui::Shape::line(arc_pts, stroke));
            let left_cup = Rect::from_center_size(Pos2::new(c.x - 5.6 * s, c.y + 2.8 * s), Vec2::new(2.8 * s, 5.2 * s));
            let right_cup = Rect::from_center_size(Pos2::new(c.x + 5.6 * s, c.y + 2.8 * s), Vec2::new(2.8 * s, 5.2 * s));
            painter.rect_filled(left_cup, Rounding::same(1.4 * s), color);
            painter.rect_filled(right_cup, Rounding::same(1.4 * s), color);
        }
        DeviceKind::Keyboard => {
            let body = Rect::from_center_size(c, Vec2::new(16.5 * s, 11.0 * s));
            painter.rect_stroke(body, Rounding::same(2.5 * s), stroke);
            for col in -2..=2 {
                let x = c.x + (col as f32) * 2.6 * s;
                painter.circle_filled(Pos2::new(x, c.y - 2.0 * s), 0.75 * s, color);
            }
            painter.line_segment(
                [Pos2::new(c.x - 4.0 * s, c.y + 2.3 * s), Pos2::new(c.x + 4.0 * s, c.y + 2.3 * s)],
                Stroke::new(1.4 * s, color),
            );
        }
        DeviceKind::Peripheral => {
            let body = Rect::from_center_size(Pos2::new(c.x, c.y + 1.5 * s), Vec2::new(10.0 * s, 11.0 * s));
            painter.rect_stroke(body, Rounding::same(2.5 * s), stroke);
            painter.line_segment(
                [Pos2::new(c.x - 2.2 * s, body.top() - 3.0 * s), Pos2::new(c.x - 2.2 * s, body.top())],
                stroke,
            );
            painter.line_segment(
                [Pos2::new(c.x + 2.2 * s, body.top() - 3.0 * s), Pos2::new(c.x + 2.2 * s, body.top())],
                stroke,
            );
        }
    }
}

/// Draws crisp vector UI action icons centered inside `rect`
pub fn draw_action_icon(
    painter: &egui::Painter,
    rect: Rect,
    icon: ActionIcon,
    color: Color32,
    spin_angle: f32,
) {
    let c = rect.center();
    let s = (rect.width().min(rect.height()) / 16.0).clamp(0.6, 2.0);
    let stroke = Stroke::new(1.5 * s, color);

    let rot = |dx: f32, dy: f32, ang: f32| -> Pos2 {
        let (sin, cos) = ang.sin_cos();
        Pos2::new(c.x + dx * cos - dy * sin, c.y + dx * sin + dy * cos)
    };

    match icon {
        ActionIcon::Refresh => {
            let r = 5.2 * s;
            for base in [0.25_f32, std::f32::consts::PI + 0.25] {
                let mut pts = Vec::with_capacity(9);
                for step in 0..=8 {
                    let a = spin_angle + base + (step as f32 / 8.0) * 2.2;
                    pts.push(Pos2::new(c.x + r * a.cos(), c.y + r * a.sin()));
                }
                if let Some(&tip) = pts.last() {
                    let end_a = spin_angle + base + 2.2;
                    let tx = -end_a.sin();
                    let ty = end_a.cos();
                    let nx = -end_a.cos();
                    let ny = -end_a.sin();
                    painter.line_segment(
                        [tip, Pos2::new(tip.x - (tx * 2.3 - nx * 1.6) * s, tip.y - (ty * 2.3 - ny * 1.6) * s)],
                        stroke,
                    );
                    painter.line_segment(
                        [tip, Pos2::new(tip.x - (tx * 2.3 + nx * 1.6) * s, tip.y - (ty * 2.3 + ny * 1.6) * s)],
                        stroke,
                    );
                }
                painter.add(egui::Shape::line(pts, stroke));
            }
        }
        ActionIcon::Pin { pinned } => {
            let angle = if pinned { 0.0 } else { 0.55 };
            // Top bar, body, crossbar, needle
            painter.line_segment([rot(-2.6 * s, -5.2 * s, angle), rot(2.6 * s, -5.2 * s, angle)], stroke);
            painter.line_segment([rot(-1.8 * s, -5.2 * s, angle), rot(-2.8 * s, 0.6 * s, angle)], stroke);
            painter.line_segment([rot(1.8 * s, -5.2 * s, angle), rot(2.8 * s, 0.6 * s, angle)], stroke);
            painter.line_segment([rot(-4.0 * s, 0.6 * s, angle), rot(4.0 * s, 0.6 * s, angle)], stroke);
            painter.line_segment([rot(0.0, 0.6 * s, angle), rot(0.0, 5.8 * s, angle)], stroke);
            if pinned {
                let fill_pts = vec![
                    rot(-1.8 * s, -5.0 * s, angle),
                    rot(1.8 * s, -5.0 * s, angle),
                    rot(2.6 * s, 0.5 * s, angle),
                    rot(-2.6 * s, 0.5 * s, angle),
                ];
                painter.add(egui::Shape::convex_polygon(fill_pts, color, Stroke::NONE));
            }
        }
        ActionIcon::Gear => {
            let inner_r = 2.2 * s;
            let outer_r = 4.6 * s;
            painter.circle_stroke(c, inner_r, stroke);
            painter.circle_stroke(c, outer_r, Stroke::new(1.3 * s, color));
            for i in 0..6 {
                let a = (i as f32) * std::f32::consts::TAU / 6.0;
                let p1 = Pos2::new(c.x + (outer_r - 0.3 * s) * a.cos(), c.y + (outer_r - 0.3 * s) * a.sin());
                let p2 = Pos2::new(c.x + (outer_r + 1.6 * s) * a.cos(), c.y + (outer_r + 1.6 * s) * a.sin());
                painter.line_segment([p1, p2], Stroke::new(1.6 * s, color));
            }
        }
        ActionIcon::Back => {
            painter.line_segment([Pos2::new(c.x + 4.8 * s, c.y), Pos2::new(c.x - 4.2 * s, c.y)], stroke);
            painter.line_segment([Pos2::new(c.x - 4.5 * s, c.y), Pos2::new(c.x - 0.8 * s, c.y - 3.8 * s)], stroke);
            painter.line_segment([Pos2::new(c.x - 4.5 * s, c.y), Pos2::new(c.x - 0.8 * s, c.y + 3.8 * s)], stroke);
        }
        ActionIcon::Close => {
            let d = 4.0 * s;
            painter.line_segment([Pos2::new(c.x - d, c.y - d), Pos2::new(c.x + d, c.y + d)], stroke);
            painter.line_segment([Pos2::new(c.x + d, c.y - d), Pos2::new(c.x - d, c.y + d)], stroke);
        }
        ActionIcon::Folder => {
            let pts = vec![
                Pos2::new(c.x - 5.5 * s, c.y - 4.2 * s),
                Pos2::new(c.x - 1.5 * s, c.y - 4.2 * s),
                Pos2::new(c.x - 0.2 * s, c.y - 2.6 * s),
                Pos2::new(c.x + 5.5 * s, c.y - 2.6 * s),
                Pos2::new(c.x + 5.5 * s, c.y + 4.4 * s),
                Pos2::new(c.x - 5.5 * s, c.y + 4.4 * s),
            ];
            painter.add(egui::Shape::closed_line(pts, stroke));
        }
        ActionIcon::Grip => {
            for row in [-1, 0, 1] {
                for col in [-1, 1] {
                    painter.circle_filled(
                        Pos2::new(c.x + (col as f32) * 1.9 * s, c.y + (row as f32) * 3.2 * s),
                        1.05 * s,
                        color,
                    );
                }
            }
        }
    }
}

/// Draws a compact vector lightning bolt centered at `center`
pub fn draw_bolt(painter: &egui::Painter, center: Pos2, scale: f32, color: Color32) {
    let pts = vec![
        Pos2::new(center.x + 0.8 * scale, center.y - 5.0 * scale),
        Pos2::new(center.x - 3.0 * scale, center.y + 0.4 * scale),
        Pos2::new(center.x - 0.3 * scale, center.y + 0.4 * scale),
        Pos2::new(center.x - 0.8 * scale, center.y + 5.0 * scale),
        Pos2::new(center.x + 3.0 * scale, center.y - 0.4 * scale),
        Pos2::new(center.x + 0.3 * scale, center.y - 0.4 * scale),
    ];
    painter.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
}
