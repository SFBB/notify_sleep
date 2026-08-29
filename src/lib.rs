pub mod session;
pub mod state_machine;

pub const APP_WINDOW_NAME: &str = "Sleep OSD";
pub const SLEEP_TIME: u64 = 120 * 60;

pub fn trigger_system_suspend() {
    println!("Time out! Start to suspend system via DBus!");
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
