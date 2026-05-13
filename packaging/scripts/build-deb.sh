#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
VERSION="$(grep '^version' "$PROJECT_DIR/Cargo.toml" | head -1 | sed 's/.*"\([^"]*\)".*/\1/')"
ARCH="${1:-amd64}"
PKG_NAME="verba_${VERSION}-${ARCH}"

echo "Building verba ${VERSION} for ${ARCH}..."

cargo build --release --manifest-path "$PROJECT_DIR/Cargo.toml"

DEPS="$(ldd "$PROJECT_DIR/target/release/verba" \
    | grep -o '/[^ ]*\.so\.[0-9]*' \
    | sort -u \
    | xargs -I{} dpkg -S {} 2>/dev/null \
    | grep -v 'diversion' \
    | sed 's/: .*//' \
    | sort -u \
    | paste -sd, - \
    | sed 's/,/, /g')" || true

if [ -z "$DEPS" ]; then
    DEPS="libgtk-4-1, libglib2.0-0"
fi

echo "Detected dependencies: $DEPS"

STAGING="$PROJECT_DIR/target/debian/$PKG_NAME"
rm -rf "$STAGING"
mkdir -p "$STAGING/DEBIAN"

mkdir -p "$STAGING/usr/bin"
install -Dm755 "$PROJECT_DIR/target/release/verba" "$STAGING/usr/bin/verba"

mkdir -p "$STAGING/usr/lib/systemd/user"
install -Dm644 "$PROJECT_DIR/packaging/systemd/verba.service" "$STAGING/usr/lib/systemd/user/verba.service"

mkdir -p "$STAGING/usr/share/applications"
install -Dm644 "$PROJECT_DIR/packaging/linux/verba.desktop" "$STAGING/usr/share/applications/verba.desktop"

mkdir -p "$STAGING/usr/share/icons/hicolor/scalable/apps"
install -Dm644 "$PROJECT_DIR/packaging/icons/hicolor/scalable/apps/verba.svg" "$STAGING/usr/share/icons/hicolor/scalable/apps/verba.svg"

mkdir -p "$STAGING/usr/share/metainfo"
install -Dm644 "$PROJECT_DIR/packaging/metainfo/dev.aronov.Verba.metainfo.xml" "$STAGING/usr/share/metainfo/dev.aronov.Verba.metainfo.xml"

cat > "$STAGING/DEBIAN/control" <<EOF
Package: verba
Version: ${VERSION}-1
Section: utils
Priority: optional
Architecture: ${ARCH}
Depends: ${DEPS}
Maintainer: Glaicer <aronov.mml@gmail.com>
Description: Tray-based LLM translation utility
 Verba is a desktop translation utility that runs in the
 user graphical session and sends translation requests to
 an OpenAI-compatible chat completions API.
EOF

cat > "$STAGING/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
if [ "$1" = "configure" ]; then
    systemctl --user daemon-reload || true

    REAL_USER="${SUDO_USER:-}"
    if [ -z "$REAL_USER" ]; then
        REAL_USER=$(getent passwd | while IFS=: read -r u _ uid _ _ _ _; do
            if [ "$uid" -ge 1000 ] && [ "$uid" -lt 65534 ]; then echo "$u"; break; fi
        done)
    fi

    if [ -n "$REAL_USER" ] && [ "$(id -u)" = "0" ] && [ "$REAL_USER" != "root" ]; then
        if ! runuser -u "$REAL_USER" -- systemctl --user is-enabled verba.service >/dev/null 2>&1; then
            runuser -u "$REAL_USER" -- systemctl --user enable --now verba.service || true
        fi
    fi
fi
EOF
chmod 755 "$STAGING/DEBIAN/postinst"

cat > "$STAGING/DEBIAN/postrm" <<'EOF'
#!/bin/sh
set -e
if [ "$1" = "remove" ] || [ "$1" = "purge" ]; then
    REAL_USER="${SUDO_USER:-}"
    if [ -z "$REAL_USER" ]; then
        REAL_USER=$(getent passwd | while IFS=: read -r u _ uid _ _ _ _; do
            if [ "$uid" -ge 1000 ] && [ "$uid" -lt 65534 ]; then echo "$u"; break; fi
        done)
    fi

    if [ -n "$REAL_USER" ] && [ "$(id -u)" = "0" ] && [ "$REAL_USER" != "root" ]; then
        runuser -u "$REAL_USER" -- systemctl --user disable --now verba.service || true
    fi

    systemctl --user daemon-reload || true
fi
EOF
chmod 755 "$STAGING/DEBIAN/postrm"

dpkg-deb --build "$STAGING" "$PROJECT_DIR/target/debian/${PKG_NAME}.deb"

echo ""
echo "Built: target/debian/${PKG_NAME}.deb"
echo "Size:  $(du -h "$PROJECT_DIR/target/debian/${PKG_NAME}.deb" | cut -f1)"
