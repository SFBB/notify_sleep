use std::{
    process::Command,
    time::{Duration, Instant},
};

use eframe::egui_wgpu::{SurfaceConfig, WgpuConfiguration, wgpu::PresentMode};
use x11rb::{
    connection::Connection,
    protocol::xproto::{AtomEnum, ConnectionExt, PropMode},
    wrapper::ConnectionExt as _,
};

use crate::state_machine::OsdStateMachine;

mod state_machine;

const APP_WINDOW_NAME: &str = "Sleep OSD";
const SLEEP_TIME: u64 = 120 * 60;

fn trigger_system_suspend() {
    println!("Time out! Start to suspend system!");
    let _ = Command::new("systemctl").args(["suspend", "-i"]).status();
}

fn get_screen_resolution() -> (f32, f32) {
    if let Ok((conn, screen_num)) = x11rb::connect(None) {
        let screen = &conn.setup().roots[screen_num];
        return (
            screen.width_in_pixels as f32,
            screen.height_in_pixels as f32,
        );
    }
    println!("We use the fallback screen resolution!");
    (1920.0, 1080.0)
}

fn make_window_osd(window_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    let wm_name = conn.intern_atom(false, b"_NET_WM_NAME")?.reply()?.atom;
    let net_wm_type = conn
        .intern_atom(false, b"_NET_WM_WINDOW_TYPE")?
        .reply()?
        .atom;
    let type_utility = conn
        .intern_atom(false, b"_NET_WM_WINDOW_TYPE_UTILITY")?
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

    let tree = conn.query_tree(root)?.reply()?;
    for &win in &tree.children {
        if let Ok(prop) = conn
            .get_property(false, win, wm_name, AtomEnum::ANY, 0, 1024)?
            .reply()
        {
            if let Ok(title) = String::from_utf8(prop.value) {
                if title == window_name {
                    println!("Window found, setting OSD properties...");

                    conn.change_property32(
                        PropMode::REPLACE,
                        win,
                        net_wm_type,
                        AtomEnum::ATOM,
                        &[type_utility],
                    )?;

                    conn.change_property32(
                        PropMode::REPLACE,
                        win,
                        net_wm_state,
                        AtomEnum::ATOM,
                        &[
                            state_above,
                            state_skip_taskbar,
                            state_skip_pager,
                            state_sticky,
                        ],
                    )?;

                    conn.flush()?;
                    break;
                }
            }
        }
    }

    Ok(())
}

struct SleepOdsApp {
    target_time: Instant,
    state_machine: OsdStateMachine,
    is_window_shrunk: bool,
}

impl SleepOdsApp {
    fn new(duration: Duration) -> Self {
        Self {
            target_time: Instant::now() + duration,
            state_machine: OsdStateMachine::new(),
            is_window_shrunk: false,
        }
    }
}

impl eframe::App for SleepOdsApp {
    fn ui(&mut self, ctx: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let screen_rect = ctx.viewport_rect();

        if now >= self.target_time && !self.state_machine.is_finished() {
            self.state_machine
                .request(state_machine::Request::Finish, screen_rect);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        self.state_machine.tick(now, screen_rect);

        if !self.is_window_shrunk
            && self.state_machine.is_corner_badge()
            && let Some(corner) = self.state_machine.get_current_rect()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(corner.size()));
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(corner.min));
            self.is_window_shrunk = true;
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

    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(200));
        let _ = make_window_osd(APP_WINDOW_NAME);
    });

    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(SLEEP_TIME));
        trigger_system_suspend();
    });

    let options = eframe::NativeOptions {
        wgpu_options: WgpuConfiguration {
            surface: SurfaceConfig {
                present_mode: PresentMode::AutoNoVsync,
                desired_maximum_frame_latency: Some(1),
            },
            ..Default::default()
        },
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_mouse_passthrough(true)
            .with_position([0.0, 0.0])
            .with_inner_size([screen_w, screen_h]),
        ..Default::default()
    };

    let app = SleepOdsApp::new(Duration::from_secs(SLEEP_TIME));
    eframe::run_native(APP_WINDOW_NAME, options, Box::new(|_| Ok(Box::new(app))))
}
