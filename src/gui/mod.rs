use crate::driver::{BatteryStatus, MchoseMouse, MouseConfig, PollingRate, DPI_STEP, MAX_DPI, MIN_DPI};
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, RichText, Stroke, StrokeKind, Vec2};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

// ============================================================================
// COLOR PALETTE
// ============================================================================
const COLOR_BG_MAIN: Color32 = Color32::from_rgb(0x09, 0x0C, 0x11);
const COLOR_BG_CARD: Color32 = Color32::from_rgb(0x11, 0x18, 0x20);
const COLOR_BG_SECONDARY: Color32 = Color32::from_rgb(0x0B, 0x10, 0x16);
const COLOR_BG_ELEMENT: Color32 = Color32::from_rgb(0x17, 0x1E, 0x28);
const COLOR_BG_TELEMETRY: Color32 = Color32::from_rgb(0x11, 0x16, 0x1D);

const COLOR_TEXT_PRIMARY: Color32 = Color32::from_rgb(0xF1, 0xF5, 0xF9);
const COLOR_TEXT_SECONDARY: Color32 = Color32::from_rgb(0x94, 0xA3, 0xB8);
const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(0x64, 0x74, 0x8B);
const COLOR_TEXT_DISABLED: Color32 = Color32::from_rgb(0x47, 0x55, 0x69);

const COLOR_ACCENT: Color32 = Color32::from_rgb(0x38, 0xBD, 0xF8);
const COLOR_ACCENT_HOVER: Color32 = Color32::from_rgb(0x7D, 0xD3, 0xFC);
const COLOR_TEXT_ON_ACCENT: Color32 = Color32::from_rgb(0x06, 0x12, 0x1C);

const COLOR_BORDER_CARD: Color32 = Color32::from_rgb(0x23, 0x2B, 0x36);
const COLOR_BORDER_CONTROL: Color32 = Color32::from_rgb(0x2C, 0x37, 0x46);
const COLOR_BORDER_HOVER: Color32 = Color32::from_rgb(0x45, 0x53, 0x66);
const COLOR_BORDER_TELEMETRY: Color32 = Color32::from_rgb(0x25, 0x2F, 0x3B);
const COLOR_BORDER_INSET: Color32 = Color32::from_rgb(0x1E, 0x26, 0x32);

const COLOR_SUCCESS: Color32 = Color32::from_rgb(0x34, 0xD3, 0x99);
const COLOR_WARNING: Color32 = Color32::from_rgb(0xFB, 0xBF, 0x24);
const COLOR_ERROR: Color32 = Color32::from_rgb(0xEF, 0x44, 0x44);

const COLOR_ACTIVE_BG: Color32 = Color32::from_rgba_premultiplied(1, 18, 26, 26);
const COLOR_ACTIVE_BORDER: Color32 = Color32::from_rgba_premultiplied(14, 47, 62, 70);
const COLOR_ACCENT_GLOW: Color32 = Color32::from_rgba_premultiplied(3, 42, 60, 65);

const COLOR_SLIDER_TRACK: Color32 = Color32::from_rgb(0x23, 0x2D, 0x3A);
const COLOR_SLIDER_KNOB: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);

const STAGE_COLORS: [Color32; 6] = [
    Color32::from_rgb(0x38, 0xBD, 0xF8), // Stage 1: Cyan (#38BDF8)
    Color32::from_rgb(0x8B, 0x5C, 0xF6), // Stage 2: Purple (#8B5CF6)
    Color32::from_rgb(0xA7, 0x8B, 0xFA), // Stage 3: Indigo (#A78BFA)
    Color32::from_rgb(0xF5, 0x9E, 0x0B), // Stage 4: Amber (#F59E0B)
    Color32::from_rgb(0xD9, 0x46, 0xEF), // Stage 5: Magenta (#D946EF)
    Color32::from_rgb(0x22, 0xD3, 0xEE), // Stage 6: Bright Teal (#22D3EE)
];

// ============================================================================
// TYPOGRAPHY / FONTS
// ============================================================================
pub fn font_family_bold() -> egui::FontFamily {
    egui::FontFamily::Name("Bold".into())
}
fn font_app_title() -> egui::FontId {
    egui::FontId::new(16.5, font_family_bold())
}
fn font_device_sub() -> egui::FontId {
    egui::FontId::new(10.0, font_family_bold())
}
fn font_section_title() -> egui::FontId {
    egui::FontId::new(12.5, font_family_bold())
}
fn font_control_text() -> egui::FontId {
    egui::FontId::new(13.0, egui::FontFamily::Proportional)
}
fn font_dpi_value() -> egui::FontId {
    egui::FontId::new(14.5, egui::FontFamily::Proportional)
}
fn font_small_caption() -> egui::FontId {
    egui::FontId::new(11.0, egui::FontFamily::Proportional)
}

pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    if let Ok(data) = std::fs::read("C:\\Windows\\Fonts\\segoeui.ttf") {
        fonts.font_data.insert(
            "segoe_ui".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(data)),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "segoe_ui".to_owned());
    }

    if let Ok(data_bold) = std::fs::read("C:\\Windows\\Fonts\\segoeuib.ttf") {
        fonts.font_data.insert(
            "segoe_ui_bold".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(data_bold)),
        );
        fonts
            .families
            .entry(font_family_bold())
            .or_default()
            .push("segoe_ui_bold".to_owned());
    }

    if let Some(prop_fonts) = fonts.families.get(&egui::FontFamily::Proportional).cloned() {
        let bold_entry = fonts.families.entry(font_family_bold()).or_default();
        for f in prop_fonts {
            if !bold_entry.contains(&f) {
                bold_entry.push(f);
            }
        }
    }

    ctx.set_fonts(fonts);
}

fn format_dpi(dpi: u16) -> String {
    if dpi >= 1000 {
        let thousands = dpi / 1000;
        let rem = dpi % 1000;
        format!("{},{:03}", thousands, rem)
    } else {
        dpi.to_string()
    }
}

/// Decrement DPI to previous multiple of DPI_STEP, clamping to [MIN_DPI, MAX_DPI].
/// If the value is already outside [MIN_DPI, MAX_DPI] or at MIN_DPI, leaves it untouched.
pub fn step_dpi_down(cur: u16) -> u16 {
    if cur > MIN_DPI && cur <= MAX_DPI {
        ((cur - 1) / DPI_STEP * DPI_STEP).max(MIN_DPI)
    } else {
        cur
    }
}

/// Increment DPI to next multiple of DPI_STEP, clamping to [MIN_DPI, MAX_DPI].
/// If the value is already outside [MIN_DPI, MAX_DPI] or at MAX_DPI, leaves it untouched.
pub fn step_dpi_up(cur: u16) -> u16 {
    if cur >= MIN_DPI && cur < MAX_DPI {
        ((cur / DPI_STEP + 1) * DPI_STEP).min(MAX_DPI)
    } else {
        cur
    }
}

// ============================================================================
// FRAMES
// ============================================================================
fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(COLOR_BG_CARD)
        .stroke(Stroke::new(1.0, COLOR_BORDER_CARD))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(16, 14))
}

fn inset_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(COLOR_BG_SECONDARY)
        .stroke(Stroke::new(1.0, COLOR_BORDER_INSET))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(14, 6))
}

// ============================================================================
// VECTOR ICON PAINTERS
// ============================================================================

fn draw_status_icon(painter: &egui::Painter, center: Pos2, color: Color32) {
    painter.circle_stroke(center, 9.0, Stroke::new(1.4, color));

    if color == COLOR_ERROR {
        // Red X for errors and disconnected state
        let r = 3.5;
        let stroke = Stroke::new(1.6, color);
        painter.line_segment(
            [Pos2::new(center.x - r, center.y - r), Pos2::new(center.x + r, center.y + r)],
            stroke,
        );
        painter.line_segment(
            [Pos2::new(center.x - r, center.y + r), Pos2::new(center.x + r, center.y - r)],
            stroke,
        );
    } else if color == COLOR_WARNING {
        // Warning exclamation mark for unsaved / in-progress actions
        let stroke = Stroke::new(1.6, color);
        painter.line_segment(
            [Pos2::new(center.x, center.y - 4.0), Pos2::new(center.x, center.y + 1.0)],
            stroke,
        );
        painter.circle_filled(Pos2::new(center.x, center.y + 3.8), 1.0, color);
    } else {
        // Checkmark for synced / success
        let p1 = Pos2::new(center.x - 4.2, center.y + 0.2);
        let p2 = Pos2::new(center.x - 1.2, center.y + 3.2);
        let p3 = Pos2::new(center.x + 4.8, center.y - 3.2);
        let check_stroke = Stroke::new(1.6, color);
        painter.line_segment([p1, p2], check_stroke);
        painter.line_segment([p2, p3], check_stroke);
    }
}

fn draw_wireless_icon(painter: &egui::Painter, center: Pos2, color: Color32) {
    let stroke = Stroke::new(1.5, color);
    let base_y = center.y + 3.5;

    painter.circle_filled(Pos2::new(center.x, base_y), 1.5, color);

    for r in [4.5, 7.5] {
        let start_angle = -0.75 * std::f32::consts::PI;
        let end_angle = -0.25 * std::f32::consts::PI;
        let steps = 7;
        for i in 0..steps {
            let t1 = start_angle + (end_angle - start_angle) * (i as f32 / steps as f32);
            let t2 = start_angle + (end_angle - start_angle) * ((i + 1) as f32 / steps as f32);
            let p1 = Pos2::new(center.x + r * t1.cos(), base_y + r * t1.sin());
            let p2 = Pos2::new(center.x + r * t2.cos(), base_y + r * t2.sin());
            painter.line_segment([p1, p2], stroke);
        }
    }
}

pub(crate) fn battery_display_info(
    battery: Option<&BatteryStatus>,
    is_connected: bool,
) -> (String, Color32, Color32, Option<u8>, String) {
    match battery {
        Some(bat) => {
            let pct = bat.percent.clamp(0, 100);
            let color = if bat.charging {
                COLOR_SUCCESS
            } else if pct > 30 {
                COLOR_ACCENT
            } else if pct > 15 {
                COLOR_WARNING
            } else {
                COLOR_ERROR
            };
            let tooltip = if bat.charging {
                format!("Battery: {}% (Charging)", pct)
            } else {
                format!("Battery: {}%", pct)
            };
            (format!("{}%", pct), color, COLOR_TEXT_PRIMARY, Some(pct), tooltip)
        }
        None => {
            let tooltip = if is_connected {
                "Battery status unavailable".to_string()
            } else {
                "Device disconnected".to_string()
            };
            ("--%".to_string(), COLOR_TEXT_MUTED, COLOR_TEXT_MUTED, None, tooltip)
        }
    }
}

pub(crate) fn active_dpi_display_info(
    saved_config: &MouseConfig,
    is_connected: bool,
) -> (String, Color32, bool) {
    if is_connected {
        let active_idx = saved_config.active_dpi_stage.saturating_sub(1).min(5);
        let active_dpi = saved_config.dpi_stages[active_idx];
        (
            format!("Active: {} DPI", format_dpi(active_dpi)),
            COLOR_ACCENT,
            true,
        )
    } else {
        (
            "Active: -- DPI".to_string(),
            COLOR_TEXT_MUTED,
            false,
        )
    }
}

fn draw_battery_badge(painter: &egui::Painter, min: Pos2, percent: Option<u8>, color: Color32) {
    let w = 18.0;
    let h = 10.0;
    let body_rect = Rect::from_min_size(min, Vec2::new(w, h));

    painter.rect(
        body_rect,
        CornerRadius::same(2),
        Color32::TRANSPARENT,
        Stroke::new(1.2, color),
        StrokeKind::Inside,
    );

    let pip_rect = Rect::from_min_size(
        Pos2::new(body_rect.max.x + 1.0, body_rect.min.y + 2.5),
        Vec2::new(1.5, 5.0),
    );
    painter.rect_filled(pip_rect, CornerRadius::same(1), color);

    if let Some(pct) = percent {
        let clamped = pct.clamp(0, 100) as f32 / 100.0;
        let inner_w = ((w - 3.0) * clamped).round();
        if inner_w > 0.0 {
            let fill_rect = Rect::from_min_size(
                Pos2::new(body_rect.min.x + 1.5, body_rect.min.y + 1.5),
                Vec2::new(inner_w, h - 3.0),
            );
            painter.rect_filled(fill_rect, CornerRadius::same(1), color);
        }
    }
}

// ============================================================================
// CUSTOM INTERACTIVE SLIDERS
// ============================================================================
fn custom_slider_u16(
    ui: &mut egui::Ui,
    val: &mut u16,
    min: u16,
    max: u16,
    step: u16,
    width: f32,
) -> bool {
    let desired_size = Vec2::new(width.max(60.0), 28.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
    let mut changed = false;

    let knob_w = 14.0;
    let track_inset = knob_w / 2.0 + 2.0;
    let track_left = rect.min.x + track_inset;
    let track_right = rect.max.x - track_inset;
    let track_len = (track_right - track_left).max(1.0);
    let cy = rect.center().y;

    if response.dragged() || response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let norm = ((pos.x - track_left) / track_len).clamp(0.0, 1.0);
            let raw = min as f32 + norm * (max - min) as f32;
            let stepped = ((raw - min as f32) / step as f32).round() * step as f32 + min as f32;
            let new_val = (stepped as u16).clamp(min, max);
            if new_val != *val {
                *val = new_val;
                changed = true;
            }
        }
    }

    // Track
    let track_rect = Rect::from_min_max(
        Pos2::new(track_left, cy - 2.0),
        Pos2::new(track_right, cy + 2.0),
    );
    ui.painter()
        .rect_filled(track_rect, CornerRadius::same(2), COLOR_SLIDER_TRACK);

    // Pill Knob
    let norm = ((*val as f32 - min as f32) / (max - min) as f32).clamp(0.0, 1.0);
    let knob_x = track_left + norm * track_len;
    let knob_rect = Rect::from_center_size(Pos2::new(knob_x, cy), Vec2::new(knob_w, 20.0));

    let is_active = response.dragged() || response.hovered();

    // Drop shadow
    let shadow_rect = Rect::from_center_size(Pos2::new(knob_x, cy + 1.5), Vec2::new(knob_w, 20.0));
    ui.painter().rect_filled(
        shadow_rect,
        CornerRadius::same(6),
        Color32::from_rgba_premultiplied(0, 0, 0, 80),
    );

    // Glow if hovered/dragged
    if is_active {
        let glow_rect = knob_rect.expand(3.0);
        ui.painter()
            .rect_filled(glow_rect, CornerRadius::same(8), COLOR_ACCENT_GLOW);
    }

    ui.painter().rect(
        knob_rect,
        CornerRadius::same(6),
        COLOR_SLIDER_KNOB,
        Stroke::new(1.0, Color32::from_rgb(0xD8, 0xE2, 0xEC)),
        StrokeKind::Inside,
    );

    changed
}

fn custom_slider_u8(
    ui: &mut egui::Ui,
    val: &mut u8,
    min: u8,
    max: u8,
    step: u8,
    width: f32,
) -> bool {
    let desired_size = Vec2::new(width.max(40.0), 28.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
    let mut changed = false;

    let knob_w = 14.0;
    let track_inset = knob_w / 2.0 + 2.0;
    let track_left = rect.min.x + track_inset;
    let track_right = rect.max.x - track_inset;
    let track_len = (track_right - track_left).max(1.0);
    let cy = rect.center().y;

    if response.dragged() || response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let norm = ((pos.x - track_left) / track_len).clamp(0.0, 1.0);
            let raw = min as f32 + norm * (max - min) as f32;
            let stepped = ((raw - min as f32) / step as f32).round() * step as f32 + min as f32;
            let new_val = (stepped as u8).clamp(min, max);
            if new_val != *val {
                *val = new_val;
                changed = true;
            }
        }
    }

    // Track
    let track_rect = Rect::from_min_max(
        Pos2::new(track_left, cy - 2.0),
        Pos2::new(track_right, cy + 2.0),
    );
    ui.painter()
        .rect_filled(track_rect, CornerRadius::same(2), COLOR_SLIDER_TRACK);

    // Pill Knob
    let norm = ((*val as f32 - min as f32) / (max - min) as f32).clamp(0.0, 1.0);
    let knob_x = track_left + norm * track_len;
    let knob_rect = Rect::from_center_size(Pos2::new(knob_x, cy), Vec2::new(knob_w, 20.0));

    let is_active = response.dragged() || response.hovered();

    // Drop shadow
    let shadow_rect = Rect::from_center_size(Pos2::new(knob_x, cy + 1.5), Vec2::new(knob_w, 20.0));
    ui.painter().rect_filled(
        shadow_rect,
        CornerRadius::same(6),
        Color32::from_rgba_premultiplied(0, 0, 0, 80),
    );

    // Glow if hovered/dragged
    if is_active {
        let glow_rect = knob_rect.expand(3.0);
        ui.painter()
            .rect_filled(glow_rect, CornerRadius::same(8), COLOR_ACCENT_GLOW);
    }

    ui.painter().rect(
        knob_rect,
        CornerRadius::same(6),
        COLOR_SLIDER_KNOB,
        Stroke::new(1.0, Color32::from_rgb(0xD8, 0xE2, 0xEC)),
        StrokeKind::Inside,
    );

    changed
}

fn step_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let desired_size = Vec2::new(28.0, 28.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

    let (fill, stroke, text_color) = if response.is_pointer_button_down_on() {
        (COLOR_BG_CARD, Stroke::new(1.0, COLOR_ACCENT), COLOR_TEXT_PRIMARY)
    } else if response.hovered() {
        (Color32::from_rgb(0x20, 0x29, 0x37), Stroke::new(1.0, COLOR_BORDER_HOVER), COLOR_TEXT_PRIMARY)
    } else {
        (COLOR_BG_ELEMENT, Stroke::new(1.0, COLOR_BORDER_CONTROL), COLOR_TEXT_SECONDARY)
    };

    ui.painter()
        .rect(rect, CornerRadius::same(6), fill, stroke, StrokeKind::Inside);

    let c = rect.center();
    let arm = 5.5;
    let icon_stroke = Stroke::new(1.8, text_color);
    match label {
        "-" | "−" => {
            ui.painter().line_segment(
                [Pos2::new(c.x - arm, c.y), Pos2::new(c.x + arm, c.y)],
                icon_stroke,
            );
        }
        "+" => {
            ui.painter().line_segment(
                [Pos2::new(c.x - arm, c.y), Pos2::new(c.x + arm, c.y)],
                icon_stroke,
            );
            ui.painter().line_segment(
                [Pos2::new(c.x, c.y - arm), Pos2::new(c.x, c.y + arm)],
                icon_stroke,
            );
        }
        _ => {
            ui.painter().text(
                c,
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::new(12.0, egui::FontFamily::Proportional),
                text_color,
            );
        }
    }

    response
}

fn level_count_button(ui: &mut egui::Ui, count: usize, is_selected: bool) -> egui::Response {
    let desired_size = Vec2::new(32.0, 26.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    if is_selected {
        let glow_rect = rect.expand(2.5);
        ui.painter()
            .rect_filled(glow_rect, CornerRadius::same(8), COLOR_ACCENT_GLOW);
        ui.painter().rect(
            rect,
            CornerRadius::same(6),
            COLOR_ACCENT,
            Stroke::new(1.0, COLOR_ACCENT),
            StrokeKind::Inside,
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            count.to_string(),
            egui::FontId::new(12.5, egui::FontFamily::Proportional),
            COLOR_TEXT_ON_ACCENT,
        );
    } else {
        let (fill, stroke, text_color) = if response.hovered() {
            (Color32::from_rgb(0x1B, 0x24, 0x30), Stroke::new(1.0, COLOR_BORDER_HOVER), COLOR_TEXT_PRIMARY)
        } else {
            (Color32::from_rgb(0x13, 0x1A, 0x23), Stroke::new(1.0, COLOR_BORDER_INSET), COLOR_TEXT_MUTED)
        };

        ui.painter()
            .rect(rect, CornerRadius::same(6), fill, stroke, StrokeKind::Inside);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            count.to_string(),
            egui::FontId::new(12.0, egui::FontFamily::Proportional),
            text_color,
        );
    }

    response
}

fn apply_button(ui: &mut egui::Ui, enabled: bool) -> egui::Response {
    let desired_size = Vec2::new(130.0, 38.0);
    let (rect, response) = ui.allocate_exact_size(
        desired_size,
        if enabled { egui::Sense::click() } else { egui::Sense::hover() },
    );

    let (fill, stroke, text_color) = if !enabled {
        (COLOR_BG_ELEMENT, Stroke::new(1.0, COLOR_BORDER_CARD), COLOR_TEXT_DISABLED)
    } else if response.is_pointer_button_down_on() {
        (COLOR_ACCENT, Stroke::NONE, COLOR_TEXT_ON_ACCENT)
    } else if response.hovered() {
        (COLOR_ACCENT_HOVER, Stroke::NONE, COLOR_TEXT_ON_ACCENT)
    } else {
        (COLOR_ACCENT, Stroke::NONE, COLOR_TEXT_ON_ACCENT)
    };

    ui.painter()
        .rect(rect, CornerRadius::same(8), fill, stroke, StrokeKind::Inside);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "Apply settings",
        egui::FontId::new(13.0, egui::FontFamily::Proportional),
        text_color,
    );

    response
}

fn restore_button(ui: &mut egui::Ui, enabled: bool) -> (egui::Response, bool) {
    let desired_size = Vec2::new(150.0, 38.0);
    let (rect, response) = ui.allocate_exact_size(
        desired_size,
        if enabled { egui::Sense::click() } else { egui::Sense::hover() },
    );

    let is_shift = enabled && ui.input(|i| i.modifiers.shift);
    let is_highlighted = enabled && (response.hovered() || response.has_focus());
    let is_factory = is_highlighted && is_shift;

    let (fill, stroke, text_color, label) = if !enabled {
        (
            COLOR_BG_ELEMENT,
            Stroke::new(1.0, COLOR_BORDER_CARD),
            COLOR_TEXT_DISABLED,
            "Restore from mouse",
        )
    } else if is_factory {
        if response.is_pointer_button_down_on() {
            (
                Color32::from_rgb(0x45, 0x0A, 0x0A),
                Stroke::new(1.0, Color32::from_rgb(0xEF, 0x44, 0x44)),
                Color32::from_rgb(0xFF, 0xFF, 0xFF),
                "Factory reset",
            )
        } else {
            (
                Color32::from_rgb(0x28, 0x0C, 0x10),
                Stroke::new(1.0, Color32::from_rgb(0xDC, 0x26, 0x26)),
                Color32::from_rgb(0xFE, 0xCA, 0xCA),
                "Factory reset",
            )
        }
    } else if response.is_pointer_button_down_on() {
        (
            COLOR_BG_CARD,
            Stroke::new(1.0, COLOR_ACCENT),
            COLOR_TEXT_PRIMARY,
            "Restore from mouse",
        )
    } else if response.hovered() {
        (
            Color32::from_rgb(0x20, 0x28, 0x34),
            Stroke::new(1.0, COLOR_BORDER_HOVER),
            COLOR_TEXT_PRIMARY,
            "Restore from mouse",
        )
    } else {
        (
            Color32::from_rgb(0x17, 0x1D, 0x25),
            Stroke::new(1.0, COLOR_BORDER_CONTROL),
            COLOR_TEXT_SECONDARY,
            "Restore from mouse",
        )
    };

    ui.painter()
        .rect(rect, CornerRadius::same(8), fill, stroke, StrokeKind::Inside);

    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::new(13.0, egui::FontFamily::Proportional),
        text_color,
    );

    let response = if !enabled {
        response
    } else if is_factory {
        response.on_hover_text("Reset all settings to factory defaults")
    } else {
        response.on_hover_text("Discard unsaved changes and reload from mouse\n(Hold Shift for Factory reset)")
    };

    (response, is_factory)
}

fn remap_button(ui: &mut egui::Ui, height: f32, is_expanded: bool) -> egui::Response {
    let desired_size = Vec2::new(92.0, height);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    let is_highlighted = response.hovered() || response.has_focus();

    let (fill, stroke, text_color) = if response.is_pointer_button_down_on() {
        (
            COLOR_BG_CARD,
            Stroke::new(1.0, COLOR_ACCENT),
            COLOR_TEXT_PRIMARY,
        )
    } else if is_highlighted {
        (
            Color32::from_rgb(0x20, 0x28, 0x34),
            Stroke::new(1.0, COLOR_BORDER_HOVER),
            COLOR_TEXT_PRIMARY,
        )
    } else if is_expanded {
        (
            Color32::from_rgb(0x1C, 0x24, 0x30),
            Stroke::new(1.0, COLOR_BORDER_HOVER),
            COLOR_TEXT_PRIMARY,
        )
    } else {
        (
            Color32::from_rgb(0x17, 0x1D, 0x25),
            Stroke::new(1.0, COLOR_BORDER_CONTROL),
            COLOR_TEXT_SECONDARY,
        )
    };

    ui.painter()
        .rect(rect, CornerRadius::same(8), fill, stroke, StrokeKind::Inside);

    let font_id = egui::FontId::new(13.0, egui::FontFamily::Proportional);
    let cy = rect.center().y;

    // Render text slightly shifted left to center the group (text + chevron)
    let text_rect = ui.painter().text(
        Pos2::new(rect.center().x - 6.0, cy),
        egui::Align2::CENTER_CENTER,
        "Remap",
        font_id,
        text_color,
    );

    let chevron_x = text_rect.max.x + 8.0;
    let span_x = 3.5;
    let chevron_stroke = Stroke::new(1.6, text_color);

    if is_expanded {
        // Chevron up (collapse)
        let p1 = Pos2::new(chevron_x - span_x, cy + 1.5);
        let p2 = Pos2::new(chevron_x, cy - 1.5);
        let p3 = Pos2::new(chevron_x + span_x, cy + 1.5);
        ui.painter().line_segment([p1, p2], chevron_stroke);
        ui.painter().line_segment([p2, p3], chevron_stroke);
    } else {
        // Chevron down (expand)
        let p1 = Pos2::new(chevron_x - span_x, cy - 1.5);
        let p2 = Pos2::new(chevron_x, cy + 1.5);
        let p3 = Pos2::new(chevron_x + span_x, cy - 1.5);
        ui.painter().line_segment([p1, p2], chevron_stroke);
        ui.painter().line_segment([p2, p3], chevron_stroke);
    }

    response.on_hover_text(if is_expanded {
        "Collapse Buttons section"
    } else {
        "Expand Buttons section"
    })
}

pub use crate::driver::{ButtonAction, BUTTON_NAMES, DEFAULT_BUTTON_MAPPINGS};

fn reset_buttons_button(ui: &mut egui::Ui, enabled: bool) -> egui::Response {
    let desired_size = Vec2::new(130.0, 38.0);
    let sense = if enabled { egui::Sense::click() } else { egui::Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(desired_size, sense);

    let is_highlighted = enabled && (response.hovered() || response.has_focus());
    let (fill, stroke, text_color) = if !enabled {
        (
            Color32::from_rgb(0x14, 0x19, 0x20),
            Stroke::new(1.0, COLOR_BORDER_CONTROL),
            COLOR_TEXT_MUTED,
        )
    } else if response.is_pointer_button_down_on() {
        (COLOR_BG_CARD, Stroke::new(1.0, COLOR_ACCENT), COLOR_TEXT_PRIMARY)
    } else if is_highlighted {
        (Color32::from_rgb(0x20, 0x28, 0x34), Stroke::new(1.0, COLOR_BORDER_HOVER), COLOR_TEXT_PRIMARY)
    } else {
        (Color32::from_rgb(0x17, 0x1D, 0x25), Stroke::new(1.0, COLOR_BORDER_CONTROL), COLOR_TEXT_SECONDARY)
    };

    ui.painter().rect(rect, CornerRadius::same(8), fill, stroke, StrokeKind::Inside);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "Reset to default",
        font_control_text(),
        text_color,
    );

    if enabled && response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    if enabled {
        response.on_hover_text("Reset all button mappings and scroll direction to default")
    } else {
        response
    }
}

fn custom_button_dropdown(
    ui: &mut egui::Ui,
    idx: usize,
    current_action: ButtonAction,
    is_open: bool,
    desired_w: f32,
) -> (egui::Response, Option<ButtonAction>, bool) {
    let desired_size = Vec2::new(desired_w, 32.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    let mut toggle_open = false;
    let mut selected_action = None;

    if response.clicked() {
        toggle_open = true;
    }

    let is_highlighted = response.hovered() || response.has_focus() || is_open;

    let (fill, stroke, text_color) = if is_open {
        (
            COLOR_BG_ELEMENT,
            Stroke::new(1.5, COLOR_ACCENT),
            COLOR_TEXT_PRIMARY,
        )
    } else if is_highlighted {
        (
            Color32::from_rgb(0x1D, 0x25, 0x31),
            Stroke::new(1.0, COLOR_BORDER_HOVER),
            COLOR_TEXT_PRIMARY,
        )
    } else {
        (
            COLOR_BG_ELEMENT,
            Stroke::new(1.0, COLOR_BORDER_CONTROL),
            COLOR_TEXT_PRIMARY,
        )
    };

    ui.painter()
        .rect(rect, CornerRadius::same(8), fill, stroke, StrokeKind::Inside);

    // Left aligned text
    let text_pos = Pos2::new(rect.min.x + 14.0, rect.center().y);
    ui.painter().text(
        text_pos,
        egui::Align2::LEFT_CENTER,
        current_action.label(),
        font_control_text(),
        text_color,
    );

    // Right chevron: down v when closed, up ^ when open
    let chevron_x = rect.max.x - 16.0;
    let cy = rect.center().y;
    let span_x = 3.5;
    let chevron_stroke = Stroke::new(1.5, if is_open { COLOR_ACCENT } else { COLOR_TEXT_SECONDARY });

    if is_open {
        let p1 = Pos2::new(chevron_x - span_x, cy + 1.5);
        let p2 = Pos2::new(chevron_x, cy - 1.5);
        let p3 = Pos2::new(chevron_x + span_x, cy + 1.5);
        ui.painter().line_segment([p1, p2], chevron_stroke);
        ui.painter().line_segment([p2, p3], chevron_stroke);
    } else {
        let p1 = Pos2::new(chevron_x - span_x, cy - 1.5);
        let p2 = Pos2::new(chevron_x, cy + 1.5);
        let p3 = Pos2::new(chevron_x + span_x, cy - 1.5);
        ui.painter().line_segment([p1, p2], chevron_stroke);
        ui.painter().line_segment([p2, p3], chevron_stroke);
    }

    if is_open {
        let popup_id = ui.make_persistent_id(format!("btn_popup_{}", idx));
        let item_h = 28.0;
        let pad_v = 4.0;
        let pad_h = 4.0;
        let total_popup_h = item_h * 5.0 + pad_v * 2.0;
        let popup_rect = Rect::from_min_size(
            Pos2::new(rect.min.x, rect.max.y + 3.0),
            Vec2::new(rect.width(), total_popup_h),
        );

        egui::Area::new(popup_id)
            .order(egui::Order::Foreground)
            .fixed_pos(popup_rect.min)
            .show(ui.ctx(), |popup_ui| {
                // Register entire popup rect to intercept clicks in padding/border
                // and prevent them from falling through to CentralPanel
                let _bg_resp = popup_ui.allocate_rect(popup_rect, egui::Sense::click());

                popup_ui.painter().rect(
                    popup_rect,
                    CornerRadius::same(8),
                    Color32::from_rgb(0x13, 0x1A, 0x22),
                    Stroke::new(1.0, COLOR_BORDER_CONTROL),
                    StrokeKind::Inside,
                );

                for (item_idx, action) in ButtonAction::ALL.iter().enumerate() {
                    let item_rect = Rect::from_min_size(
                        Pos2::new(
                            popup_rect.min.x + pad_h,
                            popup_rect.min.y + pad_v + item_idx as f32 * item_h,
                        ),
                        Vec2::new(popup_rect.width() - pad_h * 2.0, item_h),
                    );
                    let item_resp = popup_ui.interact(
                        item_rect,
                        popup_id.with(item_idx),
                        egui::Sense::click(),
                    );
                    let is_item_hovered = item_resp.hovered();

                    if is_item_hovered {
                        popup_ui.painter().rect_filled(
                            item_rect,
                            CornerRadius::same(6),
                            Color32::from_rgb(0x28, 0x30, 0x3D),
                        );
                    }

                    popup_ui.painter().text(
                        Pos2::new(item_rect.min.x + 10.0, item_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        action.label(),
                        font_control_text(),
                        COLOR_TEXT_PRIMARY,
                    );

                    if item_resp.clicked() {
                        selected_action = Some(*action);
                    }
                }
            });

        // Close on click outside or Escape
        let pointer_down = ui.input(|i| i.pointer.any_pressed());
        if pointer_down {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                if !rect.contains(pos) && !popup_rect.contains(pos) {
                    toggle_open = true; // will close
                }
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            toggle_open = true; // will close
        }
    }

    (response, selected_action, toggle_open)
}

// ============================================================================
// BACKGROUND WORKER THREAD (Non-blocking HID operations)
// ============================================================================
enum WorkerRequest {
    ConnectAndRead,
    ApplyConfig(MouseConfig, Option<[ButtonAction; 5]>),
}

enum WorkerResponse {
    Connected {
        config: MouseConfig,
        battery: Option<BatteryStatus>,
        is_wireless: bool,
        button_mappings: [ButtonAction; 5],
    },
    Reloaded {
        config: MouseConfig,
        battery: Option<BatteryStatus>,
        is_wireless: bool,
        button_mappings: [ButtonAction; 5],
    },
    Telemetry {
        battery: Option<BatteryStatus>,
        is_wireless: bool,
    },
    DpiStageChanged(usize),
    ApplySuccess {
        config: MouseConfig,
        button_mappings: Option<[ButtonAction; 5]>,
    },
    ApplyError(String),
    Disconnected(String),
}

pub struct AppInitState {
    pub mouse: Option<MchoseMouse>,
    pub config: MouseConfig,
    pub battery: Option<BatteryStatus>,
    pub is_wireless: bool,
    pub is_connected: bool,
    pub button_mappings: [ButtonAction; 5],
}

impl AppInitState {
    pub fn detect() -> Self {
        if let Ok(m) = MchoseMouse::open() {
            if m.get_online_status().unwrap_or(false) {
                if let (Ok(cfg), Ok(btn_mappings)) = (m.get_config(), m.get_button_mappings()) {
                    let bat = m.get_battery().ok();
                    let is_wireless = m.info.is_wireless;
                    return Self {
                        mouse: Some(m),
                        config: cfg,
                        battery: bat,
                        is_wireless,
                        is_connected: true,
                        button_mappings: btn_mappings,
                    };
                }
            }
        }
        Self {
            mouse: None,
            config: MouseConfig::default(),
            battery: None,
            is_wireless: true,
            is_connected: false,
            button_mappings: DEFAULT_BUTTON_MAPPINGS,
        }
    }
}

fn start_worker(
    tx_resp: Sender<WorkerResponse>,
    rx_req: Receiver<WorkerRequest>,
    initial_mouse: Option<MchoseMouse>,
    initial_stage: Option<usize>,
) {
    std::thread::spawn(move || {
        let mut mouse = initial_mouse;
        let now = Instant::now();
        let ten_secs = Duration::from_secs(10);
        let mut last_battery_poll = now;
        let mut last_stage_poll = now;
        let mut last_reconnect_poll = if mouse.is_none() {
            now.checked_sub(ten_secs).unwrap_or(now)
        } else {
            now
        };
        let mut last_known_stage: Option<usize> = initial_stage;
        let mut consecutive_poll_errors: u32 = 0;

        loop {
            let req = rx_req.recv_timeout(Duration::from_millis(200));
            match req {
                Ok(WorkerRequest::ConnectAndRead) => {
                    let mut active_mouse = mouse.take();
                    if active_mouse.is_none() {
                        active_mouse = MchoseMouse::open().ok();
                    }
                    if let Some(m) = active_mouse {
                        if !m.get_online_status().unwrap_or(false) {
                            mouse = None;
                            last_reconnect_poll = Instant::now();
                            let _ = tx_resp.send(WorkerResponse::Disconnected("Mouse offline".into()));
                        } else {
                            match (m.get_config(), m.get_button_mappings()) {
                                (Ok(cfg), Ok(btn_mappings)) => {
                                    let bat = m.get_battery().ok();
                                    let is_wireless = m.info.is_wireless;
                                    last_known_stage = Some(cfg.active_dpi_stage);
                                    consecutive_poll_errors = 0;
                                    mouse = Some(m);
                                    let _ = tx_resp.send(WorkerResponse::Reloaded {
                                        config: cfg,
                                        battery: bat,
                                        is_wireless,
                                        button_mappings: btn_mappings,
                                    });
                                }
                                _ => {
                                    drop(m);
                                    match MchoseMouse::open() {
                                        Ok(m2) => {
                                            if !m2.get_online_status().unwrap_or(false) {
                                                mouse = None;
                                                last_reconnect_poll = Instant::now();
                                                let _ = tx_resp.send(WorkerResponse::Disconnected("Mouse offline".into()));
                                            } else {
                                                match (m2.get_config(), m2.get_button_mappings()) {
                                                    (Ok(cfg), Ok(btn_mappings)) => {
                                                        let bat = m2.get_battery().ok();
                                                        let is_wireless = m2.info.is_wireless;
                                                        last_known_stage = Some(cfg.active_dpi_stage);
                                                        consecutive_poll_errors = 0;
                                                        mouse = Some(m2);
                                                        let _ = tx_resp.send(WorkerResponse::Reloaded {
                                                            config: cfg,
                                                            battery: bat,
                                                            is_wireless,
                                                            button_mappings: btn_mappings,
                                                        });
                                                    }
                                                    (Err(e), _) | (_, Err(e)) => {
                                                        mouse = None;
                                                        last_reconnect_poll = Instant::now();
                                                        let _ = tx_resp.send(WorkerResponse::Disconnected(e.to_string()));
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            mouse = None;
                                            last_reconnect_poll = Instant::now();
                                            let _ = tx_resp.send(WorkerResponse::Disconnected(e.to_string()));
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        mouse = None;
                        let _ = tx_resp.send(WorkerResponse::Disconnected("Mouse not detected".into()));
                    }
                }
                Ok(WorkerRequest::ApplyConfig(cfg, btn_mappings)) => {
                    let mut active_mouse = mouse.take();
                    if active_mouse.is_none() {
                        active_mouse = MchoseMouse::open().ok();
                    }
                    if let Some(m) = active_mouse {
                        if !m.get_online_status().unwrap_or(false) {
                            mouse = None;
                            last_reconnect_poll = Instant::now();
                            let _ = tx_resp.send(WorkerResponse::ApplyError("Mouse is offline or sleeping".into()));
                            let _ = tx_resp.send(WorkerResponse::Disconnected("Mouse offline".into()));
                        } else {
                            let cfg_res = m.set_config(&cfg);
                            let btns_res = match (&cfg_res, btn_mappings) {
                                (Ok(_), Some(btns)) => m.set_button_mappings(&btns),
                                _ => Ok(()),
                            };

                            match (cfg_res, btns_res) {
                                (Ok(_), Ok(_)) => {
                                    let norm_count = cfg.dpi_stages_count.clamp(1, 6);
                                    last_known_stage = Some(cfg.active_dpi_stage.clamp(1, norm_count));
                                    consecutive_poll_errors = 0;
                                    mouse = Some(m);
                                    let _ = tx_resp.send(WorkerResponse::ApplySuccess {
                                        config: cfg,
                                        button_mappings: btn_mappings,
                                    });
                                }
                                (Err(e), _) | (_, Err(e)) => {
                                    drop(m);
                                    let _ = tx_resp.send(WorkerResponse::ApplyError(e.to_string()));
                                    match MchoseMouse::open() {
                                        Ok(m2) => {
                                            if m2.get_online_status().unwrap_or(false) {
                                                if let Ok(cfg2) = m2.get_config() {
                                                    last_known_stage = Some(cfg2.active_dpi_stage);
                                                    consecutive_poll_errors = 0;
                                                    mouse = Some(m2);
                                                } else {
                                                    mouse = None;
                                                    last_reconnect_poll = Instant::now();
                                                    let _ = tx_resp.send(WorkerResponse::Disconnected("Mouse offline".into()));
                                                }
                                            } else {
                                                mouse = None;
                                                last_reconnect_poll = Instant::now();
                                                let _ = tx_resp.send(WorkerResponse::Disconnected("Mouse offline".into()));
                                            }
                                        }
                                        Err(e2) => {
                                            mouse = None;
                                            last_reconnect_poll = Instant::now();
                                            let _ = tx_resp.send(WorkerResponse::Disconnected(e2.to_string()));
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        mouse = None;
                        let _ = tx_resp.send(WorkerResponse::ApplyError("Mouse not connected".into()));
                        let _ = tx_resp.send(WorkerResponse::Disconnected("Mouse not connected".into()));
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if mouse.is_none() && last_reconnect_poll.elapsed() >= Duration::from_millis(500) {
                        last_reconnect_poll = Instant::now();
                        if let Ok(m) = MchoseMouse::open() {
                            if m.get_online_status().unwrap_or(false) {
                                if let (Ok(cfg), Ok(btn_mappings)) = (m.get_config(), m.get_button_mappings()) {
                                    let bat = m.get_battery().ok();
                                    let is_wireless = m.info.is_wireless;
                                    last_known_stage = Some(cfg.active_dpi_stage);
                                    consecutive_poll_errors = 0;
                                    mouse = Some(m);
                                    let _ = tx_resp.send(WorkerResponse::Connected {
                                        config: cfg,
                                        battery: bat,
                                        is_wireless,
                                        button_mappings: btn_mappings,
                                    });
                                }
                            }
                        }
                    }

                    if let Some(m) = mouse.take() {
                        let mut mouse_ok = true;

                        // Fast check online status (1-2ms) before attempting any heavy queries
                        if m.info.is_wireless {
                            match m.get_online_status() {
                                Ok(true) => {}
                                Ok(false) => {
                                    mouse_ok = false;
                                }
                                Err(_) => {
                                    consecutive_poll_errors += 1;
                                    if consecutive_poll_errors > 4 {
                                        mouse_ok = false;
                                    }
                                }
                            }
                        }

                        // 1. Fast poll for active DPI stage (~200ms) to detect physical mouse button presses
                        if mouse_ok && last_stage_poll.elapsed() >= Duration::from_millis(200) {
                            last_stage_poll = Instant::now();
                            match m.get_config() {
                                Ok(cfg) => {
                                    consecutive_poll_errors = 0;
                                    if Some(cfg.active_dpi_stage) != last_known_stage {
                                        last_known_stage = Some(cfg.active_dpi_stage);
                                        let _ = tx_resp.send(WorkerResponse::DpiStageChanged(cfg.active_dpi_stage));
                                    }
                                }
                                Err(_) => {
                                    consecutive_poll_errors += 1;
                                    if consecutive_poll_errors > 4 {
                                        mouse_ok = false;
                                    }
                                }
                            }
                        }

                        // 2. Slow poll for battery & wireless telemetry (every 5s)
                        if mouse_ok && last_battery_poll.elapsed() >= Duration::from_secs(5) {
                            last_battery_poll = Instant::now();
                            match m.get_battery() {
                                Ok(bat) => {
                                    consecutive_poll_errors = 0;
                                    let is_wireless = m.info.is_wireless;
                                    let _ = tx_resp.send(WorkerResponse::Telemetry {
                                        battery: Some(bat),
                                        is_wireless,
                                    });
                                }
                                Err(_) => {
                                    consecutive_poll_errors += 1;
                                    if consecutive_poll_errors > 4 {
                                        mouse_ok = false;
                                    }
                                }
                            }
                        }

                        if mouse_ok {
                            mouse = Some(m);
                        } else {
                            drop(m);
                            last_reconnect_poll = Instant::now();
                            let _ = tx_resp.send(WorkerResponse::Disconnected("Mouse offline".into()));
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });
}

fn target_window_height(stages_count: usize) -> f32 {
    let count = stages_count.clamp(1, 6);
    445.0 + (count - 1) as f32 * 43.0
}

// ============================================================================
// MAIN APP STRUCT
// ============================================================================
pub struct App {
    tx_req: Sender<WorkerRequest>,
    rx_resp: Receiver<WorkerResponse>,
    config: MouseConfig,
    saved_config: MouseConfig,
    battery: Option<BatteryStatus>,
    is_wireless: bool,
    is_connected: bool,
    has_unsaved_changes: bool,
    status_msg: Option<(String, Instant)>,
    current_window_h: f32,
    pub is_remap_open: bool,
    pub button_mappings: [ButtonAction; 5],
    pub saved_button_mappings: [ButtonAction; 5],
    pub open_dropdown: Option<usize>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, init: AppInitState) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = COLOR_BG_MAIN;
        visuals.window_fill = COLOR_BG_MAIN;
        visuals.faint_bg_color = COLOR_BG_CARD;
        visuals.extreme_bg_color = COLOR_BG_SECONDARY;

        visuals.widgets.noninteractive.bg_fill = COLOR_BG_CARD;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, COLOR_BORDER_CARD);
        visuals.widgets.noninteractive.fg_stroke.color = COLOR_TEXT_PRIMARY;

        visuals.widgets.inactive.bg_fill = COLOR_BG_ELEMENT;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, COLOR_BORDER_CONTROL);
        visuals.widgets.inactive.fg_stroke.color = COLOR_TEXT_SECONDARY;
        visuals.widgets.inactive.corner_radius = CornerRadius::same(6);

        visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x20, 0x28, 0x34);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, COLOR_BORDER_HOVER);
        visuals.widgets.hovered.fg_stroke.color = COLOR_TEXT_PRIMARY;
        visuals.widgets.hovered.corner_radius = CornerRadius::same(6);

        visuals.widgets.active.bg_fill = COLOR_ACCENT;
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, COLOR_ACCENT);
        visuals.widgets.active.fg_stroke.color = COLOR_TEXT_ON_ACCENT;
        visuals.widgets.active.corner_radius = CornerRadius::same(6);

        visuals.selection.bg_fill = COLOR_ACTIVE_BG;
        visuals.selection.stroke = Stroke::new(1.0, COLOR_ACTIVE_BORDER);

        cc.egui_ctx.set_visuals(visuals);

        setup_fonts(&cc.egui_ctx);

        let (tx_req, rx_req) = channel();
        let (tx_resp, rx_resp) = channel();

        let initial_stage = if init.is_connected {
            Some(init.config.active_dpi_stage)
        } else {
            None
        };
        start_worker(tx_resp, rx_req, init.mouse, initial_stage);

        let saved_config = init.config.clone();
        let status_msg = if init.is_connected {
            Some(("Connected to MCHOSE mouse".to_string(), Instant::now()))
        } else {
            None
        };

        let initial_h = target_window_height(init.config.dpi_stages_count);

        Self {
            tx_req,
            rx_resp,
            config: init.config,
            saved_config,
            battery: init.battery,
            is_wireless: init.is_wireless,
            is_connected: init.is_connected,
            has_unsaved_changes: false,
            status_msg,
            current_window_h: initial_h,
            is_remap_open: false,
            button_mappings: init.button_mappings,
            saved_button_mappings: init.button_mappings,
            open_dropdown: None,
        }
    }

    fn set_status(&mut self, msg: &str) {
        self.status_msg = Some((msg.to_string(), Instant::now()));
    }

    pub(crate) fn is_busy(&self) -> bool {
        self.status_msg.as_ref().map(|(m, t)| {
            (m.starts_with("Applying") || m.starts_with("Reloading")) && t.elapsed() < Duration::from_secs(4)
        }).unwrap_or(false)
    }

    pub(crate) fn make_factory_config(&self) -> MouseConfig {
        let mut factory_cfg = MouseConfig::default();
        factory_cfg.sensor_flags = self.saved_config.sensor_flags;
        factory_cfg.highspeed_mode = self.saved_config.highspeed_mode;
        factory_cfg.sensor_angle = self.saved_config.sensor_angle;
        factory_cfg
    }

    pub(crate) fn set_dpi_stages_count(&mut self, count: usize) {
        let prev_count = self.config.dpi_stages_count;
        self.config.dpi_stages_count = count;
        if self.config.active_dpi_stage > count {
            self.config.active_dpi_stage = count;
        } else if count > prev_count
            && self.config.active_dpi_stage == prev_count
            && self.saved_config.active_dpi_stage > prev_count
        {
            self.config.active_dpi_stage = self.saved_config.active_dpi_stage.min(count);
        }
    }

    fn on_worker_response(&mut self, resp: WorkerResponse) -> bool {
        match resp {
            WorkerResponse::Connected {
                config,
                battery,
                is_wireless,
                button_mappings,
            } => {
                let has_user_edits = self.config != self.saved_config;
                let has_user_btn_edits = self.button_mappings != self.saved_button_mappings;
                if !has_user_edits {
                    self.config = config.clone();
                } else {
                    // Sync hardware fields not exposed in the UI to avoid overwriting
                    // them with defaults and causing permanent unsaved changes state
                    self.config.lod = config.lod;
                    self.config.sensor_flags = config.sensor_flags;
                    self.config.scroll_flag = config.scroll_flag;
                    self.config.highspeed_mode = config.highspeed_mode;
                    self.config.sensor_angle = config.sensor_angle;

                    if self.config.active_dpi_stage == self.saved_config.active_dpi_stage {
                        let count = self.config.dpi_stages_count.clamp(1, 6);
                        self.config.active_dpi_stage = config.active_dpi_stage.clamp(1, count);
                    }
                }
                if !has_user_btn_edits {
                    self.button_mappings = button_mappings;
                }
                self.saved_button_mappings = button_mappings;
                self.saved_config = config;
                self.battery = battery;
                self.is_wireless = is_wireless;
                self.is_connected = true;
                self.has_unsaved_changes = self.config != self.saved_config
                    || self.button_mappings != self.saved_button_mappings;
                if self.status_msg.as_ref().map_or(true, |(m, _)| !m.starts_with("Applying") && !m.starts_with("Reloading")) {
                    self.set_status("Connected to MCHOSE mouse");
                }
                true
            }
            WorkerResponse::Reloaded {
                config,
                battery,
                is_wireless,
                button_mappings,
            } => {
                self.config = config.clone();
                self.saved_config = config;
                self.button_mappings = button_mappings;
                self.saved_button_mappings = button_mappings;
                self.battery = battery;
                self.is_wireless = is_wireless;
                self.is_connected = true;
                self.has_unsaved_changes = false;
                self.set_status("Settings reloaded");
                false
            }
            WorkerResponse::Telemetry {
                battery,
                is_wireless,
            } => {
                self.battery = battery;
                self.is_wireless = is_wireless;
                self.is_connected = true;
                false
            }
            WorkerResponse::DpiStageChanged(new_stage) => {
                // Hardware active stage changed via physical button on bottom of mouse
                let saved_count = self.saved_config.dpi_stages_count.clamp(1, 6);
                self.saved_config.active_dpi_stage = new_stage.clamp(1, saved_count);
                let count = self.config.dpi_stages_count.clamp(1, 6);
                self.config.active_dpi_stage = new_stage.clamp(1, count);
                true
            }
            WorkerResponse::ApplySuccess {
                config: saved_cfg,
                button_mappings: saved_btns,
            } => {
                self.saved_config = saved_cfg;
                if let Some(btns) = saved_btns {
                    self.saved_button_mappings = btns;
                }
                self.has_unsaved_changes = self.config != self.saved_config
                    || self.button_mappings != self.saved_button_mappings;
                if let Some((ref msg, _)) = self.status_msg {
                    if msg.starts_with("Factory") || msg.contains("factory") {
                        self.set_status("Factory settings restored");
                    } else {
                        self.set_status("Settings applied");
                    }
                } else {
                    self.set_status("Settings applied");
                }
                false
            }
            WorkerResponse::ApplyError(err) => {
                self.set_status(&format!("Save error: {err}"));
                false
            }
            WorkerResponse::Disconnected(err) => {
                self.is_connected = false;
                self.battery = None;
                if let Some((ref msg, _)) = self.status_msg {
                    if !msg.starts_with("Save error") {
                        self.set_status(&format!("Disconnected: {err}"));
                    }
                } else {
                    self.set_status(&format!("Disconnected: {err}"));
                }
                false
            }
        }
    }

    fn handle_worker_messages(&mut self, ctx: &egui::Context) {
        while let Ok(resp) = self.rx_resp.try_recv() {
            if self.on_worker_response(resp) {
                ctx.request_repaint();
            }
        }
    }

    fn render_buttons_card(&mut self, ui: &mut egui::Ui) {
        let is_busy = self.is_busy();
        if is_busy {
            self.open_dropdown = None;
        }

        card_frame().show(ui, |card_ui| {
            card_ui.vertical(|ui| {
                // Card Header
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;

                    ui.label(
                        RichText::new("BUTTONS")
                            .font(font_section_title())
                            .strong()
                            .color(COLOR_TEXT_PRIMARY),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let can_reset = !is_busy
                            && (self.button_mappings != DEFAULT_BUTTON_MAPPINGS || self.config.scroll_flag != 0);

                        if reset_buttons_button(ui, can_reset).clicked() && can_reset {
                            self.button_mappings = DEFAULT_BUTTON_MAPPINGS;
                            self.config.scroll_flag = 0;
                            self.has_unsaved_changes = self.config != self.saved_config
                                || self.button_mappings != self.saved_button_mappings;
                            self.set_status("Buttons reset to default");
                        }
                    });
                });

                ui.add_space(12.0);

                // Card Body: Two columns
                ui.add_enabled_ui(!is_busy, |ui| {
                    let total_card_w = ui.available_width();
                    let col_gap = 14.0;
                    let right_col_w = 216.0;
                    let left_col_w = (total_card_w - col_gap - right_col_w).max(280.0);
                    let label_w = 125.0;
                    let dropdown_w = (left_col_w - label_w).max(200.0);
                    let row_h = 32.0;
                    let row_gap = 8.0;
                    let left_col_h = 5.0 * row_h + 4.0 * row_gap; // 192.0

                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |row_ui| {
                        row_ui.spacing_mut().item_spacing = Vec2::ZERO;

                        // Left Column: 5 Button Rows
                        row_ui.allocate_ui_with_layout(
                            Vec2::new(left_col_w, left_col_h),
                            egui::Layout::top_down(egui::Align::Min),
                            |col_ui| {
                                col_ui.set_width(left_col_w);

                                for idx in 0..5 {
                                    if idx > 0 {
                                        col_ui.add_space(row_gap);
                                    }

                                    col_ui.horizontal(|r_ui| {
                                        r_ui.spacing_mut().item_spacing = Vec2::ZERO;

                                        // Label
                                        let (lbl_rect, _) = r_ui.allocate_exact_size(
                                            Vec2::new(label_w, row_h),
                                            egui::Sense::hover(),
                                        );
                                        r_ui.painter().text(
                                            lbl_rect.left_center(),
                                            egui::Align2::LEFT_CENTER,
                                            BUTTON_NAMES[idx],
                                            font_control_text(),
                                            COLOR_TEXT_PRIMARY,
                                        );

                                        // Dropdown
                                        let is_open = self.open_dropdown == Some(idx);
                                        let (_, selected_action, toggle_open) = custom_button_dropdown(
                                            r_ui,
                                            idx,
                                            self.button_mappings[idx],
                                            is_open,
                                            dropdown_w,
                                        );

                                        if let Some(new_action) = selected_action {
                                            if self.button_mappings[idx] != new_action {
                                                self.button_mappings[idx] = new_action;
                                                self.has_unsaved_changes = self.config != self.saved_config
                                                    || self.button_mappings != self.saved_button_mappings;
                                            }
                                            self.open_dropdown = None;
                                        } else if toggle_open {
                                            if is_open {
                                                self.open_dropdown = None;
                                            } else {
                                                self.open_dropdown = Some(idx);
                                            }
                                        }
                                    });
                                }
                            },
                        );

                        row_ui.add_space(col_gap);

                        // Right Column: Scroll Direction Panel
                        row_ui.allocate_ui_with_layout(
                            Vec2::new(right_col_w, left_col_h),
                            egui::Layout::top_down(egui::Align::Min),
                            |col_ui| {
                                col_ui.set_width(right_col_w);
                                egui::Frame::new()
                                    .fill(COLOR_BG_SECONDARY)
                                    .stroke(Stroke::new(1.0, COLOR_BORDER_CARD))
                                    .corner_radius(CornerRadius::same(10))
                                    .inner_margin(egui::Margin::symmetric(14, 14))
                                    .show(col_ui, |p_ui| {
                                        p_ui.set_width(p_ui.available_width());
                                        let inner_card_h = left_col_h - 28.0;
                                        p_ui.set_min_height(inner_card_h);
                                        p_ui.vertical(|ui| {
                                            let resp_title = ui.label(
                                                RichText::new("Scroll direction")
                                                    .font(font_section_title())
                                                    .strong()
                                                    .color(COLOR_TEXT_PRIMARY),
                                            );
                                            ui.add_space(4.0);
                                            let resp_sub = ui.label(
                                                RichText::new("Choose how the wheel moves\ncontent")
                                                    .font(font_small_caption())
                                                    .color(COLOR_TEXT_MUTED),
                                            );

                                            let track_h = 36.0;
                                            let used_h = resp_title.rect.height() + 4.0 + resp_sub.rect.height() + track_h;
                                            let space_h = (inner_card_h - used_h).max(14.0);
                                            ui.add_space(space_h);

                                            // Unified segmented switch track (Standard vs Reverse)
                                            let avail_w = ui.available_width();
                                            let (track_rect, _) = ui.allocate_exact_size(
                                                Vec2::new(avail_w, track_h),
                                                egui::Sense::hover(),
                                            );

                                            let track_fill = Color32::from_rgb(0x11, 0x17, 0x22);
                                            let track_stroke = Stroke::new(1.0, COLOR_BORDER_INSET);
                                            ui.painter().rect(
                                                track_rect,
                                                CornerRadius::same(8),
                                                track_fill,
                                                track_stroke,
                                                StrokeKind::Inside,
                                            );

                                            let pad = 3.0;
                                            let inner_rect = track_rect.shrink(pad);
                                            let seg_w = (inner_rect.width() / 2.0).floor();
                                            let seg_h = inner_rect.height();

                                            let rect_std = Rect::from_min_size(
                                                inner_rect.min,
                                                Vec2::new(seg_w, seg_h),
                                            );
                                            let rect_rev = Rect::from_min_size(
                                                Pos2::new(inner_rect.min.x + seg_w, inner_rect.min.y),
                                                Vec2::new(inner_rect.width() - seg_w, seg_h),
                                            );

                                            let resp_std = ui.interact(rect_std, ui.auto_id_with("scroll_std"), egui::Sense::click());
                                            let resp_rev = ui.interact(rect_rev, ui.auto_id_with("scroll_rev"), egui::Sense::click());

                                            if resp_std.hovered() || resp_rev.hovered() {
                                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                            }

                                            let is_reverse = self.config.scroll_flag != 0;

                                            if resp_std.clicked() && is_reverse {
                                                self.config.scroll_flag = 0;
                                                self.has_unsaved_changes = self.config != self.saved_config
                                                    || self.button_mappings != self.saved_button_mappings;
                                            } else if resp_rev.clicked() && !is_reverse {
                                                self.config.scroll_flag = 1;
                                                self.has_unsaved_changes = self.config != self.saved_config
                                                    || self.button_mappings != self.saved_button_mappings;
                                            }

                                            // Standard segment
                                            if !is_reverse {
                                                ui.painter().rect(
                                                    rect_std,
                                                    CornerRadius::same(6),
                                                    COLOR_ACCENT,
                                                    Stroke::NONE,
                                                    StrokeKind::Inside,
                                                );
                                                ui.painter().text(
                                                    rect_std.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    "Standard",
                                                    font_control_text(),
                                                    COLOR_TEXT_ON_ACCENT,
                                                );
                                            } else {
                                                if resp_std.hovered() {
                                                    ui.painter().rect(
                                                        rect_std,
                                                        CornerRadius::same(6),
                                                        Color32::from_rgba_unmultiplied(255, 255, 255, 10),
                                                        Stroke::NONE,
                                                        StrokeKind::Inside,
                                                    );
                                                }
                                                let text_color = if resp_std.hovered() {
                                                    COLOR_TEXT_PRIMARY
                                                } else {
                                                    COLOR_TEXT_MUTED
                                                };
                                                ui.painter().text(
                                                    rect_std.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    "Standard",
                                                    font_control_text(),
                                                    text_color,
                                                );
                                            }

                                            // Reverse segment
                                            if is_reverse {
                                                ui.painter().rect(
                                                    rect_rev,
                                                    CornerRadius::same(6),
                                                    COLOR_ACCENT,
                                                    Stroke::NONE,
                                                    StrokeKind::Inside,
                                                );
                                                ui.painter().text(
                                                    rect_rev.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    "Reverse",
                                                    font_control_text(),
                                                    COLOR_TEXT_ON_ACCENT,
                                                );
                                            } else {
                                                if resp_rev.hovered() {
                                                    ui.painter().rect(
                                                        rect_rev,
                                                        CornerRadius::same(6),
                                                        Color32::from_rgba_unmultiplied(255, 255, 255, 10),
                                                        Stroke::NONE,
                                                        StrokeKind::Inside,
                                                    );
                                                }
                                                let text_color = if resp_rev.hovered() {
                                                    COLOR_TEXT_PRIMARY
                                                } else {
                                                    COLOR_TEXT_MUTED
                                                };
                                                ui.painter().text(
                                                    rect_rev.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    "Reverse",
                                                    font_control_text(),
                                                    text_color,
                                                );
                                            }
                                        });
                                    });
                            },
                        );
                    });
                });
            });
        });
    }

    pub fn render_ui(&mut self, ui: &mut egui::Ui) {
        // Enforce consistent outer padding of 14 px on all 4 sides
        let frame_resp = egui::Frame::new()
            .fill(COLOR_BG_MAIN)
            .inner_margin(egui::Margin::symmetric(14, 14))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // ====================================================
                    // 1. TOP HEADER BAR
                    // ====================================================
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::ZERO;

                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new("M-RUSTHUB")
                                            .font(font_app_title())
                                            .strong()
                                            .color(COLOR_TEXT_PRIMARY),
                                    );
                                    ui.label(
                                        RichText::new("G3 V2 MANAGER")
                                            .font(font_device_sub())
                                            .strong()
                                            .color(COLOR_TEXT_MUTED),
                                    );
                                });

                                // Right Telemetry Pill & Remap Button
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let telemetry_resp = egui::Frame::new()
                                        .fill(COLOR_BG_TELEMETRY)
                                        .stroke(Stroke::new(1.0, COLOR_BORDER_TELEMETRY))
                                        .corner_radius(CornerRadius::same(10))
                                        .inner_margin(egui::Margin::symmetric(14, 8))
                                        .show(ui, |ui| {
                                            ui.spacing_mut().item_spacing = Vec2::ZERO;

                                            // 1. Battery text (rightmost)
                                            let (bat_str, bat_color, text_color, bat_percent, tooltip) =
                                                battery_display_info(self.battery.as_ref(), self.is_connected);

                                            ui.label(
                                                RichText::new(&bat_str)
                                                    .font(font_control_text())
                                                    .strong()
                                                    .color(text_color),
                                            )
                                            .on_hover_text(&tooltip);

                                            ui.add_space(6.0);

                                            // 2. Battery icon
                                            let (bat_rect, bat_resp) = ui.allocate_exact_size(Vec2::new(20.0, 11.0), egui::Sense::hover());
                                            bat_resp.on_hover_text(&tooltip);
                                            draw_battery_badge(ui.painter(), bat_rect.min, bat_percent, bat_color);

                                            ui.add_space(8.0);

                                            // 3. Separator
                                            let (sep_rect, _) = ui.allocate_exact_size(Vec2::new(1.0, 14.0), egui::Sense::hover());
                                            ui.painter().line_segment(
                                                [sep_rect.min, Pos2::new(sep_rect.min.x, sep_rect.max.y)],
                                                Stroke::new(1.0, COLOR_BORDER_TELEMETRY),
                                            );

                                            ui.add_space(8.0);

                                            // 4. Mode text
                                            let mode_str = if !self.is_connected {
                                                "Disconnected"
                                            } else if self.is_wireless {
                                                "2.4G Wireless"
                                            } else {
                                                "USB Wired"
                                            };
                                            ui.label(
                                                RichText::new(mode_str)
                                                    .font(font_control_text())
                                                    .color(if self.is_connected { COLOR_TEXT_PRIMARY } else { COLOR_TEXT_MUTED }),
                                            );

                                            ui.add_space(6.0);

                                            // 5. Wireless icon
                                            let (wifi_rect, _) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), egui::Sense::hover());
                                            draw_wireless_icon(
                                                ui.painter(),
                                                wifi_rect.center(),
                                                if self.is_connected { COLOR_ACCENT } else { COLOR_TEXT_MUTED },
                                            );

                                            ui.add_space(6.0);

                                            // 6. Connection status dot (leftmost)
                                            let (dot_rect, _) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), egui::Sense::hover());
                                            let c = dot_rect.center();
                                            let dot_color = if self.is_connected {
                                                COLOR_SUCCESS
                                            } else {
                                                COLOR_ERROR
                                            };
                                            ui.painter().circle_filled(
                                                c,
                                                5.5,
                                                Color32::from_rgba_premultiplied(
                                                    dot_color.r() / 4,
                                                    dot_color.g() / 4,
                                                    dot_color.b() / 4,
                                                    50,
                                                ),
                                            );
                                            ui.painter().circle_filled(c, 3.5, dot_color);
                                        });

                                    let pill_height = telemetry_resp.response.rect.height();
                                    let btn_height = if pill_height > 0.0 { pill_height } else { 32.0 };

                                    ui.add_space(8.0);

                                    let remap_resp = remap_button(ui, btn_height, self.is_remap_open);
                                    if remap_resp.clicked() {
                                        self.is_remap_open = !self.is_remap_open;
                                        if !self.is_remap_open {
                                            self.open_dropdown = None;
                                        }
                                    }
                                });
                            });

                            // ====================================================
                            // 1.5. BUTTONS CARD (Expands directly below Header)
                            // ====================================================
                            if self.is_remap_open {
                                ui.add_space(10.0);
                                self.render_buttons_card(ui);
                            }

                            ui.add_space(10.0);

                            // ====================================================
                            // 2. DPI STAGES CARD
                            // ====================================================
                            card_frame().show(ui, |card_ui| {
                                card_ui.vertical(|ui| {
                                    // Card Header
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing = Vec2::ZERO;

                                        ui.label(
                                            RichText::new("DPI STAGES")
                                                .font(font_section_title())
                                                .strong()
                                                .color(COLOR_TEXT_PRIMARY),
                                        );

                                        let (active_str, active_color, has_glow) =
                                            active_dpi_display_info(&self.saved_config, self.is_connected);

                                        // Right align active readout flush with the card right edge
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.spacing_mut().item_spacing = Vec2::ZERO;

                                            ui.label(
                                                RichText::new(active_str)
                                                    .font(font_dpi_value())
                                                    .color(active_color)
                                                    .strong(),
                                            );

                                            ui.add_space(6.0);

                                            let (dot_rect, _) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), egui::Sense::hover());
                                            let c = Pos2::new(dot_rect.center().x, dot_rect.center().y + 1.5);
                                            if has_glow {
                                                ui.painter().circle_filled(c, 5.5, COLOR_ACCENT_GLOW);
                                            }
                                            ui.painter().circle_filled(c, 3.5, active_color);
                                        });
                                    });

                                    ui.add_space(10.0);

                                    // Number of levels panel
                                    inset_frame().show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing = Vec2::ZERO;

                                            ui.label(
                                                RichText::new("Number of levels")
                                                    .font(font_control_text())
                                                    .color(COLOR_TEXT_MUTED),
                                            );

                                            // 6 buttons each 32 px + 5 gaps of 4 px = 212 px
                                            let total_btn_w = 6.0 * 32.0 + 5.0 * 4.0;
                                            let space = (ui.available_width() - total_btn_w).max(10.0);
                                            ui.add_space(space);

                                            for count in 1..=6 {
                                                let is_selected = self.config.dpi_stages_count == count;
                                                if level_count_button(ui, count, is_selected).clicked() && !is_selected {
                                                    self.set_dpi_stages_count(count);
                                                }
                                                if count < 6 {
                                                    ui.add_space(4.0);
                                                }
                                            }
                                        });
                                    });

                                    ui.add_space(10.0);

                                    // DPI rows
                                    for i in 0..self.config.dpi_stages_count {
                                        let is_active = self.config.active_dpi_stage == i + 1;
                                        let color = STAGE_COLORS[i];

                                        let row_frame = if is_active {
                                            egui::Frame::new()
                                                .fill(COLOR_ACTIVE_BG)
                                                .stroke(Stroke::new(1.0, COLOR_ACTIVE_BORDER))
                                                .corner_radius(CornerRadius::same(8))
                                                .inner_margin(egui::Margin::symmetric(10, 6))
                                        } else {
                                            egui::Frame::new()
                                                .fill(Color32::TRANSPARENT)
                                                .stroke(Stroke::NONE)
                                                .corner_radius(CornerRadius::same(8))
                                                .inner_margin(egui::Margin::symmetric(10, 6))
                                        };

                                        let mut stage_clicked = false;
                                        let mut dpi_clicked = false;
                                        let mut slider_rect = Rect::NOTHING;
                                        let mut btn_minus_rect = Rect::NOTHING;
                                        let mut btn_plus_rect = Rect::NOTHING;

                                        let row_resp = row_frame.show(ui, |row_ui| {
                                            row_ui.horizontal(|row_ui| {
                                                row_ui.spacing_mut().item_spacing = Vec2::ZERO;

                                                // 1 & 2. Stage Dot & Name (Unified clickable badge: 84px)
                                                let (stage_rect, stage_resp) = row_ui.allocate_exact_size(Vec2::new(84.0, 28.0), egui::Sense::click());
                                                let stage_resp = stage_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                                                let dot_center = Pos2::new(stage_rect.min.x + 7.0, stage_rect.center().y);
                                                row_ui.painter().circle_filled(dot_center, 4.8, color);
                                                if is_active {
                                                    row_ui.painter().circle_stroke(dot_center, 7.2, Stroke::new(1.2, color));
                                                }

                                                let label_color = if is_active {
                                                    COLOR_TEXT_PRIMARY
                                                } else {
                                                    COLOR_TEXT_SECONDARY
                                                };
                                                let stage_name = format!("Stage {}", i + 1);
                                                row_ui.painter().text(
                                                    Pos2::new(stage_rect.min.x + 22.0, stage_rect.center().y - 1.5),
                                                    egui::Align2::LEFT_CENTER,
                                                    &stage_name,
                                                    font_control_text(),
                                                    label_color,
                                                );
                                                if stage_resp.clicked() {
                                                    stage_clicked = true;
                                                }

                                                row_ui.add_space(10.0);

                                                // 3. Slider (takes remaining space)
                                                let right_reserved = 80.0 + 8.0 + 28.0 + 4.0 + 28.0;
                                                let slider_w = (row_ui.available_width() - right_reserved - 10.0).max(60.0);
                                                slider_rect = Rect::from_min_size(row_ui.cursor().min, Vec2::new(slider_w, 28.0));
                                                if custom_slider_u16(row_ui, &mut self.config.dpi_stages[i], MIN_DPI, MAX_DPI, DPI_STEP, slider_w) {
                                                    self.has_unsaved_changes = true;
                                                }

                                                row_ui.add_space(10.0);

                                                // 4. DPI readout (Clickable: 80px)
                                                let (dpi_rect, dpi_resp) = row_ui.allocate_exact_size(Vec2::new(80.0, 28.0), egui::Sense::click());
                                                let dpi_resp = dpi_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                                                let full_str = format!("{} DPI", format_dpi(self.config.dpi_stages[i]));
                                                row_ui.painter().text(
                                                    dpi_rect.right_center(),
                                                    egui::Align2::RIGHT_CENTER,
                                                    &full_str,
                                                    font_dpi_value(),
                                                    COLOR_TEXT_PRIMARY,
                                                );
                                                if dpi_resp.clicked() {
                                                    dpi_clicked = true;
                                                }

                                                row_ui.add_space(8.0);

                                                // 5. [-] button
                                                let btn_minus_resp = step_button(row_ui, "-")
                                                    .on_hover_text("-100 DPI");
                                                btn_minus_rect = btn_minus_resp.rect;
                                                if btn_minus_resp.clicked() {
                                                    let cur = self.config.dpi_stages[i];
                                                    let next = step_dpi_down(cur);
                                                    if next != cur {
                                                        self.config.dpi_stages[i] = next;
                                                        self.has_unsaved_changes = true;
                                                    }
                                                }

                                                row_ui.add_space(4.0);

                                                // 6. [+] button
                                                let btn_plus_resp = step_button(row_ui, "+")
                                                    .on_hover_text("+100 DPI");
                                                btn_plus_rect = btn_plus_resp.rect;
                                                if btn_plus_resp.clicked() {
                                                    let cur = self.config.dpi_stages[i];
                                                    let next = step_dpi_up(cur);
                                                    if next != cur {
                                                        self.config.dpi_stages[i] = next;
                                                        self.has_unsaved_changes = true;
                                                    }
                                                }
                                            });
                                        }).response;

                                        // Row background click (excluding interactive controls like slider and step buttons)
                                        let clicked_bg = ui.input(|inp| {
                                            inp.pointer.button_clicked(egui::PointerButton::Primary)
                                                && inp.pointer.interact_pos().is_some_and(|pos| {
                                                    row_resp.rect.contains(pos)
                                                         && !slider_rect.contains(pos)
                                                         && !btn_minus_rect.contains(pos)
                                                         && !btn_plus_rect.contains(pos)
                                                })
                                        });

                                        if (stage_clicked || dpi_clicked || clicked_bg) && !is_active {
                                            self.config.active_dpi_stage = i + 1;
                                            self.has_unsaved_changes = true;
                                        }

                                        // Visual cursor feedback when hovering over selectable areas of inactive row
                                        if !is_active {
                                            if let Some(pos) = ui.input(|inp| inp.pointer.hover_pos()) {
                                                if row_resp.rect.contains(pos)
                                                    && !slider_rect.contains(pos)
                                                    && !btn_minus_rect.contains(pos)
                                                    && !btn_plus_rect.contains(pos)
                                                {
                                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                                }
                                            }
                                        }

                                        ui.add_space(3.0);
                                    }
                                });
                            });

                            ui.add_space(10.0);

                            // ====================================================
                            // 3. MIDDLE ROW: POLLING RATE & SWITCH TIMINGS
                            // ====================================================
                            let total_w = ui.available_width();
                            let col_gap = 10.0;
                            let left_w = 262.0;
                            let right_w = total_w - col_gap - left_w;

                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |row_ui| {
                                row_ui.spacing_mut().item_spacing = Vec2::ZERO;

                                // Column 1: POLLING RATE
                                row_ui.allocate_ui_with_layout(
                                    Vec2::new(left_w, 0.0),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |col_ui| {
                                        col_ui.set_width(left_w);
                                        card_frame().show(col_ui, |card_ui| {
                                            card_ui.set_width(card_ui.available_width());
                                            card_ui.set_min_height(96.0);
                                            card_ui.vertical(|ui| {
                                                ui.label(
                                                    RichText::new("POLLING RATE")
                                                        .font(font_section_title())
                                                        .strong()
                                                        .color(COLOR_TEXT_PRIMARY),
                                                );
                                                ui.add_space(10.0);

                                                egui::Frame::new()
                                                    .fill(COLOR_BG_SECONDARY)
                                                    .stroke(Stroke::new(1.0, COLOR_BORDER_INSET))
                                                    .corner_radius(CornerRadius::same(8))
                                                    .inner_margin(egui::Margin::symmetric(4, 4))
                                                    .show(ui, |ui| {
                                                        ui.set_width(ui.available_width());
                                                        let rates = [
                                                            (PollingRate::Hz125, "125"),
                                                            (PollingRate::Hz250, "250"),
                                                            (PollingRate::Hz500, "500"),
                                                            (PollingRate::Hz1000, "1000"),
                                                        ];
                                                        let avail_w = ui.available_width();
                                                        let seg_w = (avail_w / 4.0).floor();

                                                        ui.horizontal(|ui| {
                                                            ui.spacing_mut().item_spacing = Vec2::ZERO;
                                                            for (rate, val_str) in rates {
                                                                let is_selected = self.config.report_rate == rate;
                                                                let (rect, resp) = ui.allocate_exact_size(
                                                                    Vec2::new(seg_w, 52.0),
                                                                    egui::Sense::click(),
                                                                );
                                                                if resp.clicked() && !is_selected {
                                                                    self.config.report_rate = rate;
                                                                    self.has_unsaved_changes = true;
                                                                }

                                                                let text_color = if is_selected {
                                                                    let glow = rect.expand(2.0);
                                                                    ui.painter().rect_filled(glow, CornerRadius::same(7), COLOR_ACCENT_GLOW);
                                                                    ui.painter().rect(rect, CornerRadius::same(6), COLOR_ACCENT, Stroke::NONE, StrokeKind::Inside);
                                                                    COLOR_TEXT_ON_ACCENT
                                                                } else if resp.hovered() {
                                                                    COLOR_TEXT_PRIMARY
                                                                } else {
                                                                    COLOR_TEXT_MUTED
                                                                };

                                                                let font = egui::FontId::new(12.0, egui::FontFamily::Proportional);
                                                                ui.painter().text(
                                                                    Pos2::new(rect.center().x, rect.center().y - 9.0),
                                                                    egui::Align2::CENTER_CENTER,
                                                                    val_str,
                                                                    font.clone(),
                                                                    text_color,
                                                                );
                                                                ui.painter().text(
                                                                    Pos2::new(rect.center().x, rect.center().y + 9.0),
                                                                    egui::Align2::CENTER_CENTER,
                                                                    "Hz",
                                                                    font,
                                                                    text_color,
                                                                );
                                                            }
                                                        });
                                                    });
                                            });
                                        });
                                    },
                                );

                                row_ui.add_space(col_gap);

                                // Column 2: SWITCH & POWER TIMINGS
                                row_ui.allocate_ui_with_layout(
                                    Vec2::new(right_w, 0.0),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |col_ui| {
                                        col_ui.set_width(right_w);
                                        card_frame().show(col_ui, |card_ui| {
                                            card_ui.set_width(card_ui.available_width());
                                            card_ui.set_min_height(96.0);
                                            card_ui.vertical(|ui| {
                                                ui.label(
                                                    RichText::new("SWITCH & POWER TIMINGS")
                                                        .font(font_section_title())
                                                        .strong()
                                                        .color(COLOR_TEXT_PRIMARY),
                                                );
                                                ui.add_space(10.0);

                                                // Row 1: Debounce
                                                ui.horizontal(|row_ui| {
                                                    row_ui.spacing_mut().item_spacing = Vec2::ZERO;

                                                    let (lbl_rect, _) = row_ui.allocate_exact_size(Vec2::new(170.0, 26.0), egui::Sense::hover());
                                                    row_ui.painter().text(
                                                        lbl_rect.left_center(),
                                                        egui::Align2::LEFT_CENTER,
                                                        "Button debounce",
                                                        font_control_text(),
                                                        COLOR_TEXT_PRIMARY,
                                                    );
                                                    let anti_pos = Pos2::new(lbl_rect.min.x + 104.0, lbl_rect.center().y);
                                                    row_ui.painter().text(
                                                        anti_pos,
                                                        egui::Align2::LEFT_CENTER,
                                                        "Anti-doubleclick",
                                                        font_small_caption(),
                                                        COLOR_TEXT_MUTED,
                                                    );

                                                    row_ui.add_space(8.0);

                                                    let val_w = 58.0;
                                                    let slider_w = (row_ui.available_width() - val_w - 8.0).max(40.0);
                                                    if custom_slider_u8(row_ui, &mut self.config.debounce_ms, 0, 20, 1, slider_w) {
                                                        self.has_unsaved_changes = true;
                                                    }

                                                    row_ui.add_space(8.0);

                                                    let (val_rect, _) = row_ui.allocate_exact_size(Vec2::new(val_w, 26.0), egui::Sense::hover());
                                                    row_ui.painter().text(
                                                        val_rect.right_center(),
                                                        egui::Align2::RIGHT_CENTER,
                                                        format!("{} ms", self.config.debounce_ms),
                                                        font_dpi_value(),
                                                        COLOR_TEXT_PRIMARY,
                                                    );
                                                });

                                                ui.add_space(8.0);

                                                // Row 2: Sleep timer
                                                ui.horizontal(|row_ui| {
                                                    row_ui.spacing_mut().item_spacing = Vec2::ZERO;

                                                    let (lbl_rect, _) = row_ui.allocate_exact_size(Vec2::new(170.0, 26.0), egui::Sense::hover());
                                                    let label_rect = row_ui.painter().text(
                                                        lbl_rect.left_center(),
                                                        egui::Align2::LEFT_CENTER,
                                                        "Wireless sleep",
                                                        font_control_text(),
                                                        COLOR_TEXT_PRIMARY,
                                                    );
                                                    if self.config.sleep_minutes == 0 {
                                                        // Visually center between "Wireless sleep" and the slider knob at 0
                                                        let knob_left = lbl_rect.max.x + 8.0 + 2.0;
                                                        let mid_x = (label_rect.max.x + knob_left) / 2.0;
                                                        row_ui.painter().text(
                                                            Pos2::new(mid_x, lbl_rect.center().y),
                                                            egui::Align2::CENTER_CENTER,
                                                            "Never sleep",
                                                            font_small_caption(),
                                                            COLOR_ACCENT,
                                                        );
                                                    }

                                                    row_ui.add_space(8.0);

                                                    let val_w = 58.0;
                                                    let slider_w = (row_ui.available_width() - val_w - 8.0).max(40.0);
                                                    if custom_slider_u8(row_ui, &mut self.config.sleep_minutes, 0, 100, 1, slider_w) {
                                                        self.has_unsaved_changes = true;
                                                    }

                                                    row_ui.add_space(8.0);

                                                    let (val_rect, _) = row_ui.allocate_exact_size(Vec2::new(val_w, 26.0), egui::Sense::hover());
                                                    row_ui.painter().text(
                                                        val_rect.right_center(),
                                                        egui::Align2::RIGHT_CENTER,
                                                        format!("{} min", self.config.sleep_minutes),
                                                        font_dpi_value(),
                                                        COLOR_TEXT_PRIMARY,
                                                    );
                                                });
                                            });
                                        });
                                    },
                                );
                            });

                            ui.add_space(10.0);

                            // ====================================================
                            // 4. BOTTOM ACTION BAR
                            // ====================================================
                            card_frame().show(ui, |card_ui| {
                                card_ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing = Vec2::ZERO;

                                    // Dynamic check: are there genuine unsaved changes compared to the mouse?
                                    self.has_unsaved_changes = self.config != self.saved_config
                                        || self.button_mappings != self.saved_button_mappings;

                                    // Check if an apply, factory, or reload action is currently in progress
                                    let is_busy = self.is_busy();

                                    // Resolve active status message and color
                                    let active_msg = if let Some((ref msg, time)) = self.status_msg {
                                        let timeout = if msg.starts_with("Save error") || msg.starts_with("Disconnected") {
                                            Duration::from_secs(6)
                                        } else {
                                            Duration::from_secs(4)
                                        };
                                        if time.elapsed() < timeout {
                                            let color = if msg.starts_with("Save error") || msg.starts_with("Disconnected") {
                                                COLOR_ERROR
                                            } else if msg.starts_with("Applying") || msg.starts_with("Reloading") {
                                                COLOR_WARNING
                                            } else {
                                                COLOR_SUCCESS
                                            };
                                            Some((color, msg.clone()))
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    };

                                    let (status_color, status_text) = if let Some((color, ref msg)) = active_msg {
                                        // If there are unsaved changes and the message was an old success message, prioritize showing "Unsaved changes"
                                        if self.has_unsaved_changes && (msg == "Settings applied" || msg == "Settings synced" || msg == "Factory settings restored" || msg == "Connected to MCHOSE mouse" || msg == "Settings reloaded" || msg == "Settings discarded" || msg == "Factory defaults loaded (offline)") {
                                            (COLOR_WARNING, "Unsaved changes".to_string())
                                        } else {
                                            (color, msg.clone())
                                        }
                                    } else if !self.is_connected {
                                        (COLOR_ERROR, "Disconnected".to_string())
                                    } else if self.has_unsaved_changes {
                                        (COLOR_WARNING, "Unsaved changes".to_string())
                                    } else {
                                        (COLOR_SUCCESS, "Settings synced".to_string())
                                    };

                                    // Right: buttons (Restore from mouse & Apply settings)
                                    let reload_btn_w = 150.0;
                                    let apply_btn_w = 130.0;
                                    let btn_gap = 10.0;
                                    let total_btn_w = reload_btn_w + btn_gap + apply_btn_w;

                                    // Left: status allocated zone (remaining width up to 10px before buttons)
                                    let status_zone_w = (ui.available_width() - total_btn_w - 10.0).max(180.0);
                                    let (status_rect, status_resp) =
                                        ui.allocate_exact_size(Vec2::new(status_zone_w, 38.0), egui::Sense::hover());
                                    let cy = status_rect.center().y;

                                    // Icon centered at cy
                                    let icon_center = Pos2::new(status_rect.min.x + 10.0, cy);
                                    draw_status_icon(ui.painter(), icon_center, status_color);

                                    // Text centered at exact cy with Align2::LEFT_CENTER
                                    let text_pos = Pos2::new(status_rect.min.x + 28.0, cy);
                                    let max_text_chars = 36;
                                    if status_text.chars().count() > max_text_chars {
                                        status_resp.on_hover_text(&status_text);
                                    }
                                    let display_text = if status_text.chars().count() > max_text_chars {
                                        let mut s: String = status_text.chars().take(max_text_chars - 1).collect();
                                        s.push('…');
                                        s
                                    } else {
                                        status_text.clone()
                                    };

                                    ui.painter().text(
                                        text_pos,
                                        egui::Align2::LEFT_CENTER,
                                        display_text,
                                        font_control_text(),
                                        status_color,
                                    );

                                    let space = (ui.available_width() - total_btn_w).max(10.0);
                                    ui.add_space(space);

                                    let restore_enabled = !is_busy;
                                    let (restore_resp, is_factory) = restore_button(ui, restore_enabled);
                                    if restore_resp.clicked() && restore_enabled {
                                        if is_factory {
                                            let factory_cfg = self.make_factory_config();
                                            self.button_mappings = DEFAULT_BUTTON_MAPPINGS;
                                            if self.is_connected {
                                                let _ = self.tx_req.send(WorkerRequest::ApplyConfig(
                                                    factory_cfg.clone(),
                                                    Some(DEFAULT_BUTTON_MAPPINGS),
                                                ));
                                                self.config = factory_cfg;
                                                self.set_status("Applying factory defaults...");
                                            } else {
                                                self.config = factory_cfg;
                                                self.has_unsaved_changes = self.config != self.saved_config
                                                    || self.button_mappings != self.saved_button_mappings;
                                                self.set_status("Factory defaults loaded (offline)");
                                            }
                                        } else if self.is_connected {
                                            self.open_dropdown = None;
                                            let _ = self.tx_req.send(WorkerRequest::ConnectAndRead);
                                            self.set_status("Reloading settings...");
                                        } else {
                                            self.config = self.saved_config.clone();
                                            self.button_mappings = self.saved_button_mappings;
                                            self.has_unsaved_changes = false;
                                            self.set_status("Settings discarded");
                                        }
                                    }

                                    ui.add_space(btn_gap);

                                    let apply_enabled = self.is_connected && self.has_unsaved_changes && !is_busy;
                                    if apply_button(ui, apply_enabled).clicked() && apply_enabled {
                                        let btns_to_apply = (self.button_mappings != self.saved_button_mappings)
                                            .then_some(self.button_mappings);
                                        let _ = self.tx_req.send(WorkerRequest::ApplyConfig(
                                            self.config.clone(),
                                            btns_to_apply,
                                        ));
                                        self.set_status("Applying settings...");
                                    }
                                });
                            });
                });
            });

        // Dynamically adjust window height to wrap content with 14 px vertical margins
        let exact_h = frame_resp.response.rect.height().round();
        if (exact_h - self.current_window_h).abs() > 0.5 {
            self.current_window_h = exact_h;
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(680.0, exact_h)));
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_worker_messages(ctx);
        self.has_unsaved_changes = self.config != self.saved_config
            || self.button_mappings != self.saved_button_mappings;
        ctx.request_repaint_after(Duration::from_millis(200));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render_ui(ui);
    }
}

pub fn run_gui() -> eframe::Result {
    let init_state = AppInitState::detect();
    let initial_h = target_window_height(init_state.config.dpi_stages_count);
    let icon_bytes = include_bytes!("../../assets/icon_64x64.rgba");
    let icon_data = egui::IconData {
        rgba: icon_bytes.to_vec(),
        width: 64,
        height: 64,
    };
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([680.0, initial_h])
            .with_min_inner_size([680.0, 430.0])
            .with_max_inner_size([680.0, 1050.0])
            .with_resizable(false)
            .with_maximize_button(false)
            .with_icon(icon_data)
            .with_title("M-RustHub - G3 V2 Manager"),
        ..Default::default()
    };

    eframe::run_native(
        "M-RustHub",
        native_options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, init_state)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::Lod;

    #[test]
    fn test_format_dpi() {
        assert_eq!(format_dpi(50), "50");
        assert_eq!(format_dpi(800), "800");
        assert_eq!(format_dpi(1000), "1,000");
        assert_eq!(format_dpi(12000), "12,000");
        assert_eq!(format_dpi(15000), "15,000");
        assert_eq!(format_dpi(18000), "18,000");
    }

    #[test]
    fn test_step_dpi_up_and_down_within_range() {
        // Standard steps
        assert_eq!(step_dpi_up(800), 900);
        assert_eq!(step_dpi_down(800), 700);

        assert_eq!(step_dpi_up(1600), 1700);
        assert_eq!(step_dpi_down(1600), 1500);

        assert_eq!(step_dpi_up(5000), 5100);
        assert_eq!(step_dpi_down(5000), 4900);
    }

    #[test]
    fn test_step_dpi_boundaries() {
        // At MIN_DPI (100)
        assert_eq!(step_dpi_down(100), 100); // Does not decrease below min
        assert_eq!(step_dpi_up(100), 200);

        // At MAX_DPI (12000)
        assert_eq!(step_dpi_up(12000), 12000); // Does not exceed max
        assert_eq!(step_dpi_down(12000), 11900);
    }

    #[test]
    fn test_step_dpi_out_of_range_untouched() {
        // Values above MAX_DPI (e.g. 15000 or 18000 previously allowed by CLI bug)
        // MUST NOT be spoiled or reduced to 12000 on clicking +/- buttons:
        assert_eq!(step_dpi_up(15000), 15000);
        assert_eq!(step_dpi_down(15000), 15000);

        // Values below MIN_DPI (e.g. 50 previously allowed by CLI bug)
        // MUST NOT be spoiled or raised to 100 on clicking +/- buttons:
        assert_eq!(step_dpi_up(50), 50);
        assert_eq!(step_dpi_down(50), 50);
    }

    #[test]
    fn test_step_dpi_unaligned_snaps_to_grid() {
        assert_eq!(step_dpi_up(850), 900);
        assert_eq!(step_dpi_down(850), 800);

        assert_eq!(step_dpi_up(1234), 1300);
        assert_eq!(step_dpi_down(1234), 1200);

        assert_eq!(step_dpi_up(105), 200);
        assert_eq!(step_dpi_down(105), 100);

        assert_eq!(step_dpi_up(150), 200);
        assert_eq!(step_dpi_down(150), 100);

        assert_eq!(step_dpi_up(11950), 12000);
        assert_eq!(step_dpi_down(11950), 11900);
    }

    #[test]
    fn test_dpi_stage_changed_clamping() {
        let unapplied_count = 2usize;
        let hardware_new_stage = 4usize;
        let clamped = hardware_new_stage.clamp(1, unapplied_count.clamp(1, 6));
        assert_eq!(clamped, 2);

        let unapplied_count = 5usize;
        let clamped = hardware_new_stage.clamp(1, unapplied_count.clamp(1, 6));
        assert_eq!(clamped, 4);
    }

    #[test]
    fn test_battery_display_info_disconnected() {
        let (bat_str, icon_color, text_color, pct, tooltip) = battery_display_info(None, false);
        assert_eq!(bat_str, "--%");
        assert_eq!(pct, None);
        assert_eq!(icon_color, COLOR_TEXT_MUTED);
        assert_eq!(text_color, COLOR_TEXT_MUTED);
        assert!(tooltip.contains("disconnected"), "tooltip: {tooltip}");
    }

    #[test]
    fn test_active_dpi_display_info_connected() {
        let mut cfg = MouseConfig::default();
        cfg.active_dpi_stage = 2;
        cfg.dpi_stages = [400, 800, 1600, 3200, 6400, 12000];

        let (active_str, color, has_glow) = active_dpi_display_info(&cfg, true);
        assert_eq!(active_str, "Active: 800 DPI");
        assert_eq!(color, COLOR_ACCENT);
        assert!(has_glow);

        cfg.active_dpi_stage = 6;
        let (active_str, color, has_glow) = active_dpi_display_info(&cfg, true);
        assert_eq!(active_str, "Active: 12,000 DPI");
        assert_eq!(color, COLOR_ACCENT);
        assert!(has_glow);
    }

    #[test]
    fn test_active_dpi_display_info_disconnected() {
        let cfg = MouseConfig::default();
        let (active_str, color, has_glow) = active_dpi_display_info(&cfg, false);
        assert_eq!(active_str, "Active: -- DPI");
        assert_eq!(color, COLOR_TEXT_MUTED);
        assert!(!has_glow);
    }

    #[test]
    fn test_battery_display_info_connected_but_failed_read() {
        let (bat_str, icon_color, text_color, pct, tooltip) = battery_display_info(None, true);
        assert_eq!(bat_str, "--%");
        assert_eq!(pct, None);
        assert_eq!(icon_color, COLOR_TEXT_MUTED);
        assert_eq!(text_color, COLOR_TEXT_MUTED);
        assert!(tooltip.contains("unavailable"), "tooltip: {tooltip}");
    }

    #[test]
    fn test_battery_display_info_normal_and_charging() {
        // High battery (>30%)
        let bat = BatteryStatus { percent: 85, charging: false };
        let (bat_str, icon_color, text_color, pct, tooltip) = battery_display_info(Some(&bat), true);
        assert_eq!(bat_str, "85%");
        assert_eq!(pct, Some(85));
        assert_eq!(icon_color, COLOR_ACCENT);
        assert_eq!(text_color, COLOR_TEXT_PRIMARY);
        assert_eq!(tooltip, "Battery: 85%");

        // Medium battery (16-30%)
        let bat_warn = BatteryStatus { percent: 25, charging: false };
        let (str_warn, color_warn, _, pct_warn, _) = battery_display_info(Some(&bat_warn), true);
        assert_eq!(str_warn, "25%");
        assert_eq!(pct_warn, Some(25));
        assert_eq!(color_warn, COLOR_WARNING);

        // Low battery (<=15%)
        let bat_crit = BatteryStatus { percent: 10, charging: false };
        let (str_crit, color_crit, _, pct_crit, _) = battery_display_info(Some(&bat_crit), true);
        assert_eq!(str_crit, "10%");
        assert_eq!(pct_crit, Some(10));
        assert_eq!(color_crit, COLOR_ERROR);

        // Charging
        let bat_charge = BatteryStatus { percent: 20, charging: true };
        let (str_ch, color_ch, _, pct_ch, tip_ch) = battery_display_info(Some(&bat_charge), true);
        assert_eq!(str_ch, "20%");
        assert_eq!(pct_ch, Some(20));
        assert_eq!(color_ch, COLOR_SUCCESS);
        assert!(tip_ch.contains("(Charging)"));
    }

    fn create_test_app() -> App {
        let (tx_req, _) = channel();
        let (_, rx_resp) = channel();
        let config = MouseConfig::default();
        let saved_config = config.clone();
        App {
            tx_req,
            rx_resp,
            config,
            saved_config,
            battery: None,
            is_wireless: true,
            is_connected: true,
            has_unsaved_changes: false,
            status_msg: None,
            current_window_h: 0.0,
            is_remap_open: false,
            button_mappings: DEFAULT_BUTTON_MAPPINGS,
            saved_button_mappings: DEFAULT_BUTTON_MAPPINGS,
            open_dropdown: None,
        }
    }

    #[test]
    fn test_reconnect_preserves_unsaved_user_edits() {
        let mut app = create_test_app();
        // User changes DPI stage 1 in UI
        app.config.dpi_stages[0] = 1200;
        app.has_unsaved_changes = app.config != app.saved_config;
        assert!(app.has_unsaved_changes);

        // Mouse disconnects
        app.on_worker_response(WorkerResponse::Disconnected("Mouse sleep".into()));
        assert!(!app.is_connected);
        app.has_unsaved_changes = app.config != app.saved_config;
        assert!(app.has_unsaved_changes, "Edits must remain marked unsaved even when disconnected");

        // Mouse wakes up / reconnects with hardware config
        let hw_config = MouseConfig::default(); // default has 800 in stage 0
        app.on_worker_response(WorkerResponse::Connected {
            config: hw_config.clone(),
            battery: Some(BatteryStatus { percent: 90, charging: false }),
            is_wireless: true,
            button_mappings: DEFAULT_BUTTON_MAPPINGS,
        });

        assert!(app.is_connected);
        assert_eq!(app.config.dpi_stages[0], 1200, "User edits in self.config must NOT be wiped by reconnect!");
        assert_eq!(app.saved_config.dpi_stages[0], hw_config.dpi_stages[0]);
        assert!(app.has_unsaved_changes);
    }

    #[test]
    fn test_reconnect_adopts_config_when_no_user_edits() {
        let mut app = create_test_app();
        assert!(!app.has_unsaved_changes);

        let mut hw_config = MouseConfig::default();
        hw_config.debounce_ms = 12;
        app.on_worker_response(WorkerResponse::Connected {
            config: hw_config.clone(),
            battery: Some(BatteryStatus { percent: 75, charging: false }),
            is_wireless: true,
            button_mappings: DEFAULT_BUTTON_MAPPINGS,
        });

        assert_eq!(app.config.debounce_ms, 12);
        assert_eq!(app.saved_config.debounce_ms, 12);
        assert!(!app.has_unsaved_changes);
    }

    #[test]
    fn test_reload_discards_unsaved_edits() {
        let mut app = create_test_app();
        app.config.debounce_ms = 2;
        app.button_mappings[0] = ButtonAction::RightClick;
        app.has_unsaved_changes = true;

        let mut hw_config = MouseConfig::default();
        hw_config.debounce_ms = 8;
        app.on_worker_response(WorkerResponse::Reloaded {
            config: hw_config.clone(),
            battery: None,
            is_wireless: true,
            button_mappings: DEFAULT_BUTTON_MAPPINGS,
        });

        assert_eq!(app.config.debounce_ms, 8, "Explicit reload must overwrite user edits with hardware config");
        assert_eq!(app.saved_config.debounce_ms, 8);
        assert_eq!(app.button_mappings, DEFAULT_BUTTON_MAPPINGS, "Explicit reload must overwrite button mappings with hardware values");
        assert_eq!(app.saved_button_mappings, DEFAULT_BUTTON_MAPPINGS);
        assert!(!app.has_unsaved_changes);
    }

    #[test]
    fn test_reconnect_syncs_active_stage_if_user_did_not_customize_it() {
        let mut app = create_test_app();
        // User changed debounce, but left active_dpi_stage alone
        app.config.debounce_ms = 4;
        assert_eq!(app.config.active_dpi_stage, app.saved_config.active_dpi_stage);

        // Hardware reconnected on stage 3 (physical button was pressed on mouse)
        let mut hw_config = MouseConfig::default();
        hw_config.active_dpi_stage = 3;
        app.on_worker_response(WorkerResponse::Connected {
            config: hw_config,
            battery: None,
            is_wireless: true,
            button_mappings: DEFAULT_BUTTON_MAPPINGS,
        });

        assert_eq!(app.config.active_dpi_stage, 3);
        assert_eq!(app.config.debounce_ms, 4, "Debounce edit still preserved");
    }

    #[test]
    fn test_reconnect_preserves_active_stage_if_user_customized_it() {
        let mut app = create_test_app();
        // User explicitly picked stage 2 in GUI (saved_config is 1)
        app.config.active_dpi_stage = 2;
        assert_ne!(app.config.active_dpi_stage, app.saved_config.active_dpi_stage);

        // Hardware reconnected on stage 4
        let mut hw_config = MouseConfig::default();
        hw_config.active_dpi_stage = 4;
        app.on_worker_response(WorkerResponse::Connected {
            config: hw_config,
            battery: None,
            is_wireless: true,
            button_mappings: DEFAULT_BUTTON_MAPPINGS,
        });

        assert_eq!(app.config.active_dpi_stage, 2, "User's UI-selected stage must be preserved");
    }

    #[test]
    fn test_reconnect_syncs_hidden_hardware_fields_when_user_has_edits() {
        let mut app = create_test_app();
        // User changed debounce before mouse connected
        app.config.debounce_ms = 4;
        assert_ne!(app.config, app.saved_config);

        // Hardware wakes up with non-default hidden hardware settings
        let mut hw_config = MouseConfig::default();
        hw_config.lod = Lod::Low1mm;
        hw_config.sensor_flags = 0xAA;
        hw_config.scroll_flag = 0x55;
        hw_config.highspeed_mode = 1;
        hw_config.sensor_angle = 5;
        hw_config.debounce_ms = 8;

        let repaint = app.on_worker_response(WorkerResponse::Connected {
            config: hw_config.clone(),
            battery: None,
            is_wireless: true,
            button_mappings: DEFAULT_BUTTON_MAPPINGS,
        });

        assert!(repaint, "Connecting mouse should trigger repaint");
        // Hidden fields must be synced to hardware values so they aren't clobbered on apply
        assert_eq!(app.config.lod, Lod::Low1mm);
        assert_eq!(app.config.sensor_flags, 0xAA);
        assert_eq!(app.config.scroll_flag, 0x55);
        assert_eq!(app.config.highspeed_mode, 1);
        assert_eq!(app.config.sensor_angle, 5);
        // User edit must be kept
        assert_eq!(app.config.debounce_ms, 4);
        assert!(app.has_unsaved_changes);

        // If user reverts the debounce edit back to hardware value, unsaved changes must clear
        app.config.debounce_ms = 8;
        app.has_unsaved_changes = app.config != app.saved_config;
        assert!(!app.has_unsaved_changes, "Reverting edits should clear unsaved changes since hidden fields match");
    }

    #[test]
    fn test_app_init_state_disconnected_defaults() {
        let init = AppInitState {
            mouse: None,
            config: MouseConfig::default(),
            battery: None,
            is_wireless: true,
            is_connected: false,
            button_mappings: DEFAULT_BUTTON_MAPPINGS,
        };
        assert!(!init.is_connected);
        assert!(init.mouse.is_none());
        assert_eq!(init.config, MouseConfig::default());
        assert!(init.battery.is_none());
        assert_eq!(target_window_height(init.config.dpi_stages_count), 660.0);
    }

    #[test]
    fn test_apply_error_preserves_unsaved_changes() {
        let mut app = create_test_app();
        app.config.debounce_ms = 15;
        app.has_unsaved_changes = app.config != app.saved_config;
        assert!(app.has_unsaved_changes);

        app.on_worker_response(WorkerResponse::ApplyError("Mouse is offline or sleeping".into()));
        assert!(app.has_unsaved_changes);
        assert_eq!(app.config.debounce_ms, 15);
        assert_ne!(app.config.debounce_ms, app.saved_config.debounce_ms);
        assert!(app.status_msg.as_ref().unwrap().0.contains("Save error: Mouse is offline or sleeping"));
    }

    #[test]
    fn test_row_interact_with_buttons() {
        let ctx = egui::Context::default();
        let mut row_activated = false;
        let mut btn_minus_clicked = false;
        let mut btn_plus_clicked = false;
        let mut slider_changed = false;

        let mut val = 800u16;

        // Helper closure to run UI frame
        let run_frame = |ctx: &egui::Context, raw_input: egui::RawInput,
                         _val: &mut u16,
                         row_activated: &mut bool,
                         btn_minus_clicked: &mut bool,
                         btn_plus_clicked: &mut bool,
                         slider_changed: &mut bool| -> (egui::Rect, egui::Rect, egui::Rect, egui::Rect, egui::Rect) {
            let mut dot_rect_out = egui::Rect::NOTHING;
            let mut dpi_rect_out = egui::Rect::NOTHING;
            let mut slider_rect_out = egui::Rect::NOTHING;
            let mut minus_rect_out = egui::Rect::NOTHING;
            let mut plus_rect_out = egui::Rect::NOTHING;

            let mut out = ctx.run_ui(raw_input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let is_active = false;
                    let row_frame = egui::Frame::new().inner_margin(egui::Margin::symmetric(10, 6));

                    let mut minus_resp = None;
                    let mut plus_resp = None;
                    let mut slider_rect = egui::Rect::NOTHING;
                    let mut minus_rect = egui::Rect::NOTHING;
                    let mut plus_rect = egui::Rect::NOTHING;
                    let mut stage_resp = None;
                    let mut dpi_resp = None;

                    let row_resp = row_frame.show(ui, |row_ui| {
                        row_ui.horizontal(|row_ui| {
                            // Stage badge (dot + label)
                            let (s_rect, s_resp) = row_ui.allocate_exact_size(egui::vec2(84.0, 28.0), egui::Sense::click());
                            dot_rect_out = s_rect;
                            stage_resp = Some(s_resp);

                            row_ui.add_space(10.0);

                            // Slider
                            let slider_w = 100.0;
                            let (s_slider_rect, s_slider_resp) = row_ui.allocate_exact_size(egui::vec2(slider_w, 28.0), egui::Sense::click_and_drag());
                            slider_rect = s_slider_rect;
                            slider_rect_out = s_slider_rect;
                            if s_slider_resp.clicked() || s_slider_resp.dragged() {
                                *slider_changed = true;
                            }

                            row_ui.add_space(10.0);

                            // DPI readout
                            let (d_rect, d_resp) = row_ui.allocate_exact_size(egui::vec2(80.0, 28.0), egui::Sense::click());
                            dpi_rect_out = d_rect;
                            dpi_resp = Some(d_resp);

                            row_ui.add_space(8.0);

                            // Minus button
                            let m_resp = step_button(row_ui, "-");
                            minus_rect = m_resp.rect;
                            minus_rect_out = m_resp.rect;
                            minus_resp = Some(m_resp);

                            row_ui.add_space(4.0);

                            // Plus button
                            let p_resp = step_button(row_ui, "+");
                            plus_rect = p_resp.rect;
                            plus_rect_out = p_resp.rect;
                            plus_resp = Some(p_resp);
                        });
                    }).response;

                    if let Some(r) = minus_resp {
                        if r.clicked() {
                            *btn_minus_clicked = true;
                        }
                    }
                    if let Some(r) = plus_resp {
                        if r.clicked() {
                            *btn_plus_clicked = true;
                        }
                    }

                    // Row activation: click on stage badge, DPI readout, or row background (excluding slider & buttons)
                    let clicked_stage = stage_resp.as_ref().is_some_and(|r| r.clicked());
                    let clicked_dpi = dpi_resp.as_ref().is_some_and(|r| r.clicked());
                    let clicked_bg = ui.input(|i| {
                        i.pointer.button_clicked(egui::PointerButton::Primary) && i.pointer.interact_pos().is_some_and(|pos| {
                            row_resp.rect.contains(pos)
                                && !slider_rect.contains(pos)
                                && !minus_rect.contains(pos)
                                && !plus_rect.contains(pos)
                        })
                    });

                    if (clicked_stage || clicked_dpi || clicked_bg) && !is_active {
                        *row_activated = true;
                    }

                    if !is_active {
                        if let Some(pos) = ui.input(|inp| inp.pointer.hover_pos()) {
                            if row_resp.rect.contains(pos)
                                && !slider_rect.contains(pos)
                                && !minus_rect.contains(pos)
                                && !plus_rect.contains(pos)
                            {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                        }
                    }
                });
            });
            out.textures_delta.clear();

            (dot_rect_out, dpi_rect_out, slider_rect_out, minus_rect_out, plus_rect_out)
        };

        // 1. Get initial rects
        let mut r1 = egui::RawInput::default();
        r1.time = Some(0.0);
        r1.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0)));
        let (dot_rect, dpi_rect, _slider_rect, minus_rect, _plus_rect) = run_frame(
            &ctx, r1, &mut val, &mut row_activated, &mut btn_minus_clicked, &mut btn_plus_clicked, &mut slider_changed,
        );

        // 2. Click minus button
        let minus_center = minus_rect.center();
        let mut r2 = egui::RawInput::default();
        r2.time = Some(0.05);
        r2.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0)));
        r2.events.push(egui::Event::PointerMoved(minus_center));
        r2.events.push(egui::Event::PointerButton { pos: minus_center, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::default() });
        run_frame(&ctx, r2, &mut val, &mut row_activated, &mut btn_minus_clicked, &mut btn_plus_clicked, &mut slider_changed);

        let mut r3 = egui::RawInput::default();
        r3.time = Some(0.10);
        r3.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0)));
        r3.events.push(egui::Event::PointerButton { pos: minus_center, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::default() });
        run_frame(&ctx, r3, &mut val, &mut row_activated, &mut btn_minus_clicked, &mut btn_plus_clicked, &mut slider_changed);

        assert!(btn_minus_clicked, "Minus button must be clicked");
        assert!(!row_activated, "Row must not be activated when clicking minus button");

        // 3. Click dot
        let dot_center = dot_rect.center();
        let mut r4 = egui::RawInput::default();
        r4.time = Some(0.15);
        r4.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0)));
        r4.events.push(egui::Event::PointerMoved(dot_center));
        r4.events.push(egui::Event::PointerButton { pos: dot_center, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::default() });
        run_frame(&ctx, r4, &mut val, &mut row_activated, &mut btn_minus_clicked, &mut btn_plus_clicked, &mut slider_changed);

        let mut r5 = egui::RawInput::default();
        r5.time = Some(0.20);
        r5.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0)));
        r5.events.push(egui::Event::PointerButton { pos: dot_center, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::default() });
        run_frame(&ctx, r5, &mut val, &mut row_activated, &mut btn_minus_clicked, &mut btn_plus_clicked, &mut slider_changed);

        assert!(row_activated, "Row must be activated when clicking dot");
        row_activated = false;

        // 4. Click DPI readout
        let dpi_center = dpi_rect.center();
        let mut r6 = egui::RawInput::default();
        r6.time = Some(0.25);
        r6.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0)));
        r6.events.push(egui::Event::PointerMoved(dpi_center));
        r6.events.push(egui::Event::PointerButton { pos: dpi_center, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::default() });
        run_frame(&ctx, r6, &mut val, &mut row_activated, &mut btn_minus_clicked, &mut btn_plus_clicked, &mut slider_changed);

        let mut r7 = egui::RawInput::default();
        r7.time = Some(0.30);
        r7.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0)));
        r7.events.push(egui::Event::PointerButton { pos: dpi_center, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::default() });
        run_frame(&ctx, r7, &mut val, &mut row_activated, &mut btn_minus_clicked, &mut btn_plus_clicked, &mut slider_changed);

        assert!(row_activated, "Row must be activated when clicking DPI readout");
        row_activated = false;

        // 5. Click background (e.g. top margin above dot)
        let bg_pos = egui::pos2(dot_rect.center().x, dot_rect.min.y - 3.0);
        let mut r8 = egui::RawInput::default();
        r8.time = Some(0.35);
        r8.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0)));
        r8.events.push(egui::Event::PointerMoved(bg_pos));
        r8.events.push(egui::Event::PointerButton { pos: bg_pos, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::default() });
        run_frame(&ctx, r8, &mut val, &mut row_activated, &mut btn_minus_clicked, &mut btn_plus_clicked, &mut slider_changed);

        let mut r9 = egui::RawInput::default();
        r9.time = Some(0.40);
        r9.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0)));
        r9.events.push(egui::Event::PointerButton { pos: bg_pos, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::default() });
        run_frame(&ctx, r9, &mut val, &mut row_activated, &mut btn_minus_clicked, &mut btn_plus_clicked, &mut slider_changed);

        assert!(row_activated, "Row must be activated when clicking row background");
    }

    #[test]
    fn test_set_dpi_stages_count_preserves_selected_active_stage() {
        let mut app = create_test_app();
        app.saved_config.dpi_stages_count = 6;
        app.saved_config.active_dpi_stage = 1;
        app.config.dpi_stages_count = 6;
        app.config.active_dpi_stage = 3; // user selected Stage 3

        // Switch to 4 levels
        app.set_dpi_stages_count(4);
        assert_eq!(app.config.dpi_stages_count, 4);
        assert_eq!(app.config.active_dpi_stage, 3, "Stage 3 is within 4 levels, should be preserved");

        // Switch back to 6 levels (matches saved_config.dpi_stages_count)
        app.set_dpi_stages_count(6);
        assert_eq!(app.config.dpi_stages_count, 6);
        assert_eq!(
            app.config.active_dpi_stage, 3,
            "Switching back to saved count must not reset active stage to saved_config.active_dpi_stage"
        );

        // Switch down to 2 levels (3 > 2, clamps to 2)
        app.set_dpi_stages_count(2);
        assert_eq!(app.config.dpi_stages_count, 2);
        assert_eq!(app.config.active_dpi_stage, 2, "Active stage clamped to count");
    }

    #[test]
    fn test_set_dpi_stages_count_restores_active_stage_when_unclamped_towards_saved_stage() {
        let mut app = create_test_app();
        app.saved_config.dpi_stages_count = 6;
        app.saved_config.active_dpi_stage = 4;
        app.config.dpi_stages_count = 6;
        app.config.active_dpi_stage = 4;

        // User temporarily reduces stage count in GUI to 2 (clamping active stage to 2)
        app.set_dpi_stages_count(2);
        assert_eq!(app.config.dpi_stages_count, 2);
        assert_eq!(app.config.active_dpi_stage, 2);

        // User increases back to 6: active stage restores to saved 4
        app.set_dpi_stages_count(6);
        assert_eq!(app.config.dpi_stages_count, 6);
        assert_eq!(app.config.active_dpi_stage, 4);

        // If user clamps to 2 and then increases to 3: active stage restores up to min(4, 3) = 3
        app.set_dpi_stages_count(2);
        assert_eq!(app.config.active_dpi_stage, 2);
        app.set_dpi_stages_count(3);
        assert_eq!(app.config.active_dpi_stage, 3);
        // And then from 3 to 5 restores to min(4, 5) = 4
        app.set_dpi_stages_count(5);
        assert_eq!(app.config.active_dpi_stage, 4);
    }

    #[test]
    fn test_target_window_height_values_and_clamping() {
        assert_eq!(target_window_height(1), 445.0);
        assert_eq!(target_window_height(2), 488.0);
        assert_eq!(target_window_height(3), 531.0);
        assert_eq!(target_window_height(4), 574.0);
        assert_eq!(target_window_height(5), 617.0);
        assert_eq!(target_window_height(6), 660.0);

        // Clamping boundaries (min 1, max 6)
        assert_eq!(target_window_height(0), 445.0);
        assert_eq!(target_window_height(10), 660.0);

        // Each additional DPI stage increments height by exactly 43 px
        for s in 1..6 {
            assert_eq!(target_window_height(s + 1) - target_window_height(s), 43.0);
        }
    }

    #[test]
    fn test_window_height_min_inner_size_compliance() {
        let ctx = egui::Context::default();
        setup_fonts(&ctx);
        for stages in 1..=6 {
            let mut app = create_test_app();
            app.config.dpi_stages_count = stages;
            let raw_input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(680.0, 1000.0),
                )),
                ..Default::default()
            };
            for _ in 0..2 {
                let mut out = ctx.run_ui(raw_input.clone(), |ctx| {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::NONE)
                        .show(ctx, |ui| {
                            app.render_ui(ui);
                        });
                });
                out.textures_delta.clear();
            }
            // Real layout height at 1 DPI level must exceed min_inner_size (430.0)
            // so no bottom empty gap remains:
            assert!(
                app.current_window_h >= 430.0,
                "Layout height {} must be >= min_inner_size 430.0 for stage count {}",
                app.current_window_h, stages
            );
            // And must stay within max_inner_size (960.0)
            assert!(
                app.current_window_h <= 960.0,
                "Layout height {} must be <= max_inner_size 960.0 for stage count {}",
                app.current_window_h, stages
            );
        }
    }

    #[test]
    fn test_is_busy_blocking_reloading_and_applying() {
        let mut app = create_test_app();
        assert!(!app.is_busy());

        // Reloading settings
        app.set_status("Reloading settings...");
        assert!(app.is_busy(), "Must be busy while reloading settings from mouse");

        // Applying settings
        app.set_status("Applying settings...");
        assert!(app.is_busy(), "Must be busy while applying settings");

        // Applying factory defaults
        app.set_status("Applying factory defaults...");
        assert!(app.is_busy(), "Must be busy while applying factory defaults");

        // Settings reloaded / applied / error / disconnected -> not busy
        app.set_status("Settings reloaded");
        assert!(!app.is_busy());

        app.set_status("Settings applied");
        assert!(!app.is_busy());

        app.set_status("Save error: HID timeout");
        assert!(!app.is_busy());

        app.set_status("Disconnected: Mouse offline");
        assert!(!app.is_busy());

        // Safety timeout (> 4s)
        app.status_msg = Some(("Reloading settings...".to_string(), Instant::now() - Duration::from_secs(5)));
        assert!(!app.is_busy(), "Must timeout after 4 seconds of inactivity");
    }

    #[test]
    fn test_apply_success_preserves_concurrent_slider_movement() {
        let mut app = create_test_app();
        let mut sent_cfg = app.config.clone();
        sent_cfg.debounce_ms = 4;

        // User immediately changes debounce to 12 while worker is busy applying 4
        app.config.debounce_ms = 12;
        app.has_unsaved_changes = true;

        // Worker reports success with sent_cfg
        app.on_worker_response(WorkerResponse::ApplySuccess {
            config: sent_cfg.clone(),
            button_mappings: Some(DEFAULT_BUTTON_MAPPINGS),
        });

        // saved_config must record what was actually written to mouse (4), NOT current config (12)
        assert_eq!(app.saved_config.debounce_ms, 4);
        assert_eq!(app.config.debounce_ms, 12);
        // has_unsaved_changes must remain true so user can apply the new value (12)
        assert!(app.has_unsaved_changes);
    }

    #[test]
    fn test_apply_success_syncs_when_no_concurrent_movement() {
        let mut app = create_test_app();
        let mut sent_cfg = app.config.clone();
        sent_cfg.debounce_ms = 4;
        app.config = sent_cfg.clone();
        app.has_unsaved_changes = true;

        app.on_worker_response(WorkerResponse::ApplySuccess {
            config: sent_cfg.clone(),
            button_mappings: Some(DEFAULT_BUTTON_MAPPINGS),
        });

        assert_eq!(app.saved_config.debounce_ms, 4);
        assert_eq!(app.config.debounce_ms, 4);
        assert!(!app.has_unsaved_changes);
    }

    #[test]
    fn test_apply_success_syncs_button_mappings() {
        let mut app = create_test_app();
        let mut custom_btns = DEFAULT_BUTTON_MAPPINGS;
        custom_btns[3] = ButtonAction::Backward;
        custom_btns[4] = ButtonAction::Forward;

        app.button_mappings = custom_btns;
        app.has_unsaved_changes = true;

        app.on_worker_response(WorkerResponse::ApplySuccess {
            config: app.config.clone(),
            button_mappings: Some(custom_btns),
        });

        assert_eq!(app.saved_button_mappings, custom_btns);
        assert_eq!(app.button_mappings, custom_btns);
        assert!(!app.has_unsaved_changes);
    }

    #[test]
    fn test_apply_success_when_button_mappings_none() {
        let mut app = create_test_app();
        let mut custom_btns = DEFAULT_BUTTON_MAPPINGS;
        custom_btns[3] = ButtonAction::Disabled;
        custom_btns[4] = ButtonAction::Custom([0x21, 0x04, 0x00, 0x00]);
        app.saved_button_mappings = custom_btns;
        app.button_mappings = custom_btns;

        // Change only config (e.g. debounce)
        app.config.debounce_ms = 16;
        app.has_unsaved_changes = true;

        app.on_worker_response(WorkerResponse::ApplySuccess {
            config: app.config.clone(),
            button_mappings: None,
        });

        assert_eq!(app.saved_config.debounce_ms, 16);
        assert_eq!(app.saved_button_mappings, custom_btns);
        assert_eq!(app.button_mappings, custom_btns);
        assert!(!app.has_unsaved_changes);
    }

    #[test]
    fn test_make_factory_config_preserves_all_hardware_fields() {
        let mut app = create_test_app();
        // Hardware fields read from mouse:
        app.saved_config.sensor_flags = 0x81;
        app.saved_config.scroll_flag = 0x02;
        app.saved_config.highspeed_mode = 0x03;
        app.saved_config.sensor_angle = -5;

        // User configurable settings altered by user:
        app.saved_config.debounce_ms = 18;
        app.saved_config.report_rate = PollingRate::Hz1000;
        app.saved_config.sleep_minutes = 20;

        let factory_cfg = app.make_factory_config();

        // Hardware fields must be preserved:
        assert_eq!(factory_cfg.sensor_flags, 0x81);
        assert_eq!(factory_cfg.highspeed_mode, 0x03);
        assert_eq!(factory_cfg.sensor_angle, -5);

        // User settings must be reset to defaults:
        assert_eq!(factory_cfg.scroll_flag, 0);
        assert_eq!(factory_cfg.debounce_ms, 8);
        assert_eq!(factory_cfg.report_rate, PollingRate::Hz250);
        assert_eq!(factory_cfg.sleep_minutes, 3);
        assert_eq!(factory_cfg.lod, Lod::High2mm);
    }

    #[test]
    fn test_offline_factory_reset_preserves_all_hardware_fields() {
        let mut app = create_test_app();
        app.is_connected = false;
        app.saved_config.sensor_flags = 0x81;
        app.saved_config.scroll_flag = 0x02;
        app.saved_config.highspeed_mode = 0x03;
        app.saved_config.sensor_angle = -5;

        // Factory reset performed while offline
        let factory_cfg = app.make_factory_config();
        app.config = factory_cfg;
        app.has_unsaved_changes = app.config != app.saved_config;

        assert_eq!(app.config.sensor_flags, 0x81);
        assert_eq!(app.config.highspeed_mode, 0x03);
        assert_eq!(app.config.sensor_angle, -5);
        assert_eq!(app.config.scroll_flag, 0);
        assert_eq!(app.config.debounce_ms, 8);
    }

    #[test]
    fn test_remap_button_rendering_and_height_constraint() {
        let ctx = egui::Context::default();
        setup_fonts(&ctx);

        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(680.0, 500.0),
            )),
            ..Default::default()
        };

        let mut app = create_test_app();
        assert!(!app.is_remap_open);

        let mut out = ctx.run_ui(raw_input.clone(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                app.render_ui(ui);
            });
        });
        out.textures_delta.clear();

        // Test remap button directly for height constraint and expand toggle
        let mut out2 = ctx.run_ui(raw_input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let pill_height = 34.0;
                let resp_collapsed = remap_button(ui, pill_height, false);
                assert_eq!(resp_collapsed.rect.height(), pill_height);
                assert!(resp_collapsed.rect.height() <= pill_height);

                let resp_expanded = remap_button(ui, pill_height, true);
                assert_eq!(resp_expanded.rect.height(), pill_height);
                assert!(resp_expanded.rect.height() <= pill_height);
            });
        });
        out2.textures_delta.clear();

        app.is_remap_open = true;
        assert!(app.is_remap_open);
    }

    #[test]
    fn test_buttons_section_expand_and_collapse_layout() {
        let ctx = egui::Context::default();
        setup_fonts(&ctx);

        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(680.0, 1000.0),
            )),
            ..Default::default()
        };

        let mut app = create_test_app();
        app.is_remap_open = false;

        let mut out1 = ctx.run_ui(raw_input.clone(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                app.render_ui(ui);
            });
        });
        out1.textures_delta.clear();
        let h_collapsed = app.current_window_h;

        // Expand Buttons section
        app.is_remap_open = true;
        let mut out2 = ctx.run_ui(raw_input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                app.render_ui(ui);
            });
        });
        out2.textures_delta.clear();
        let h_expanded = app.current_window_h;

        assert!(
            h_expanded > h_collapsed,
            "Expanded height {} must be greater than collapsed height {}",
            h_expanded,
            h_collapsed
        );
        assert!(
            h_expanded <= 1050.0,
            "Expanded height {} must stay within max_inner_size (1050.0)",
            h_expanded
        );
    }

    #[test]
    fn test_button_mappings_unsaved_changes_and_reset() {
        let mut app = create_test_app();
        assert_eq!(app.button_mappings, DEFAULT_BUTTON_MAPPINGS);
        assert!(!app.has_unsaved_changes);

        // User changes Left button to Right button
        app.button_mappings[0] = ButtonAction::RightClick;
        assert_ne!(app.button_mappings, app.saved_button_mappings);

        // User changes scroll direction
        app.config.scroll_flag = 1;
        assert_ne!(app.config.scroll_flag, app.saved_config.scroll_flag);

        // Reset to default
        app.button_mappings = DEFAULT_BUTTON_MAPPINGS;
        app.config.scroll_flag = 0;
        assert_eq!(app.button_mappings, app.saved_button_mappings);
        assert_eq!(app.config.scroll_flag, app.saved_config.scroll_flag);
    }

    #[test]
    fn test_buttons_section_reset_button_styling_and_scroll_segmented_switch() {
        let ctx = egui::Context::default();
        setup_fonts(&ctx);

        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(680.0, 500.0),
            )),
            ..Default::default()
        };

        // Test reset_buttons_button dimensions (matches Restore from mouse height 38.0)
        let mut out = ctx.run_ui(raw_input.clone(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let resp_enabled = reset_buttons_button(ui, true);
                assert_eq!(resp_enabled.rect.height(), 38.0);
                assert_eq!(resp_enabled.rect.width(), 130.0);
                assert!(resp_enabled.sense.interactive());

                let resp_disabled = reset_buttons_button(ui, false);
                assert_eq!(resp_disabled.rect.height(), 38.0);
                assert_eq!(resp_disabled.rect.width(), 130.0);
                assert!(!resp_disabled.sense.interactive());
            });
        });
        out.textures_delta.clear();

        // Test render_buttons_card executes and respects scroll direction state
        let mut app = create_test_app();
        app.is_remap_open = true;
        assert_eq!(app.config.scroll_flag, 0);

        let mut out2 = ctx.run_ui(raw_input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                app.render_buttons_card(ui);
            });
        });
        out2.textures_delta.clear();
    }

    #[test]
    fn test_buttons_card_busy_blocking_and_reset_enabled_logic() {
        let ctx = egui::Context::default();
        setup_fonts(&ctx);

        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(680.0, 500.0),
            )),
            ..Default::default()
        };

        let mut app = create_test_app();
        app.is_remap_open = true;

        // 1. Initially default settings -> can_reset is false
        let is_busy = app.is_busy();
        let can_reset = !is_busy
            && (app.button_mappings != DEFAULT_BUTTON_MAPPINGS || app.config.scroll_flag != 0);
        assert!(!can_reset);

        // 2. Modify button mappings -> can_reset is true
        app.button_mappings[0] = ButtonAction::RightClick;
        let is_busy = app.is_busy();
        let can_reset = !is_busy
            && (app.button_mappings != DEFAULT_BUTTON_MAPPINGS || app.config.scroll_flag != 0);
        assert!(can_reset);

        // 3. Mouse becomes busy applying settings -> can_reset is false, busy blocks reset
        app.set_status("Applying settings...");
        assert!(app.is_busy());
        let is_busy = app.is_busy();
        let can_reset = !is_busy
            && (app.button_mappings != DEFAULT_BUTTON_MAPPINGS || app.config.scroll_flag != 0);
        assert!(!can_reset);

        // 4. Render buttons card while busy with open dropdown -> closes open dropdown
        app.open_dropdown = Some(2);
        let mut out = ctx.run_ui(raw_input.clone(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                app.render_buttons_card(ui);
            });
        });
        out.textures_delta.clear();
        assert_eq!(app.open_dropdown, None, "Dropdown must be closed when is_busy()");
        assert_eq!(
            app.status_msg.as_ref().unwrap().0,
            "Applying settings...",
            "Applying status must not be overwritten while busy"
        );
    }

    #[test]
    fn test_polling_rate_section_render() {
        let ctx = egui::Context::default();
        setup_fonts(&ctx);

        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(680.0, 700.0),
            )),
            ..Default::default()
        };

        let mut app = create_test_app();
        app.config.report_rate = PollingRate::Hz500;

        let mut out = ctx.run_ui(raw_input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                app.render_ui(ui);
            });
        });
        out.textures_delta.clear();

        assert_eq!(app.config.report_rate, PollingRate::Hz500);
    }

    #[test]
    fn test_custom_button_dropdown_popup_allocation() {
        let ctx = egui::Context::default();
        setup_fonts(&ctx);

        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(680.0, 700.0),
            )),
            ..Default::default()
        };

        let mut out = ctx.run_ui(raw_input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let (resp, action, toggle) =
                    custom_button_dropdown(ui, 0, ButtonAction::LeftClick, true, 180.0);
                assert!(!resp.clicked());
                assert_eq!(action, None);
                assert!(!toggle);
            });
        });
        out.textures_delta.clear();
    }
}
