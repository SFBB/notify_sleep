use std::cell::Cell;
use std::time::{Duration, Instant};

use iced::widget::{container, row, text};
use iced::{Border, Color, Element, Subscription, Task, Theme, alignment};
use iced_layershell::daemon;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;

use notify_sleep::{SLEEP_TIME, trigger_system_suspend};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    FullScreenAlert,
    CenterCountdown,
    CornerBadge,
}

struct State {
    phase: Phase,
    start_time: Instant,
    target_time: Instant,
    window_id: Cell<Option<iced::window::Id>>,
    sleep_triggered: bool,
}

impl State {
    fn current_alpha(&self) -> f32 {
        let now = Instant::now();
        let elapsed = now.duration_since(self.start_time).as_millis() as f32;

        match self.phase {
            Phase::FullScreenAlert => {
                let total = 2500.0;
                let t = (elapsed / total).clamp(0.0, 1.0);
                if t < 0.2 {
                    t / 0.2
                } else if t > 0.8 {
                    (1.0 - t) / 0.2
                } else {
                    1.0
                }
            }
            Phase::CenterCountdown => {
                let total = 3000.0;
                let t = (elapsed / total).clamp(0.0, 1.0);
                if t < 0.167 {
                    t / 0.167
                } else if t > 0.833 {
                    (1.0 - t) / 0.167
                } else {
                    1.0
                }
            }
            Phase::CornerBadge => {
                let total = 500.0;
                let t = (elapsed / total).clamp(0.0, 1.0);
                t
            }
        }
    }
}

#[to_layer_message]
#[derive(Debug, Clone)]
enum Message {
    Tick,
}

fn boot() -> (State, Task<Message>) {
    (
        State {
            phase: Phase::FullScreenAlert,
            start_time: Instant::now(),
            target_time: Instant::now() + Duration::from_secs(SLEEP_TIME),
            window_id: Cell::new(None),
            sleep_triggered: false,
        },
        Task::none(),
    )
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    if let Message::Tick = message {
        let now = Instant::now();
        let elapsed = now.duration_since(state.start_time);

        match state.phase {
            Phase::FullScreenAlert => {
                if elapsed >= Duration::from_millis(2500) {
                    state.phase = Phase::CenterCountdown;
                    state.start_time = now;
                }
            }
            Phase::CenterCountdown => {
                if elapsed >= Duration::from_secs(3) {
                    state.phase = Phase::CornerBadge;
                    state.start_time = now;

                    // Change the physical layer shell window settings to corner badge size
                    let change_size = Task::done(Message::SizeChange((180, 44)));
                    let change_anchor =
                        Task::done(Message::AnchorChange(Anchor::Top | Anchor::Right));
                    let change_margin = Task::done(Message::MarginChange((24, 24, 0, 0)));

                    return Task::batch(vec![change_size, change_anchor, change_margin]);
                }
            }
            Phase::CornerBadge => {
                if !state.sleep_triggered && now >= state.target_time {
                    trigger_system_suspend();
                    state.sleep_triggered = true;
                    if let Some(id) = state.window_id.get() {
                        return iced::window::close(id);
                    }
                }
            }
        }
    }
    Task::none()
}

fn fade_color(color: Color, alpha: f32) -> Color {
    Color {
        a: color.a * alpha,
        ..color
    }
}

fn view<'a>(
    state: &'a State,
    window: iced::window::Id,
) -> Element<'a, Message, Theme, iced::Renderer> {
    state.window_id.set(Some(window));

    let now = Instant::now();
    let remaining = if state.target_time > now {
        state.target_time - now
    } else {
        Duration::ZERO
    };

    let hours = remaining.as_secs() / 3600;
    let mins = (remaining.as_secs() % 3600) / 60;
    let secs = remaining.as_secs() % 60;

    let time_str = if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    } else {
        format!("{:02}:{:02}", mins, secs)
    };

    let alpha = state.current_alpha();

    match state.phase {
        Phase::FullScreenAlert => {
            let content = iced::widget::column![
                text("🌙").size(64).color(fade_color(Color::WHITE, alpha)),
                text("TIME TO SLEEP")
                    .size(54)
                    .color(fade_color(Color::from_rgb(1.0, 0.43, 0.43), alpha)),
                text("Time to sleep!")
                    .size(20)
                    .color(fade_color(Color::from_rgba(1.0, 1.0, 1.0, 0.8), alpha)),
            ]
            .spacing(12)
            .align_x(alignment::Horizontal::Center);

            container(content)
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .style(move |_theme| container::Style {
                    text_color: Some(fade_color(Color::WHITE, alpha)),
                    background: Some(Color::from_rgba(0.08, 0.08, 0.1, 0.85 * alpha).into()),
                    ..Default::default()
                })
                .into()
        }
        Phase::CenterCountdown => {
            let content = iced::widget::column![
                text("Time to sleep!")
                    .size(22)
                    .color(fade_color(Color::WHITE, alpha)),
                text(time_str)
                    .size(42)
                    .color(fade_color(Color::from_rgb(1.0, 0.43, 0.43), alpha)),
                text("This PC will sleep after time out!")
                    .size(13)
                    .color(fade_color(Color::from_rgba(1.0, 1.0, 1.0, 0.7), alpha)),
            ]
            .spacing(8)
            .align_x(alignment::Horizontal::Center);

            let card = container(content)
                .padding([14, 10])
                .width(360)
                .height(190)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .style(move |_theme| container::Style {
                    text_color: Some(fade_color(Color::WHITE, alpha)),
                    background: Some(Color::from_rgba(0.08, 0.08, 0.1, 0.85 * alpha).into()),
                    border: Border {
                        color: Color::from_rgba(1.0, 1.0, 1.0, 0.15 * alpha),
                        width: 1.0,
                        radius: 24.0.into(),
                    },
                    ..Default::default()
                });

            container(card)
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .style(|_theme| container::Style {
                    background: Some(Color::TRANSPARENT.into()),
                    ..Default::default()
                })
                .into()
        }
        Phase::CornerBadge => {
            let content = row![
                text("🌙").size(18).color(fade_color(Color::WHITE, alpha)),
                text(time_str)
                    .size(17)
                    .color(fade_color(Color::from_rgb(1.0, 0.43, 0.43), alpha)),
                text("Sleep")
                    .size(12)
                    .color(fade_color(Color::from_rgba(1.0, 1.0, 1.0, 0.6), alpha)),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center);

            container(content)
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .padding([8, 14])
                .style(move |_theme| container::Style {
                    text_color: Some(fade_color(Color::WHITE, alpha)),
                    background: Some(Color::from_rgba(0.08, 0.08, 0.1, 0.85 * alpha).into()),
                    border: Border {
                        color: Color::from_rgba(1.0, 1.0, 1.0, 0.15 * alpha),
                        width: 1.0,
                        radius: 21.0.into(),
                    },
                    ..Default::default()
                })
                .into()
        }
    }
}

fn subscription(state: &State) -> Subscription<Message> {
    match state.phase {
        Phase::FullScreenAlert | Phase::CenterCountdown => {
            // Tick rapidly (every 16ms ~ 60fps) to make transitions look perfectly smooth
            iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick)
        }
        Phase::CornerBadge => {
            // Tick slowly (every 500ms) to save CPU/GPU resources when static
            let now = Instant::now();
            let elapsed = now.duration_since(state.start_time);
            if elapsed < Duration::from_millis(500) {
                // Keep ticking rapidly for the 500ms fade-in transition
                iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick)
            } else {
                iced::time::every(Duration::from_millis(500)).map(|_| Message::Tick)
            }
        }
    }
}

fn is_layer_shell_supported() -> bool {
    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        return false;
    }

    let conn = match Connection::connect_to_env() {
        Ok(c) => c,
        Err(_) => return false,
    };

    struct AppState;
    impl Dispatch<WlRegistry, GlobalListContents> for AppState {
        fn event(
            _state: &mut Self,
            _proxy: &WlRegistry,
            _event: <WlRegistry as wayland_client::Proxy>::Event,
            _data: &GlobalListContents,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
        ) {
        }
    }

    let (globals, _queue) = match registry_queue_init::<AppState>(&conn) {
        Ok(g) => g,
        Err(_) => return false,
    };

    globals
        .contents()
        .clone_list()
        .iter()
        .any(|g| g.interface == "zwlr_layer_shell_v1")
}

use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::{Connection, Dispatch, QueueHandle};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !is_layer_shell_supported() {
        eprintln!(
            "❌ Error: Your Wayland compositor does not support the 'wlr-layer-shell' protocol (zwlr_layer_shell_v1).\n\
             This protocol is required by the native Wayland mode (used by Sway, Hyprland, etc.).\n\
             If you are on GNOME or another compositor that doesn't support layer shell, please use the X11/XWayland version instead by running:\n\
             \n\
             ./notify_sleep_x11"
        );
        std::process::exit(1);
    }

    let settings = Settings {
        layer_settings: LayerShellSettings {
            layer: Layer::Overlay,
            anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
            margin: (0, 0, 0, 0),
            size: None,
            exclusive_zone: 0,
            keyboard_interactivity: KeyboardInteractivity::None,
            events_transparent: true,
            ..Default::default()
        },
        ..Default::default()
    };

    println!("🌟 [Wayland Mode] Starting native Wayland Layer Shell OSD...");
    daemon(boot, "sleep_osd", update, view)
        .settings(settings)
        .subscription(subscription)
        .run()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    Ok(())
}
