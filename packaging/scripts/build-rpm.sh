#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
VERSION="$(grep '^version' "$PROJECT_DIR/Cargo.toml" | head -1 | sed 's/.*"\([^"]*\)".*/\1/')"
RELEASE="${RELEASE:-1}"
RPM_TOPDIR="$PROJECT_DIR/target/rpm"
SPEC_FILE="$RPM_TOPDIR/SPECS/verba.spec"

if ! command -v rpmbuild >/dev/null 2>&1; then
    echo "error: rpmbuild is required. Install it with: sudo dnf install rpm-build" >&2
    exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "error: cargo is required. Install Rust with rustup or Fedora's rust/cargo packages." >&2
    exit 1
fi

ARCH="${1:-$(rpm --eval '%{_target_cpu}')}"

echo "Building verba ${VERSION}-${RELEASE} for ${ARCH}..."

cargo build --release --manifest-path "$PROJECT_DIR/Cargo.toml"

rm -rf "$RPM_TOPDIR"
mkdir -p \
    "$RPM_TOPDIR/BUILD" \
    "$RPM_TOPDIR/BUILDROOT" \
    "$RPM_TOPDIR/RPMS" \
    "$RPM_TOPDIR/SOURCES" \
    "$RPM_TOPDIR/SPECS" \
    "$RPM_TOPDIR/SRPMS"

cat > "$SPEC_FILE" <<EOF
%global debug_package %{nil}

Name:           verba
Version:        ${VERSION}
Release:        ${RELEASE}%{?dist}
Summary:        Tray-based LLM translation utility
License:        MIT

Requires:       gtk4
Requires:       glib2
Requires:       libsecret
Requires:       systemd
Requires:       hicolor-icon-theme

%description
Verba is a desktop translation utility that runs in the user graphical session
and sends translation requests to an OpenAI-compatible chat completions API.

%prep

%build

%install
install -Dm755 "%{project_dir}/target/release/verba" "%{buildroot}%{_bindir}/verba"
install -Dm644 "%{project_dir}/packaging/systemd/verba.service" "%{buildroot}%{_prefix}/lib/systemd/user/verba.service"
install -Dm644 "%{project_dir}/packaging/linux/verba.desktop" "%{buildroot}%{_datadir}/applications/verba.desktop"
install -Dm644 "%{project_dir}/packaging/icons/hicolor/scalable/apps/verba.svg" "%{buildroot}%{_datadir}/icons/hicolor/scalable/apps/verba.svg"
install -Dm644 "%{project_dir}/packaging/metainfo/dev.aronov.Verba.metainfo.xml" "%{buildroot}%{_datadir}/metainfo/dev.aronov.Verba.metainfo.xml"

%post
systemctl --user daemon-reload || true

REAL_USER="\${SUDO_USER:-}"
if [ -z "\$REAL_USER" ]; then
    REAL_USER=\$(getent passwd | while IFS=: read -r u _ uid _ _ _ _; do
        if [ "\$uid" -ge 1000 ] && [ "\$uid" -lt 65534 ]; then echo "\$u"; break; fi
    done)
fi

if [ -n "\$REAL_USER" ] && [ "\$(id -u)" = "0" ] && [ "\$REAL_USER" != "root" ]; then
    if ! runuser -u "\$REAL_USER" -- systemctl --user is-enabled verba.service >/dev/null 2>&1; then
        runuser -u "\$REAL_USER" -- systemctl --user enable --now verba.service || true
    fi
fi

%postun
if [ "\$1" = "0" ]; then
    REAL_USER="\${SUDO_USER:-}"
    if [ -z "\$REAL_USER" ]; then
        REAL_USER=\$(getent passwd | while IFS=: read -r u _ uid _ _ _ _; do
            if [ "\$uid" -ge 1000 ] && [ "\$uid" -lt 65534 ]; then echo "\$u"; break; fi
        done)
    fi

    if [ -n "\$REAL_USER" ] && [ "\$(id -u)" = "0" ] && [ "\$REAL_USER" != "root" ]; then
        runuser -u "\$REAL_USER" -- systemctl --user disable --now verba.service || true
    fi

    systemctl --user daemon-reload || true
fi

%files
%{_bindir}/verba
%{_prefix}/lib/systemd/user/verba.service
%{_datadir}/applications/verba.desktop
%{_datadir}/icons/hicolor/scalable/apps/verba.svg
%{_datadir}/metainfo/dev.aronov.Verba.metainfo.xml

%changelog
* $(date '+%a %b %d %Y') Glaicer <aronov.mml@gmail.com> - ${VERSION}-${RELEASE}
- Build RPM package.
EOF

rpmbuild -bb "$SPEC_FILE" \
    --target "$ARCH" \
    --define "_topdir $RPM_TOPDIR" \
    --define "project_dir $PROJECT_DIR"

RPM_PATH="$(find "$RPM_TOPDIR/RPMS" -type f -name "verba-${VERSION}-${RELEASE}*.rpm" | head -1)"

echo ""
echo "Built: ${RPM_PATH#$PROJECT_DIR/}"
echo "Size:  $(du -h "$RPM_PATH" | cut -f1)"
