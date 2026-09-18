//! Gray code editor palette with sharp geometric edges and eclectic neon accents.

use egui::{Color32, Rounding, Stroke, Visuals};

pub struct Theme;

impl Theme {
    // Editor Neutral Grays
    pub const BG_APP: Color32 = Color32::from_rgb(0x16, 0x16, 0x18);
    pub const BG_PANEL: Color32 = Color32::from_rgb(0x1c, 0x1c, 0x1f);
    pub const BG_TILE: Color32 = Color32::from_rgb(0x22, 0x22, 0x26);
    pub const BG_CODE: Color32 = Color32::from_rgb(0x14, 0x14, 0x16);
    pub const BORDER: Color32 = Color32::from_rgb(0x32, 0x32, 0x38);
    pub const BORDER_BRIGHT: Color32 = Color32::from_rgb(0x45, 0x45, 0x50);

    // Text Grays
    pub const TEXT_BRIGHT: Color32 = Color32::from_rgb(0xee, 0xee, 0xf0);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x8a, 0x8a, 0x94);
    pub const TEXT_DIM: Color32 = Color32::from_rgb(0x5a, 0x5a, 0x64);

    // Eclectic Neon Accents (used specifically for badges, gauges, pulses)
    pub const NEON_CYAN: Color32 = Color32::from_rgb(0x00, 0xf0, 0xff);
    pub const NEON_GREEN: Color32 = Color32::from_rgb(0x00, 0xff, 0x88);
    pub const NEON_AMBER: Color32 = Color32::from_rgb(0xff, 0xaa, 0x00);
    pub const NEON_MAGENTA: Color32 = Color32::from_rgb(0xff, 0x00, 0x66);
    pub const NEON_PURPLE: Color32 = Color32::from_rgb(0xbf, 0x00, 0xff);

    pub fn apply(ctx: &egui::Context) {
        let mut visuals = Visuals::dark();

        // Strict sharp corners everywhere
        visuals.window_rounding = Rounding::ZERO;
        visuals.menu_rounding = Rounding::ZERO;
        visuals.widgets.noninteractive.rounding = Rounding::ZERO;
        visuals.widgets.inactive.rounding = Rounding::ZERO;
        visuals.widgets.hovered.rounding = Rounding::ZERO;
        visuals.widgets.active.rounding = Rounding::ZERO;
        visuals.widgets.open.rounding = Rounding::ZERO;

        // Colors
        visuals.panel_fill = Self::BG_APP;
        visuals.window_fill = Self::BG_PANEL;
        visuals.extreme_bg_color = Self::BG_CODE;

        visuals.widgets.noninteractive.bg_fill = Self::BG_TILE;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Self::BORDER);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Self::TEXT_BRIGHT);

        visuals.widgets.inactive.bg_fill = Self::BG_TILE;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Self::BORDER);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Self::TEXT_MUTED);

        visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x2a, 0x2a, 0x30);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Self::BORDER_BRIGHT);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Self::TEXT_BRIGHT);

        visuals.widgets.active.bg_fill = Color32::from_rgb(0x32, 0x32, 0x3a);
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, Self::NEON_CYAN);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, Self::TEXT_BRIGHT);

        ctx.set_visuals(visuals);
    }
}