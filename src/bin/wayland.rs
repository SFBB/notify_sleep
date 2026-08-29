use std::time::{Duration, Instant};

use iced::widget::{container, row, text};
use iced::{alignment, Border, Color, Element, Subscription, Task, Theme};
use iced_layershell::application;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;

use notify_sleep::{trigger_system_suspend, SLEEP_TIME};

struct State {
    target_time: Instant,
}

#[to_layer_message]
#[derive(Debug, Clone)]
enum Message {
    Tick,
}

fn boot() -> (State, Task<Message>) {
    (
        State {
            target_time: Instant::now() + Duration::from_secs(SLEEP_TIME),
        },
        Task::none(),
    )
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::Tick => {
            let now = Instant::now();
            if now >= state.target_time {
                trigger_system_suspend();
                std::process::exit(0);
            }
        }
        _ => {}
    }
    Task::none()
}

fn view<'a>(state: &'a State) -> Element<'a, Message, Theme, iced::Renderer> {
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
        .padding([8, 14])
        .style(|_theme: &Theme| container::Style {
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

use wayland_client::globals::{registry_queue_init, GlobalListContents};
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
            anchor: Anchor::Top | Anchor::Right,
            margin: (24, 24, 0, 0),
            size: Some((180, 44)),
            exclusive_zone: 0,
            keyboard_interactivity: KeyboardInteractivity::None,
            events_transparent: true,
            ..Default::default()
        },
        ..Default::default()
    };

    println!("🌟 [Wayland Mode] Starting native Wayland Layer Shell OSD...");
    application(boot, "sleep_osd", update, view)
        .settings(settings)
        .subscription(subscription)
        .run()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    Ok(())
}
