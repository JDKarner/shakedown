#!/usr/bin/env bash

SD_DIR="/home/$SUDO_USER/shakedown"

if [ $EUID -eq 0 ]; then
    cp -r $SD_DIR/shakedown /usr/bin/
    systemctl daemon-reload
    systemctl enable $SD_DIR/res/shakedown-launcher.service
else echo "Run as root"
    exit 1
fi

