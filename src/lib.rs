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

pub fn pause_video_players() {
    let script = r#"
        for inh in $(dbus-send --session --dest=org.gnome.SessionManager --type=method_call --print-reply /org/gnome/SessionManager org.gnome.SessionManager.GetInhibitors 2>/dev/null | grep -o "/org/gnome/SessionManager/Inhibitor[0-9]*"); do
            reason=$(dbus-send --session --dest=org.gnome.SessionManager --type=method_call --print-reply "$inh" org.gnome.SessionManager.Inhibitor.GetReason 2>/dev/null | grep -o "string \"[^\"]*\"" | cut -d "\"" -f2)
            app=$(dbus-send --session --dest=org.gnome.SessionManager --type=method_call --print-reply "$inh" org.gnome.SessionManager.Inhibitor.GetAppId 2>/dev/null | grep -o "string \"[^\"]*\"" | cut -d "\"" -f2)

            if echo "$reason" | grep -qi "video"; then
                app_lower=$(echo "$app" | tr "[:upper:]" "[:lower:]")
                keyword="org.mpris.MediaPlayer2"
                if echo "$app_lower" | grep -q "chrome\|chromium"; then
                    keyword="chromium"
                elif echo "$app_lower" | grep -q "firefox"; then
                    keyword="firefox"
                elif echo "$app_lower" | grep -q "vlc"; then
                    keyword="vlc"
                elif echo "$app_lower" | grep -q "mpv"; then
                    keyword="mpv"
                fi

                for player in $(dbus-send --session --dest=org.freedesktop.DBus --type=method_call --print-reply /org/freedesktop/DBus org.freedesktop.DBus.ListNames 2>/dev/null | grep -o "org\.mpris\.MediaPlayer2\.[^\"]*" | grep -i "$keyword"); do
                    dbus-send --session --type=method_call --dest="$player" /org/mpris/MediaPlayer2 org.mpris.MediaPlayer2.Player.Pause >/dev/null 2>&1
                done
            fi
        done
    "#;
    let _ = std::process::Command::new("bash")
        .arg("-c")
        .arg(script)
        .status();
}

pub fn trigger_system_suspend() {
    println!("Time out! Start to suspend system via DBus!");

    // 1. Pause any active video playback (sparing audio-only streams)
    // pause_video_players();

    // 2. Short wait to allow the browser/player to release its video wake lock
    std::thread::sleep(Duration::from_millis(200));

    // 3. Suspend system via logind DBus
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
