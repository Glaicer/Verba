use verba::{
    app_runtime::{AppRuntime, AppState},
    cli::{exit_code_for_error, Command},
    error::VerbaError,
    ipc::{client::IpcCommand, constants},
};

#[test]
fn runtime_visibility_commands_should_be_idempotent() {
    let runtime = AppRuntime::new("precise");

    assert!(!runtime.main_window_visible());

    runtime.show_main_window();
    assert!(runtime.main_window_visible());

    runtime.show_main_window();
    assert!(runtime.main_window_visible());

    runtime.hide_main_window();
    assert!(!runtime.main_window_visible());

    runtime.hide_main_window();
    assert!(!runtime.main_window_visible());
}

#[test]
fn runtime_toggle_should_flip_main_window_visibility() {
    let runtime = AppRuntime::new("precise");

    runtime.toggle_main_window();
    assert_eq!(runtime.state(), AppState::VisibleIdle);

    runtime.toggle_main_window();
    assert_eq!(runtime.state(), AppState::Hidden);
}

#[test]
fn runtime_quit_should_mark_runtime_as_exiting() {
    let runtime = AppRuntime::new("precise");

    runtime.quit();

    assert_eq!(runtime.state(), AppState::Exiting);
}

#[test]
fn runtime_properties_should_expose_specified_ipc_values() {
    let runtime = AppRuntime::new("formal");

    assert!(!runtime.busy());
    assert_eq!(runtime.current_preset(), "formal");
}

#[test]
fn ipc_constants_should_match_specification() {
    assert_eq!(constants::SERVICE_NAME, "dev.aronov.Verba");
    assert_eq!(constants::OBJECT_PATH, "/io/github/example/Verba");
    assert_eq!(constants::INTERFACE_NAME, "dev.aronov.Verba");
}

#[test]
fn cli_control_commands_should_map_to_ipc_commands() {
    assert_eq!(
        IpcCommand::try_from(Command::Toggle),
        Ok(IpcCommand::Toggle)
    );
    assert_eq!(IpcCommand::try_from(Command::Show), Ok(IpcCommand::Show));
    assert_eq!(IpcCommand::try_from(Command::Hide), Ok(IpcCommand::Hide));
    assert_eq!(
        IpcCommand::try_from(Command::Settings),
        Ok(IpcCommand::Settings)
    );
    assert_eq!(IpcCommand::try_from(Command::Quit), Ok(IpcCommand::Quit));
    assert!(IpcCommand::try_from(Command::Daemon).is_err());
}

#[test]
fn cli_exit_codes_should_match_plan_contract() {
    assert_eq!(
        exit_code_for_error(&VerbaError::Dbus("service unavailable".to_string())),
        3
    );
    assert_eq!(
        exit_code_for_error(&VerbaError::SystemdStart("failed".to_string())),
        4
    );
    assert_eq!(
        exit_code_for_error(&VerbaError::Config("missing config".to_string())),
        1
    );
}
