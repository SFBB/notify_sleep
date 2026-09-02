use std::time::{Duration, Instant};

use eframe::egui_wgpu::wgpu::rwh::{HasWindowHandle, RawWindowHandle};
use eframe::egui_wgpu::{SurfaceConfig, WgpuConfiguration, wgpu::PresentMode};
use notify_sleep::{
    APP_WINDOW_NAME, SLEEP_TIME,
    session::{SessionType, detect_session_type},
    state_machine::{self, OsdStateMachine},
    trigger_alert_sound, trigger_system_suspend,
};
use x11rb::{
    connection::Connection,
    protocol::xproto::{AtomEnum, ClientMessageEvent, ConnectionExt, EventMask, PropMode},
    wrapper::ConnectionExt as _,
};

fn get_screen_resolution() -> (f32, f32) {
    if let Ok((conn, screen_num)) = x11rb::connect(None) {
        let screen = &conn.setup().roots[screen_num];
        return (
            screen.width_in_pixels as f32,
            screen.height_in_pixels as f32,
        );
    }
    (1920.0, 1080.0)
}

fn apply_osd_properties(
    conn: &impl Connection,
    root: u32,
    win_id: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let net_wm_type = conn
        .intern_atom(false, b"_NET_WM_WINDOW_TYPE")?
        .reply()?
        .atom;
    let type_notification = conn
        .intern_atom(false, b"_NET_WM_WINDOW_TYPE_NOTIFICATION")?
        .reply()?
        .atom;
    let net_wm_state = conn.intern_atom(false, b"_NET_WM_STATE")?.reply()?.atom;
    let state_above = conn
        .intern_atom(false, b"_NET_WM_STATE_ABOVE")?
        .reply()?
        .atom;
    let state_skip_taskbar = conn
        .intern_atom(false, b"_NET_WM_STATE_SKIP_TASKBAR")?
        .reply()?
        .atom;
    let state_skip_pager = conn
        .intern_atom(false, b"_NET_WM_STATE_SKIP_PAGER")?
        .reply()?
        .atom;
    let state_sticky = conn
        .intern_atom(false, b"_NET_WM_STATE_STICKY")?
        .reply()?
        .atom;

    // 1. Set window type to Notification
    conn.change_property32(
        PropMode::REPLACE,
        win_id,
        net_wm_type,
        AtomEnum::ATOM,
        &[type_notification],
    )?;

    // 2. Set WM_HINTS: explicitly declare that client rejects input focus
    let wm_hints = [
        1u32, // InputHint: input flag is valid
        0u32, // input: false (never takes keyboard focus)
        0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32,
    ];
    conn.change_property32(
        PropMode::REPLACE,
        win_id,
        AtomEnum::WM_HINTS,
        AtomEnum::WM_HINTS,
        &wm_hints,
    )?;

    // 3. Direct property write for initial state
    conn.change_property32(
        PropMode::REPLACE,
        win_id,
        net_wm_state,
        AtomEnum::ATOM,
        &[
            state_above,
            state_skip_taskbar,
            state_skip_pager,
            state_sticky,
        ],
    )?;

    // 4. Send ClientMessage to remove from taskbar and pager
    let event_skip = ClientMessageEvent {
        response_type: 33, // CLIENT_MESSAGE
        format: 32,
        sequence: 0,
        window: win_id,
        type_: net_wm_state,
        data: [1, state_skip_taskbar, state_skip_pager, 1, 0].into(),
    };
    conn.send_event(
        false,
        root,
        EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
        event_skip,
    )?;

    // 5. Send ClientMessage to ensure always on top and sticky
    let event_above = ClientMessageEvent {
        response_type: 33,
        format: 32,
        sequence: 0,
        window: win_id,
        type_: net_wm_state,
        data: [1, state_above, state_sticky, 1, 0].into(),
    };
    conn.send_event(
        false,
        root,
        EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
        event_above,
    )?;

    conn.flush()?;
    Ok(())
}

fn is_osd_confirmed(conn: &impl Connection, win_id: u32) -> bool {
    let Ok(net_wm_state) = conn.intern_atom(false, b"_NET_WM_STATE") else {
        return false;
    };
    let Ok(net_wm_state) = net_wm_state.reply() else {
        return false;
    };
    let Ok(skip_taskbar) = conn.intern_atom(false, b"_NET_WM_STATE_SKIP_TASKBAR") else {
        return false;
    };
    let Ok(skip_taskbar) = skip_taskbar.reply() else {
        return false;
    };

    let Ok(prop) = conn.get_property(
        false,
        win_id,
        net_wm_state.atom,
        AtomEnum::ATOM,
        0,
        32,
    ) else {
        return false;
    };
    let Ok(reply) = prop.reply() else {
        return false;
    };

    reply.value.chunks_exact(4).any(|chunk| {
        u32::from_ne_bytes(chunk.try_into().unwrap()) == skip_taskbar.atom
    })
}

fn ensure_osd_properties(win_id: u32) {
    std::thread::spawn(move || {
        let Ok((conn, screen_num)) = x11rb::connect(None) else {
            return;
        };
        let root = conn.setup().roots[screen_num].root;

        for _ in 0..50 {
            let _ = apply_osd_properties(&conn, root, win_id);

            if is_osd_confirmed(&conn, win_id) {
                break;
            }

            std::thread::sleep(Duration::from_millis(40));
        }
    });
}

struct SleepOdsApp {
    target_time: Instant,
    state_machine: OsdStateMachine,
    is_window_shrunk: bool,
    win_id: Option<u32>,
    first_frame: bool,
}

impl SleepOdsApp {
    fn new(duration: Duration, win_id: Option<u32>) -> Self {
        Self {
            target_time: Instant::now() + duration,
            state_machine: OsdStateMachine::new(),
            is_window_shrunk: false,
            win_id,
            first_frame: true,
        }
    }
}

impl eframe::App for SleepOdsApp {
    fn ui(&mut self, ctx: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.first_frame {
            self.first_frame = false;
            if let Some(id) = self.win_id {
                ensure_osd_properties(id);
            }
        }

        let now = Instant::now();
        let screen_rect = ctx.viewport_rect();

        if now >= self.target_time && !self.state_machine.is_finished() {
            self.state_machine
                .request(state_machine::Request::Finish, screen_rect);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        self.state_machine.tick(now, screen_rect);

        if detect_session_type() == SessionType::X11
            && !self.is_window_shrunk
            && self.state_machine.is_corner_badge()
            && let Some(corner) = self.state_machine.get_current_rect()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(corner.size()));
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(corner.min));
            self.is_window_shrunk = true;
            if let Some(id) = self.win_id {
                ensure_osd_properties(id);
            }
        }

        let remaining = if self.target_time > now {
            self.target_time - now
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

        self.state_machine.render(ctx, &time_str);

        if self.state_machine.is_animating() {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(Duration::from_millis(500));
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}

fn main() -> eframe::Result<()> {
    let (screen_w, screen_h) = get_screen_resolution();
    println!("Screen resolution: {}x{}", screen_w, screen_h);

    let suspend_timer_handle = std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(SLEEP_TIME));
        trigger_system_suspend();
    });

    trigger_alert_sound();

    let options = eframe::NativeOptions {
        wgpu_options: WgpuConfiguration {
            surface: SurfaceConfig {
                present_mode: PresentMode::AutoNoVsync,
                desired_maximum_frame_latency: Some(1),
            },
            ..Default::default()
        },
        viewport: egui::ViewportBuilder::default()
            .with_active(false)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_mouse_passthrough(true)
            .with_position([0.0, 0.0])
            .with_inner_size([screen_w, screen_h]),
        ..Default::default()
    };

    let app_result = eframe::run_native(
        APP_WINDOW_NAME,
        options,
        Box::new(|cc| {
            let mut win_id = None;
            if let Ok(handle) = cc.window_handle() {
                match handle.as_raw() {
                    RawWindowHandle::Xlib(h) => {
                        let id = h.window as u32;
                        ensure_osd_properties(id);
                        win_id = Some(id);
                    }
                    RawWindowHandle::Xcb(h) => {
                        let id = h.window.get();
                        ensure_osd_properties(id);
                        win_id = Some(id);
                    }
                    _ => {}
                }
            }

            Ok(Box::new(SleepOdsApp::new(
                Duration::from_secs(SLEEP_TIME),
                win_id,
            )))
        }),
    );

    if let Err(e) = suspend_timer_handle.join() {
        eprintln!("Failed to join suspend timer thread: {:?}", e);
    }

    app_result
}
