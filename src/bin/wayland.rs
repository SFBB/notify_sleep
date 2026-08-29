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

    match state.phase {
        Phase::FullScreenAlert => {
            let content = iced::widget::column![
                text("🌙").size(64),
                text("TIME TO SLEEP")
                    .size(54)
                    .color(Color::from_rgb(1.0, 0.43, 0.43)),
                text("Time to sleep!")
                    .size(20)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.8)),
            ]
            .spacing(12)
            .align_x(alignment::Horizontal::Center);

            container(content)
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .style(|_theme| container::Style {
                    text_color: Some(Color::WHITE),
                    background: Some(Color::from_rgba(0.08, 0.08, 0.1, 0.85).into()),
                    ..Default::default()
                })
                .into()
        }
        Phase::CenterCountdown => {
            let content = iced::widget::column![
                text("Time to sleep!").size(22).color(Color::WHITE),
                text(time_str)
                    .size(42)
                    .color(Color::from_rgb(1.0, 0.43, 0.43)),
                text("This PC will sleep after time out!")
                    .size(13)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.7)),
            ]
            .spacing(8)
            .align_x(alignment::Horizontal::Center);

            let card = container(content)
                .padding([14, 10])
                .width(360)
                .height(190)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .style(|_theme| container::Style {
                    text_color: Some(Color::WHITE),
                    background: Some(Color::from_rgba(0.08, 0.08, 0.1, 0.85).into()),
                    border: Border {
                        color: Color::from_rgba(1.0, 1.0, 1.0, 0.15),
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
                text("🌙").size(18),
                text(time_str)
                    .size(17)
                    .color(Color::from_rgb(1.0, 0.43, 0.43)),
                text("Sleep")
                    .size(12)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.6)),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center);

            container(content)
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .padding([8, 14])
                .style(|_theme| container::Style {
                    text_color: Some(Color::WHITE),
                    background: Some(Color::from_rgba(0.08, 0.08, 0.1, 0.85).into()),
                    border: Border {
                        color: Color::from_rgba(1.0, 1.0, 1.0, 0.15),
                        width: 1.0,
                        radius: 21.0.into(),
                    },
                    ..Default::default()
                })
                .into()
        }
    }
}

fn subscription(_state: &State) -> Subscription<Message> {
    iced::time::every(Duration::from_millis(500)).map(|_| Message::Tick)
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
