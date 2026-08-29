#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionType {
    X11,
    Wayland,
}

pub fn detect_session_type() -> SessionType {
    if let Ok(backend) = std::env::var("WINIT_UNIX_BACKEND") {
        if backend.eq_ignore_ascii_case("x11") {
            return SessionType::X11;
        } else if backend.eq_ignore_ascii_case("wayland") {
            return SessionType::Wayland;
        }
    }

    if let Ok(session) = std::env::var("XDG_SESSION_TYPE") {
        if session.eq_ignore_ascii_case("wayland") {
            return SessionType::Wayland;
        } else if session.eq_ignore_ascii_case("x11") {
            return SessionType::X11;
        }
    }

    if std::env::var("WAYLAND_DISPLAY")
        .map(|s| !s.is_empty())
        .unwrap_or(false)
    {
        return SessionType::Wayland;
    }

    SessionType::X11
}
