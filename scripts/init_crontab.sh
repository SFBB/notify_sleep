#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
RUN_SCRIPT="$SCRIPT_DIR/run.sh"

echo "Setting up notify_sleep scheduling for 21:00 every day via systemd user timer..."

# Ensure run.sh is executable
chmod +x "$RUN_SCRIPT"

# Build release binary if not present
if [ ! -f "$PROJECT_DIR/target/release/notify_sleep_x11" ]; then
  echo "Building release binary..."
  (cd "$PROJECT_DIR" && cargo build --release --bin notify_sleep_x11)
fi

# Clean up any legacy crontab entries to avoid double execution
if crontab -l 2>/dev/null | grep -q "notify_sleep"; then
  echo "Cleaning up legacy crontab entries..."
  (crontab -l 2>/dev/null | grep -v "notify_sleep" || true) | crontab -
fi

# Prepare user systemd directory
SYSTEMD_USER_DIR="$HOME/.config/systemd/user"
mkdir -p "$SYSTEMD_USER_DIR"

SERVICE_FILE="$SYSTEMD_USER_DIR/notify_sleep.service"
TIMER_FILE="$SYSTEMD_USER_DIR/notify_sleep.timer"

echo "Creating systemd user service: $SERVICE_FILE"
cat >"$SERVICE_FILE" <<EOF
[Unit]
Description=Notify Sleep OSD and Auto Suspend
After=graphical-session.target

[Service]
Type=simple
ExecStart=$RUN_SCRIPT
Restart=no

[Install]
WantedBy=default.target
EOF

echo "Creating systemd user timer: $TIMER_FILE"
cat >"$TIMER_FILE" <<'EOF'
[Unit]
Description=Daily 21:00 Timer for Notify Sleep OSD
PartOf=notify_sleep.service

[Timer]
OnCalendar=*-*-* 21:00:00
Persistent=true

[Install]
WantedBy=timers.target
EOF

# Reload and enable the timer
echo "Reloading systemd user daemon..."
systemctl --user daemon-reload

echo "Enabling and starting notify_sleep.timer..."
systemctl --user enable --now notify_sleep.timer

echo ""
echo "Setup successfully! Current timer status:"
echo "--------------------------------------------------"
systemctl --user list-timers notify_sleep.timer
echo "--------------------------------------------------"
echo "Helpful commands:"
echo "  - Test run immediately:  systemctl --user start notify_sleep.service"
echo "  - Check real-time logs:  journalctl --user -u notify_sleep.service -f"
echo "  - View timer schedule:   systemctl --user list-timers notify_sleep.timer"
echo "--------------------------------------------------"
