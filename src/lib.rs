use std::{
    io::Cursor,
    time::{Duration, Instant},
};

pub mod session;
pub mod state_machine;

pub const APP_WINDOW_NAME: &str = "Sleep OSD";
pub const SLEEP_TIME: u64 = 120 * 60;

pub fn trigger_alert_sound() {
    std::thread::spawn(|| {
        let handle =
            rodio::DeviceSinkBuilder::open_default_sink().expect("open default audio stream");
        let player = rodio::Player::connect_new(handle.mixer());
        let sound_bytes = include_bytes!("../assets/memoria_fragmentada.mp3");
        if let Ok(source) = rodio::Decoder::new(Cursor::new(sound_bytes)) {
            let max_duration = Duration::from_secs(30);
            let fade_duration = Duration::from_secs(5);
            let base_volume = 0.75;

            player.set_volume(0.0);
            player.append(source);

            let start = Instant::now();
            while !player.empty() {
                let elasped = start.elapsed();
                if elasped >= max_duration {
                    break;
                }

                if elasped <= fade_duration {
                    let progress =
                        (elasped.as_secs_f32() / fade_duration.as_secs_f32()).clamp(0.0, 1.0);
                    player.set_volume(base_volume * progress);
                }
                if elasped >= max_duration - fade_duration {
                    let time_left = max_duration - elasped;
                    let progress =
                        (time_left.as_secs_f32() / fade_duration.as_secs_f32()).clamp(0.0, 1.0);
                    player.set_volume(base_volume * progress);
                }
                std::thread::sleep(Duration::from_millis(100));
            }

            player.stop();
        }
    });
}

pub fn trigger_system_suspend() {
    println!("Time out! Start to suspend system via DBus!");
    let _ = std::process::Command::new("wmctrl")
        .args(["-k", "on"])
        .status();
    let _ = std::process::Command::new("qdbus")
        .args([
            "org.kde.KWin",
            "/KWin",
            "org.kde.KWin.
  showDesktop",
            "true",
        ])
        .status();
    let _ = std::process::Command::new("dbus-send")
        .args([
            "--system",
            "--print-reply",
            "--dest=org.freedesktop.login1",
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager.Suspend",
            "boolean:true",
        ])
        .status();
}
