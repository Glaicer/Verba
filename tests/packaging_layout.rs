use std::fs;

#[test]
fn systemd_service_should_match_user_service_contract() {
    let service = fs::read_to_string("packaging/systemd/verba.service")
        .expect("systemd service should be readable");

    assert!(service.contains("Description=Verba tray daemon"));
    assert!(service.contains("PartOf=graphical-session.target"));
    assert!(service.contains("After=graphical-session.target"));
    assert!(service.contains("Type=exec"));
    assert!(service.contains("ExecStart=/usr/bin/verba daemon"));
    assert!(service.contains("Restart=on-failure"));
    assert!(service.contains("RestartSec=2"));
    assert!(service.contains("Environment=RUST_LOG=info"));
    assert!(service.contains("WantedBy=graphical-session.target"));
}

#[test]
fn install_script_should_follow_packaging_layout_and_enable_user_service() {
    let script =
        fs::read_to_string("packaging/scripts/install.sh").expect("install script should exist");

    assert!(script.contains(r#"PREFIX="${PREFIX:-/usr}""#));
    assert!(script.contains(r#"SUDO="${SUDO-sudo}""#));
    assert!(script.contains(r#"$SUDO install -Dm755 target/release/verba "$PREFIX/bin/verba""#));
    assert!(script.contains(
        r#"$SUDO install -Dm644 packaging/systemd/verba.service "$SYSTEMD_USER_DIR/verba.service""#
    ));
    assert!(script.contains(r#"systemctl --user daemon-reload"#));
    assert!(script.contains(r#"systemctl --user enable --now verba.service"#));
    assert!(
        !script.contains("sudo systemctl --user"),
        "user service commands must run as the graphical user, not root"
    );
    assert!(
        !script.contains("loginctl enable-linger"),
        "Verba must not enable lingering by default"
    );
}

#[test]
fn desktop_file_should_toggle_the_daemon_owned_window() {
    let desktop = fs::read_to_string("packaging/linux/verba.desktop")
        .expect("desktop file should be readable");

    assert!(desktop.contains("Type=Application"));
    assert!(desktop.contains("Name=Verba"));
    assert!(desktop.contains("Comment=Tray-based LLM translation utility"));
    assert!(desktop.contains("Exec=verba toggle"));
    assert!(desktop.contains("Icon=verba"));
    assert!(desktop.contains("Terminal=false"));
    assert!(desktop.contains("Categories=Utility;"));
}

#[test]
fn uninstall_script_should_remove_packaged_files_and_reload_user_systemd() {
    let script = fs::read_to_string("packaging/scripts/uninstall.sh")
        .expect("uninstall script should exist");

    assert!(script.contains(r#"PREFIX="${PREFIX:-/usr}""#));
    assert!(script.contains(r#"SUDO="${SUDO-sudo}""#));
    assert!(script.contains(r#"systemctl --user disable --now verba.service"#));
    assert!(script.contains(r#"$SUDO rm -f "$SYSTEMD_USER_DIR/verba.service""#));
    assert!(script.contains(r#"$SUDO rm -f "$APPLICATIONS_DIR/verba.desktop""#));
    assert!(script.contains(r#"$SUDO rm -f "$ICONS_DIR/verba.svg""#));
    assert!(script.contains(r#"$SUDO rm -f "$METAINFO_DIR/dev.aronov.Verba.metainfo.xml""#));
    assert!(script.contains(r#"$SUDO rm -f "$PREFIX/bin/verba""#));
    assert!(script.contains(r#"systemctl --user daemon-reload"#));
    assert!(
        !script.contains("sudo systemctl --user"),
        "user service commands must run as the graphical user, not root"
    );
    assert!(
        !script.contains("loginctl enable-linger"),
        "Verba must not manage lingering during uninstall"
    );
}
