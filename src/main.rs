use std::time::{Duration, Instant};

use x11rb::{
    connection::Connection,
    protocol::xproto::{AtomEnum, ClientMessageEvent, ConnectionExt, EventMask, PropMode},
    wrapper::ConnectionExt as _,
};

const APP_WINDOW_NAME: &str = "Sleep OSD";

fn make_window_osd(window_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (conn, screen_num) = x11rb::connect(None)?;

    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    let wm_name = conn.intern_atom(false, b"_NET_WM_NAME")?.reply()?.atom;
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

    let tree = conn.query_tree(root)?.reply()?;
    for &win in &tree.children {
        if let Ok(prop) = conn
            .get_property(false, win, wm_name, AtomEnum::ANY, 0, 1024)?
            .reply()
        {
            if let Ok(title) = String::from_utf8(prop.value) {
                println!("title: {}", &title);
                if title == window_name {
                    println!("Window found, make it osd!");

                    conn.change_property32(
                        PropMode::REPLACE,
                        win,
                        net_wm_type,
                        AtomEnum::ATOM,
                        &[type_notification],
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

                    let event = ClientMessageEvent {
                        response_type: 33, // CLIENT_MESSAGE
                        format: 32,
                        sequence: 0,
                        window: win,
                        type_: net_wm_state,
                        data: [
                            1, // _NET_WM_STATE_ADD
                            state_skip_taskbar,
                            state_skip_pager,
                            state_above,
                            state_sticky,
                        ]
                        .into(),
                    };

                    conn.send_event(
                        false,
                        root,
                        EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                        event,
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
}

impl SleepOdsApp {
    fn new(duration: Duration) -> Self {
        Self {
            target_time: Instant::now() + duration,
        }
    }
}

impl eframe::App for SleepOdsApp {
    fn ui(&mut self, ctx: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let remaining = if self.target_time > now {
            self.target_time - now
        } else {
            Duration::ZERO
        };

        let mins = remaining.as_secs() / 60;
        let secs = remaining.as_secs() % 60;

        let frame = egui::Frame::NONE
            .fill(egui::Color32::from_black_alpha(160))
            .corner_radius(24.0)
            .inner_margin(27.0);

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.heading(
                    egui::RichText::new("Time to sleep!")
                        .color(egui::Color32::WHITE)
                        .size(24.0),
                );
                ui.add_space(9.0);
                ui.label(
                    egui::RichText::new(format!("{:02}:{:02}", mins, secs))
                        .color(egui::Color32::from_rgb(255, 100, 100))
                        .size(45.0)
                        .strong(),
                );
                ui.add_space(9.0);
                ui.label(
                    egui::RichText::new("This PC will sleep after time out!")
                        .color(egui::Color32::LIGHT_GRAY)
                        .size(15.0),
                );
            })
        });

        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::AlwaysOnTop,
        ));
        ctx.request_repaint_after(Duration::from_millis(500));
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}
fn main() -> eframe::Result<()> {
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(200));
        let _ = make_window_osd(APP_WINDOW_NAME);
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_mouse_passthrough(true)
            .with_inner_size([300.0, 150.0])
            .with_position([100.0, 100.0]),
        ..Default::default()
    };

    let app = SleepOdsApp::new(Duration::from_secs(120 * 60));
    eframe::run_native(APP_WINDOW_NAME, options, Box::new(|_| Ok(Box::new(app))))
}
