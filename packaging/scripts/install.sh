#!/usr/bin/env sh
set -eu

PREFIX="${PREFIX:-/usr}"
SUDO="${SUDO-sudo}"
SYSTEMD_USER_DIR="${SYSTEMD_USER_DIR:-$PREFIX/lib/systemd/user}"
APPLICATIONS_DIR="${APPLICATIONS_DIR:-$PREFIX/share/applications}"
ICONS_DIR="${ICONS_DIR:-$PREFIX/share/icons/hicolor/scalable/apps}"
METAINFO_DIR="${METAINFO_DIR:-$PREFIX/share/metainfo}"

cargo build --release

$SUDO install -Dm755 target/release/verba "$PREFIX/bin/verba"
$SUDO install -Dm644 packaging/systemd/verba.service "$SYSTEMD_USER_DIR/verba.service"
$SUDO install -Dm644 packaging/linux/verba.desktop "$APPLICATIONS_DIR/verba.desktop"
$SUDO install -Dm644 packaging/icons/hicolor/scalable/apps/verba.svg "$ICONS_DIR/verba.svg"
$SUDO install -Dm644 packaging/metainfo/dev.aronov.Verba.metainfo.xml "$METAINFO_DIR/dev.aronov.Verba.metainfo.xml"

systemctl --user daemon-reload
systemctl --user enable --now verba.service
