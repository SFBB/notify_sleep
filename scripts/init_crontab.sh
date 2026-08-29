#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
RUN_SCRIPT="$SCRIPT_DIR/run.sh"

echo "Start to setup contab task scheduling on 21:00 every day..."

if [ ! -f "$RUN_SCRIPT" ]; then
  echo "Creating $RUN_SCRIPT..."
  cat >"$RUN_SCRIPT" <<'EOF'
#!/bin/bash
export DISPLAY=:0
export XAUTHORITY=/home/tom/.Xauthority
export XDG_RUNTIME_DIR=/run/user/1000

exec /home/tom/Documents/codes/rust/notify_sleep/target/release/notify_sleep_x11
EOF
fi
chmod +x "$RUN_SCRIPT"

if [ ! -f "$PROJECT_DIR/target/release/notify_sleep_x11" ]; then
  echo "Building release program..."
  (cd "$PROJECT_DIR" && cargo build --release)
fi

CRON_SCHEDULE="0 21 * * *"
CRON_COMMAND="$RUN_SCRIPT >/dev/null 2>&1"
NEW_CRON_LINE="$CRON_SCHEDULE $CRON_COMMAND"

echo "Writing $(whoami)'s job..."
(
  crontab -l 2>/dev/null | grep -v "notify_sleep" || true
  echo "$NEW_CRON_LINE"
) | crontab -

echo ""
echo "Setup successfully, current crontab job list:"
echo "--------------------------------------------------"
crontab -l
echo "--------------------------------------------------"
echo "We will alert you to sleep at 21:00 every day."
