//! Custom sharp geometric widgets with eclectic neon highlights.

use crate::theme::Theme;
use egui::{Color32, Frame, Margin, Pos2, Rect, Rounding, Stroke, Ui, Vec2};

/// Renders a sharp rectangular badge with bright neon styling.
pub fn neon_badge(ui: &mut Ui, text: &str, color: Color32) {
    let padding = Vec2::new(8.0, 3.0);
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        egui::FontId::monospace(11.0),
        color,
    );
    let rect = ui.allocate_space(galley.size() + padding * 2.0).1;

    ui.painter().rect(
        rect,
        Rounding::ZERO,
        Color32::from_rgba_premultiplied(color.r() / 8, color.g() / 8, color.b() / 8, 40),
        Stroke::new(1.0, color),
    );

    let text_pos = rect.min + padding;
    ui.painter().galley(text_pos, galley, color);
}

/// Renders a sharp horizontal neon gauge (0.0 to 1.0) with numeric readout.
pub fn neon_gauge(ui: &mut Ui, fraction: f32, label: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.monospace(egui::RichText::new(label).size(11.0).color(Theme::TEXT_MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let pct = (fraction.clamp(0.0, 1.0) * 100.0) as u32;
            ui.monospace(egui::RichText::new(format!("{pct}%")).size(11.0).color(color));
        });
    });

    let height = 8.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), egui::Sense::hover());

    // Track
    ui.painter().rect_filled(rect, Rounding::ZERO, Color32::from_rgb(0x18, 0x18, 0x1c));
    ui.painter().rect_stroke(rect, Rounding::ZERO, Stroke::new(1.0, Theme::BORDER));

    // Neon Fill
    let fill_width = rect.width() * fraction.clamp(0.0, 1.0);
    if fill_width > 0.0 {
        let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_width, height));
        ui.painter().rect_filled(fill_rect, Rounding::ZERO, color);
    }
}

/// Renders a scrollable monospace code/log area whose height is a percentage
/// of total output lines, capped from above.
pub fn scrollable_code_area(ui: &mut Ui, code: &str, max_lines_percent: f32) {
    let line_count = code.lines().count().max(1);
    let row_height = 15.0;
    let full_content_height = line_count as f32 * row_height;
    // Area size = percentage of output with upper cap
    let target_height = (full_content_height * max_lines_percent).clamp(55.0, 240.0);

    Frame::none()
        .fill(Theme::BG_CODE)
        .stroke(Stroke::new(1.0, Theme::BORDER))
        .inner_margin(Margin::same(8.0))
        .rounding(Rounding::ZERO)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(target_height)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    for line in code.lines() {
                        render_syntax_line(ui, line);
                    }
                });
        });
}

/// Minimal fast syntax highlighter for JSON and logs in the code editor style.
fn render_syntax_line(ui: &mut Ui, line: &str) {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    let indent_str = " ".repeat(indent);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        if !indent_str.is_empty() {
            ui.monospace(egui::RichText::new(&indent_str).size(11.0));
        }

        if let Some((key, val)) = trimmed.split_once(':') {
            ui.monospace(
                egui::RichText::new(key)
                    .size(11.0)
                    .color(Color32::from_rgb(0x9c, 0xdc, 0xfe)), // VSCode property light blue
            );
            ui.monospace(egui::RichText::new(":").size(11.0).color(Theme::TEXT_MUTED));
            
            let val_trimmed = val.trim();
            let val_color = if val_trimmed.starts_with('"') {
                Color32::from_rgb(0xce, 0x91, 0x78) // String orange-brown
            } else if val_trimmed.parse::<f64>().is_ok() {
                Color32::from_rgb(0xb5, 0xce, 0xa8) // Number light green
            } else if val_trimmed == "true" || val_trimmed == "false" || val_trimmed == "null" {
                Color32::from_rgb(0x56, 0x9c, 0xd6) // Keyword blue
            } else if val_trimmed.contains("ok") || val_trimmed.contains("success") {
                Theme::NEON_GREEN
            } else if val_trimmed.contains("error") || val_trimmed.contains("fail") {
                Theme::NEON_MAGENTA
            } else {
                Theme::TEXT_BRIGHT
            };
            ui.monospace(egui::RichText::new(format!(" {val_trimmed}")).size(11.0).color(val_color));
        } else if trimmed.starts_with("//") || trimmed.starts_with('#') {
            ui.monospace(egui::RichText::new(trimmed).size(11.0).color(Theme::TEXT_DIM));
        } else if trimmed.contains("ERR") || trimmed.contains("error") {
            ui.monospace(egui::RichText::new(trimmed).size(11.0).color(Theme::NEON_MAGENTA));
        } else if trimmed.contains("OK") || trimmed.contains("done") {
            ui.monospace(egui::RichText::new(trimmed).size(11.0).color(Theme::NEON_GREEN));
        } else {
            ui.monospace(egui::RichText::new(trimmed).size(11.0).color(Theme::TEXT_BRIGHT));
        }
    });
}

/// Mini sparkline showing historical pulse values with neon color.
pub fn pulse_sparkline(ui: &mut Ui, values: &[f32], color: Color32) {
    let height = 24.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), egui::Sense::hover());

    ui.painter().rect_filled(rect, Rounding::ZERO, Color32::from_rgb(0x1a, 0x1a, 0x1e));
    ui.painter().rect_stroke(rect, Rounding::ZERO, Stroke::new(1.0, Theme::BORDER));

    if values.len() < 2 {
        return;
    }

    let min_val = values.iter().copied().fold(f32::INFINITY, f32::min);
    let max_val = values.iter().copied().fold(f32::NEG_INFINITY, f32::max).max(min_val + 0.001);

    let step_x = rect.width() / (values.len() - 1) as f32;
    let mut points = Vec::new();

    for (i, &v) in values.iter().enumerate() {
        let norm = (v - min_val) / (max_val - min_val);
        let x = rect.min.x + i as f32 * step_x;
        let y = rect.max.y - norm * (height - 4.0) - 2.0;
        points.push(Pos2::new(x, y));
    }

    for window in points.windows(2) {
        ui.painter().line_segment([window[0], window[1]], Stroke::new(1.5, color));
    }
}

/// Sharp card container for a tile with editor style.
pub fn tile_container<F>(ui: &mut Ui, title: &str, badge: Option<(&str, Color32)>, content: F)
where
    F: FnOnce(&mut Ui),
{
    Frame::none()
        .fill(Theme::BG_TILE)
        .stroke(Stroke::new(1.0, Theme::BORDER))
        .inner_margin(Margin::same(12.0))
        .rounding(Rounding::ZERO)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.monospace(
                    egui::RichText::new(title)
                        .size(12.0)
                        .strong()
                        .color(Theme::TEXT_BRIGHT),
                );
                if let Some((b_text, b_col)) = badge {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        neon_badge(ui, b_text, b_col);
                    });
                }
            });
            ui.add_space(8.0);
            content(ui);
        });
}