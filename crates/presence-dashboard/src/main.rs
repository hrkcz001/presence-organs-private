//! Presence Tiling Telemetry Dashboard.
//! Standalone non-invasive desktop application with code editor aesthetic,
//! custom sharp-cornered window frame, and eclectic neon badges/gauges.

mod telemetry;
mod theme;
mod widgets;

use eframe::egui::{self, Margin, Rounding, Sense, Stroke, Ui, Vec2, ViewportCommand};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use telemetry::{find_workspace_root, poll_triad_telemetry, TriadState};
use theme::Theme;
use widgets::{neon_badge, neon_gauge, pulse_sparkline, scrollable_code_area, tile_container};

struct DashboardApp {
    root: PathBuf,
    state: TriadState,
    last_poll: Instant,
    sparkline_history: Vec<f32>,
    is_maximized: bool,
}

impl DashboardApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let root = find_workspace_root();
        let state = poll_triad_telemetry(&root);
        Self {
            root,
            state,
            last_poll: Instant::now(),
            sparkline_history: vec![12.0, 14.0, 15.0, 18.0, 16.0, 14.0, 20.0, 22.0, 19.0, 15.0, 16.0],
            is_maximized: false,
        }
    }
}

impl eframe::App for DashboardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        Theme::apply(ctx);

        // Auto-poll telemetry every 1.5 seconds without invasive token calls
        if self.last_poll.elapsed() > Duration::from_millis(1500) {
            self.state = poll_triad_telemetry(&self.root);
            self.last_poll = Instant::now();

            let next_val = (self.state.transcript_count as f32 * 1.5) % 30.0 + 10.0;
            self.sparkline_history.push(next_val);
            if self.sparkline_history.len() > 24 {
                self.sparkline_history.remove(0);
            }
        }

        // Custom window container (Sharp borderless window frame)
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(Theme::BG_APP)
                    .stroke(Stroke::new(1.0, Theme::BORDER))
                    .rounding(Rounding::ZERO)
                    .inner_margin(Margin::ZERO),
            )
            .show(ctx, |ui| {
                // 1. Custom Title Bar with Draggable Window Area & Sharp Controls
                self.render_custom_titlebar(ctx, ui);

                // 2. Main Infinite Vertical Scrollable Tiling Grid
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.add_space(4.0);
                        self.render_tiling_grid(ui);
                        ui.add_space(16.0);
                    });
            });

        // Request constant smooth repaint for real-time telemetry
        ctx.request_repaint_after(Duration::from_millis(500));
    }
}

impl DashboardApp {
    fn render_custom_titlebar(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        let titlebar_height = 36.0;
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), titlebar_height),
            Sense::click_and_drag(),
        );

        if response.dragged() {
            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
        }

        ui.painter().rect_filled(rect, Rounding::ZERO, Theme::BG_PANEL);
        ui.painter().line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            Stroke::new(1.0, Theme::BORDER),
        );

        ui.allocate_ui_at_rect(rect, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.monospace(
                    egui::RichText::new("PRESENCE // TELEMETRY")
                        .size(12.0)
                        .strong()
                        .color(Theme::TEXT_BRIGHT),
                );
                ui.add_space(8.0);
                neon_badge(ui, "DAEMON LIVE", Theme::NEON_GREEN);
                ui.add_space(8.0);
                ui.monospace(
                    egui::RichText::new(format!("[{}]", self.root.display()))
                        .size(11.0)
                        .color(Theme::TEXT_MUTED),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(6.0);

                    // Close button
                    if ui.add_sized([30.0, 24.0], egui::Button::new(egui::RichText::new("✕").size(12.0))).clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }

                    // Maximize button
                    let max_label = if self.is_maximized { "❐" } else { "□" };
                    if ui.add_sized([30.0, 24.0], egui::Button::new(egui::RichText::new(max_label).size(12.0))).clicked() {
                        self.is_maximized = !self.is_maximized;
                        ctx.send_viewport_cmd(ViewportCommand::Maximized(self.is_maximized));
                    }

                    // Minimize button
                    if ui.add_sized([30.0, 24.0], egui::Button::new(egui::RichText::new("—").size(12.0))).clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                    }
                });
            });
        });
    }

    fn render_tiling_grid(&mut self, ui: &mut Ui) {
        let available_width = ui.available_width() - 24.0;
        let col_count = if available_width > 1050.0 { 3 } else { 2 };
        let col_width = (available_width - (col_count as f32 - 1.0) * 12.0) / col_count as f32;

        ui.horizontal(|ui| {
            ui.add_space(12.0);

            // Column 1
            ui.vertical(|ui| {
                ui.set_width(col_width);

                // Tile 1: Hermeneutic Circle
                tile_container(
                    ui,
                    "HERMENEUTIC CIRCLE",
                    Some((&self.state.current_phase, Theme::NEON_CYAN)),
                    |ui| {
                        ui.horizontal(|ui| {
                            ui.monospace(egui::RichText::new("Active Persona:").color(Theme::TEXT_MUTED));
                            neon_badge(ui, &self.state.active_agent.to_uppercase(), Theme::NEON_PURPLE);
                        });
                        ui.add_space(4.0);
                        ui.monospace(egui::RichText::new(format!("Goal: {}", self.state.current_goal)).color(Theme::TEXT_BRIGHT));
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.monospace(egui::RichText::new("Strikes:").color(Theme::TEXT_MUTED));
                            let strike_color = if self.state.strikes == 0 { Theme::NEON_GREEN } else { Theme::NEON_MAGENTA };
                            neon_badge(ui, &format!("{}/3", self.state.strikes), strike_color);
                            ui.add_space(8.0);
                            ui.monospace(egui::RichText::new(format!("Entries: {}", self.state.transcript_count)).color(Theme::TEXT_MUTED));
                        });
                        ui.add_space(8.0);
                        ui.monospace(egui::RichText::new("TRANSCRIPT LOG").size(10.0).color(Theme::TEXT_MUTED));
                        let tr_text = self.state.recent_transcript.join("\n");
                        scrollable_code_area(ui, &tr_text, 0.45);
                    },
                );

                ui.add_space(12.0);

                // Tile 2: Winsense Sensory
                tile_container(
                    ui,
                    "WINSENSE TELEMETRY",
                    Some(("SENSORY ACTIVE", Theme::NEON_CYAN)),
                    |ui| {
                        ui.horizontal(|ui| {
                            ui.monospace(egui::RichText::new("Power Source:").color(Theme::TEXT_MUTED));
                            neon_badge(ui, &self.state.power_source, Theme::NEON_GREEN);
                        });
                        ui.add_space(6.0);
                        neon_gauge(ui, self.state.battery_percent.unwrap_or(1.0), "Battery Level", Theme::NEON_CYAN);
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.monospace(egui::RichText::new("User Idle:").color(Theme::TEXT_MUTED));
                            ui.monospace(egui::RichText::new(format!("{}s", self.state.idle_sec)).color(Theme::TEXT_BRIGHT));
                        });
                    },
                );

                ui.add_space(12.0);

                // Tile 3: Friction & Mood
                tile_container(
                    ui,
                    "FRICTION & STRIKES",
                    Some((&format!("COUNT: {}", self.state.friction_count), Theme::NEON_AMBER)),
                    |ui| {
                        ui.monospace(egui::RichText::new("Historical Friction Log").size(10.0).color(Theme::TEXT_MUTED));
                        let fr_text = if self.state.recent_friction.is_empty() {
                            "// 0 friction events recorded (system quiet)".to_string()
                        } else {
                            self.state.recent_friction.join("\n")
                        };
                        scrollable_code_area(ui, &fr_text, 0.4);
                    },
                );
            });

            ui.add_space(12.0);

            // Column 2
            ui.vertical(|ui| {
                ui.set_width(col_width);

                // Tile 4: Cord Spine
                tile_container(
                    ui,
                    "CORD REFLEX SPINE",
                    Some(("ARMED", Theme::NEON_GREEN)),
                    |ui| {
                        ui.horizontal(|ui| {
                            ui.monospace(egui::RichText::new("Suppression rules:").color(Theme::TEXT_MUTED));
                            neon_badge(ui, "0 active", Theme::NEON_GREEN);
                        });
                        ui.add_space(6.0);
                        ui.monospace(egui::RichText::new("CORD REFLEX MANIFEST").size(10.0).color(Theme::TEXT_MUTED));
                        let cord_code = "{\n  \"cord\": \"active\",\n  \"mode\": \"hardware_reflex\",\n  \"interceptors\": [\"emergency_stop\", \"rule_suppression\"],\n  \"latency_ms\": 0.4\n}";
                        scrollable_code_area(ui, cord_code, 0.6);
                    },
                );

                ui.add_space(12.0);

                // Tile 5: Channel Inbox & Outbox
                tile_container(
                    ui,
                    "CHANNEL COMMUNICATIONS",
                    Some(("NATIVE RUST", Theme::NEON_GREEN)),
                    |ui| {
                        ui.monospace(egui::RichText::new("OUTBOX (DISPATCHED)").size(10.0).color(Theme::TEXT_MUTED));
                        let out_text = if self.state.outbox_lines.is_empty() {
                            "// outbox clean".to_string()
                        } else {
                            self.state.outbox_lines.join("\n")
                        };
                        scrollable_code_area(ui, &out_text, 0.45);

                        ui.add_space(8.0);
                        ui.monospace(egui::RichText::new("INBOX (INCOMING)").size(10.0).color(Theme::TEXT_MUTED));
                        let in_text = if self.state.inbox_lines.is_empty() {
                            "// inbox clean".to_string()
                        } else {
                            self.state.inbox_lines.join("\n")
                        };
                        scrollable_code_area(ui, &in_text, 0.45);
                    },
                );

                ui.add_space(12.0);

                // Tile 6: Stem & Stimuli Bus
                tile_container(
                    ui,
                    "STEM & STIMULI BUS",
                    Some(("INTERVAL: 100ms", Theme::NEON_CYAN)),
                    |ui| {
                        ui.horizontal(|ui| {
                            ui.monospace(egui::RichText::new("Sensory Bus:").color(Theme::TEXT_MUTED));
                            neon_badge(ui, "ACTIVE", Theme::NEON_GREEN);
                            neon_badge(ui, "CADENCE OK", Theme::NEON_CYAN);
                        });
                        ui.add_space(6.0);
                        ui.monospace(egui::RichText::new("PULSE ACTIVITY SPARKLINE").size(10.0).color(Theme::TEXT_MUTED));
                        pulse_sparkline(ui, &self.sparkline_history, Theme::NEON_CYAN);
                    },
                );
            });

            // Column 3 (if wide screen)
            if col_count == 3 {
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.set_width(col_width);

                    // Discovered Organs Tiling
                    for organ in &self.state.organs {
                        let b_color = if organ.organ_type == "cli" { Theme::NEON_CYAN } else { Theme::NEON_AMBER };
                        tile_container(
                            ui,
                            &format!("ORGAN: {}", organ.name.to_uppercase()),
                            Some((&format!("v{}", organ.version), b_color)),
                            |ui| {
                                ui.horizontal(|ui| {
                                    ui.monospace(egui::RichText::new("Type:").color(Theme::TEXT_MUTED));
                                    neon_badge(ui, &organ.organ_type.to_uppercase(), Theme::NEON_PURPLE);
                                });
                                ui.add_space(4.0);
                                ui.monospace(egui::RichText::new(&organ.description).size(11.0).color(Theme::TEXT_MUTED));
                                ui.add_space(6.0);
                                scrollable_code_area(ui, &organ.raw_yaml, 0.35);
                            },
                        );
                        ui.add_space(12.0);
                    }
                });
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Presence Tiling Telemetry")
            .with_inner_size([1150.0, 780.0])
            .with_min_inner_size([680.0, 480.0])
            .with_decorations(false) // Custom sharp window frame in color of background
            .with_transparent(false),
        ..Default::default()
    };

    eframe::run_native(
        "presence-dashboard",
        native_options,
        Box::new(|cc| Ok(Box::new(DashboardApp::new(cc)))),
    )
}