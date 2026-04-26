#!/usr/bin/env sh
set -eu

PREFIX="${PREFIX:-/usr}"
SUDO="${SUDO-sudo}"
SYSTEMD_USER_DIR="${SYSTEMD_USER_DIR:-$PREFIX/lib/systemd/user}"
APPLICATIONS_DIR="${APPLICATIONS_DIR:-$PREFIX/share/applications}"
ICONS_DIR="${ICONS_DIR:-$PREFIX/share/icons/hicolor/scalable/apps}"
METAINFO_DIR="${METAINFO_DIR:-$PREFIX/share/metainfo}"

systemctl --user disable --now verba.service 2>/dev/null || true
$SUDO rm -f "$SYSTEMD_USER_DIR/verba.service"
$SUDO rm -f "$APPLICATIONS_DIR/verba.desktop"
$SUDO rm -f "$ICONS_DIR/verba.svg"
$SUDO rm -f "$METAINFO_DIR/dev.aronov.Verba.metainfo.xml"
$SUDO rm -f "$PREFIX/bin/verba"
systemctl --user daemon-reload
