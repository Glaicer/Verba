use verba::{
    app_runtime::{AppRuntime, AppState},
    tray::indicator::{TrayIndicator, VerbaTray},
};

#[test]
fn tray_contract_should_match_plan() {
    assert_eq!(VerbaTray::icon_name_static(), "verba");
    assert_eq!(
        VerbaTray::new(AppRuntime::new("precise")).menu_labels(),
        ["Open", "Settings", "Exit"]
    );
}

#[test]
fn tray_should_provide_embedded_icon_pixmap() {
    let tray = VerbaTray::new(AppRuntime::new("precise"));
    let pixmaps = tray.icon_pixmaps_for_tests();

    assert_eq!(pixmaps.len(), 1);
    assert_eq!(pixmaps[0].width, 32);
    assert_eq!(pixmaps[0].height, 32);
    assert_eq!(pixmaps[0].data.len(), 32 * 32 * 4);
}

#[test]
fn tray_should_expose_repo_icon_theme_path_for_dev_runs() {
    let path = VerbaTray::icon_theme_path_static();

    assert!(path.ends_with("packaging/icons"));
    assert!(path.join("hicolor/scalable/apps/verba.svg").exists());
}

#[test]
fn tray_left_click_should_toggle_main_window() {
    let runtime = AppRuntime::new("precise");
    let mut tray = VerbaTray::new(runtime.clone());

    tray.left_click();
    assert_eq!(runtime.state(), AppState::VisibleIdle);

    tray.left_click();
    assert_eq!(runtime.state(), AppState::Hidden);
}

#[test]
fn tray_open_menu_should_show_main_window() {
    let runtime = AppRuntime::new("precise");
    let mut tray = VerbaTray::new(runtime.clone());

    tray.open_or_minimize();

    assert_eq!(runtime.state(), AppState::VisibleIdle);
}

#[test]
fn tray_open_menu_should_change_to_minimize_when_window_is_visible() {
    let runtime = AppRuntime::new("precise");
    let mut tray = VerbaTray::new(runtime.clone());

    tray.open_or_minimize();
    assert_eq!(tray.menu_labels()[0], "Minimize");

    tray.open_or_minimize();
    assert_eq!(runtime.state(), AppState::Hidden);
    assert_eq!(tray.menu_labels()[0], "Open");
}

#[test]
fn tray_settings_menu_should_open_settings_state() {
    let runtime = AppRuntime::new("precise");
    let mut tray = VerbaTray::new(runtime.clone());

    tray.open_settings();

    assert_eq!(runtime.state(), AppState::SettingsOpen);
}

#[test]
fn tray_exit_menu_should_request_runtime_exit() {
    let runtime = AppRuntime::new("precise");
    let mut tray = VerbaTray::new(runtime.clone());

    tray.exit();

    assert!(runtime.is_exiting());
}

#[test]
fn tray_indicator_should_be_optional_when_start_fails() {
    let indicator = TrayIndicator::unavailable();

    indicator.shutdown();
}

#[test]
fn tray_indicator_should_notice_external_runtime_state_changes() {
    let runtime = AppRuntime::new("precise");
    let mut sync = TrayIndicator::state_sync_for_tests(runtime.clone());

    assert!(!sync.poll_state_changed());

    runtime.show_main_window();
    assert!(sync.poll_state_changed());

    assert!(!sync.poll_state_changed());

    runtime.hide_main_window();
    assert!(sync.poll_state_changed());
}
