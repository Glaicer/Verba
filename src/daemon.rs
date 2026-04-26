use crate::{app_runtime::AppRuntime, config::ConfigStore, error::Result, gui, ipc, tray};

pub async fn run() -> Result<()> {
    let store = ConfigStore::default_path()?;
    let config = store.load_or_create()?;
    let runtime = AppRuntime::new(config.ui.last_preset_id.clone());

    let server_runtime = runtime.clone();
    let server = tokio::spawn(async move { ipc::server::serve(server_runtime).await });
    let tray = tray::indicator::TrayIndicator::start(runtime.clone());
    gui::run(config, store, runtime);
    tray.shutdown();

    server
        .await
        .map_err(|err| crate::error::VerbaError::Dbus(err.to_string()))?
}
