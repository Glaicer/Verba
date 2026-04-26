pub mod actions;
pub mod main_window;
pub mod preset_editor;
pub mod settings_dialog;

use gtk4::{gio, prelude::*, Application};

use crate::{
    app_runtime::AppRuntime,
    config::{AppConfig, ConfigStore},
    secrets::secret_service::SecretServiceStore,
};

pub const APPLICATION_ID: &str = "dev.aronov.Verba.Gui";

pub fn run(config: AppConfig, store: ConfigStore, runtime: AppRuntime) {
    let app = Application::builder()
        .application_id(APPLICATION_ID)
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();

    let _hold = app.hold();

    let config_for_activate = config.clone();
    let store_for_activate = store.clone();
    let runtime_for_activate = runtime.clone();
    app.connect_activate(move |app| {
        let controller = main_window::MainWindowController::build(
            app,
            config_for_activate.clone(),
            store_for_activate.clone(),
            SecretServiceStore::new(),
            runtime_for_activate.clone(),
        );
        controller.attach_runtime_sync(
            config_for_activate.clone(),
            store_for_activate.clone(),
            SecretServiceStore::new(),
            runtime_for_activate.clone(),
        );
        if present_window_on_startup() {
            controller.present();
        }
    });

    let runtime_for_poll = runtime.clone();
    app.connect_startup(move |app| {
        let app = app.clone();
        let runtime = runtime_for_poll.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            if runtime.is_exiting() {
                app.quit();
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    });

    app.run_with_args(&application_args());
}

pub fn application_args() -> [&'static str; 1] {
    ["verba"]
}

pub fn application_id() -> &'static str {
    APPLICATION_ID
}

pub fn present_window_on_startup() -> bool {
    false
}
