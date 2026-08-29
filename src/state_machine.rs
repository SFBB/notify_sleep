use std::time::{Duration, Instant};

use egui::{Color32, CornerRadius, Pos2, Rect, RichText, Vec2};


pub enum State {
    FullScreenAlert {
        start_time: Instant,
        duration: Duration,
    },
    CenterCountdown {
        start_time: Instant,
        duration: Duration,
    },
    ShrinkingToCorner {
        from_rect: Rect,
        to_rect: Rect,
        start_time: Instant,
        duration: Duration,
    },
    CornerBadge {
        rect: Rect,
    },
    Finish,
}

#[allow(dead_code)]
pub enum Request {
    TriggerAlert { splash_duration: Duration },
    ShowCenterCard { duration: Duration },
    ShrinkToCorner { duration: Duration },
    Finish,
}

pub struct OsdStateMachine {
    state: State,
}

impl OsdStateMachine {
    pub fn new() -> Self {
        Self {
            state: State::FullScreenAlert {
                start_time: Instant::now(),
                duration: Duration::from_millis(2500),
            },
        }
    }

    fn ease_out_cubic(t: f32) -> f32 {
        1.0 - (1.0 - t).powi(3)
    }

    fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }

    pub fn request(&mut self, req: Request, screen_rect: Rect) {
        let now = Instant::now();

        let center_size = Vec2::new(360.0, 190.0);
        let center_pos = Pos2::new(
            (screen_rect.width() - center_size.x) / 2.0,
            (screen_rect.height() - center_size.y) / 2.0,
        );
        let center_rect = Rect::from_min_size(center_pos, center_size);

        let corner_size = Vec2::new(175.0, 42.0);
        let corner_pos = Pos2::new(screen_rect.width() - corner_size.x - 24.0, 24.0);
        let corner_rect = Rect::from_min_size(corner_pos, corner_size);

        self.state = match req {
            Request::TriggerAlert { splash_duration } => State::FullScreenAlert {
                start_time: now,
                duration: splash_duration,
            },
            Request::ShowCenterCard { duration } => State::CenterCountdown {
                start_time: now,
                duration,
            },
            Request::ShrinkToCorner { duration } => State::ShrinkingToCorner {
                from_rect: center_rect,
                to_rect: corner_rect,
                start_time: now,
                duration,
            },
            Request::Finish => State::Finish,
        };
    }

    pub fn tick(&mut self, now: Instant, screen_rect: Rect) {
        match &self.state {
            State::FullScreenAlert {
                start_time,
                duration,
            } => {
                if now.duration_since(*start_time) >= *duration {
                    self.request(
                        Request::ShowCenterCard {
                            duration: Duration::from_secs(3),
                        },
                        screen_rect,
                    );
                }
            }
            State::CenterCountdown {
                start_time,
                duration,
            } => {
                if now.duration_since(*start_time) >= *duration {
                    self.request(
                        Request::ShrinkToCorner {
                            duration: Duration::from_millis(1000),
                        },
                        screen_rect,
                    );
                }
            }
            State::ShrinkingToCorner {
                start_time,
                duration,
                to_rect,
                ..
            } => {
                if now.duration_since(*start_time) >= *duration {
                    let final_rect = *to_rect;
                    self.state = State::CornerBadge { rect: final_rect };
                }
            }
            State::CornerBadge { .. } | State::Finish => {}
        }
    }

    pub fn is_finished(&self) -> bool {
        matches!(self.state, State::Finish)
    }

    pub fn is_animating(&self) -> bool {
        matches!(
            self.state,
            State::FullScreenAlert { .. } | State::ShrinkingToCorner { .. }
        )
    }

    pub fn is_corner_badge(&self) -> bool {
        matches!(self.state, State::CornerBadge { .. })
    }

    pub fn get_current_rect(&self) -> Option<Rect> {
        match &self.state {
            State::CornerBadge { rect } => Some(*rect),
            _ => None,
        }
    }

    pub fn render(&self, ctx: &egui::Context, time_str: &str) {
        let now = Instant::now();
        let screen_rect = ctx.viewport_rect();

        match &self.state {
            State::Finish => {}

            State::FullScreenAlert {
                start_time,
                duration,
            } => {
                let elapsed = now.duration_since(*start_time).as_secs_f32();
                let dur = duration.as_secs_f32();
                let t = (elapsed / dur).clamp(0.0, 1.0);

                let alpha = if t < 0.2 {
                    (t / 0.2) * 210.0
                } else if t > 0.8 {
                    ((1.0 - t) / 0.2) * 210.0
                } else {
                    210.0
                } as u8;

                let painter = ctx.layer_painter(egui::LayerId::background());
                painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(alpha));

                egui::Area::new(egui::Id::new("fullscreen_layer"))
                    .fixed_pos(Pos2::ZERO)
                    .show(ctx, |ui| {
                        ui.set_min_size(screen_rect.size());
                        ui.vertical_centered(|ui| {
                            ui.add_space(screen_rect.height() * 0.35);
                            ui.label(RichText::new("🌙").size(64.0));
                            ui.add_space(10.0);
                            ui.heading(
                                RichText::new("TIME TO SLEEP")
                                    .color(Color32::from_rgb(255, 110, 110))
                                    .size(54.0)
                                    .strong(),
                            );
                            ui.add_space(12.0);
                            ui.label(
                                RichText::new("Time to sleep!")
                                    .color(Color32::from_white_alpha(200))
                                    .size(20.0),
                            );
                        });
                    });
            }

            State::CenterCountdown { .. } => {
                let center_size = Vec2::new(360.0, 190.0);
                let center_pos = Pos2::new(
                    (screen_rect.width() - center_size.x) / 2.0,
                    (screen_rect.height() - center_size.y) / 2.0,
                );
                let rect = Rect::from_min_size(center_pos, center_size);
                Self::draw_card(ctx, rect, 24.0, true, time_str);
            }

            State::ShrinkingToCorner {
                from_rect,
                to_rect,
                start_time,
                duration,
            } => {
                let elapsed = now.duration_since(*start_time).as_secs_f32();
                let dur = duration.as_secs_f32();
                let raw_t = (elapsed / dur).clamp(0.0, 1.0);
                let ease_t = Self::ease_out_cubic(raw_t);

                let cur_pos = Pos2::new(
                    Self::lerp(from_rect.min.x, to_rect.min.x, ease_t),
                    Self::lerp(from_rect.min.y, to_rect.min.y, ease_t),
                );
                let cur_size = Vec2::new(
                    Self::lerp(from_rect.width(), to_rect.width(), ease_t),
                    Self::lerp(from_rect.height(), to_rect.height(), ease_t),
                );
                let cur_radius = Self::lerp(24.0, 21.0, ease_t);

                let is_large_mode = raw_t < 0.15;
                Self::draw_card(
                    ctx,
                    Rect::from_min_size(cur_pos, cur_size),
                    cur_radius,
                    is_large_mode,
                    time_str,
                );
            }

            State::CornerBadge { rect } => {
                Self::draw_card(ctx, *rect, 21.0, false, time_str);
            }
        }
    }

    fn draw_card(ctx: &egui::Context, rect: Rect, radius: f32, is_large: bool, time_str: &str) {
        egui::Area::new(egui::Id::new("osd_card_layer"))
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                ui.set_min_size(rect.size());
                ui.set_max_size(rect.size());

                let bg_color = Color32::from_rgba_premultiplied(20, 20, 26, 210);
                let border_color = Color32::from_white_alpha(35);

                let frame = egui::Frame::NONE
                    .fill(bg_color)
                    .corner_radius(CornerRadius::same(radius as u8))
                    .stroke(egui::Stroke::new(1.0, border_color))
                    .inner_margin(egui::Margin::symmetric(14, 10));

                frame.show(ui, |ui| {
                    if is_large {
                        ui.vertical_centered(|ui| {
                            ui.add_space(4.0);
                            ui.heading(
                                RichText::new("Time to sleep!")
                                    .color(Color32::WHITE)
                                    .size(22.0)
                                    .strong(),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(time_str)
                                    .color(Color32::from_rgb(255, 100, 100))
                                    .size(42.0)
                                    .strong(),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new("This PC will sleep after time out!")
                                    .color(Color32::from_white_alpha(180))
                                    .size(13.0),
                            );
                        });
                    } else {
                        ui.horizontal_centered(|ui| {
                            ui.add_space(2.0);
                            ui.label(RichText::new("🌙").size(18.0));
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(time_str)
                                    .color(Color32::from_rgb(255, 110, 110))
                                    .size(17.0)
                                    .strong(),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new("Sleep")
                                    .color(Color32::from_white_alpha(150))
                                    .size(12.0),
                            );
                        });
                    }
                });
            });
    }
}
