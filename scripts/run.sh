#!/bin/bash
export DISPLAY=:0
export XAUTHORITY=/home/tom/.Xauthority
export XDG_RUNTIME_DIR=/run/user/1000
exec /home/tom/Documents/codes/rust/notify_sleep/target/release/notify_sleep
