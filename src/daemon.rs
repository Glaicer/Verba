use crate::{app_runtime::AppRuntime, config::ConfigStore, error::Result, ipc};

pub async fn run() -> Result<()> {
    let store = ConfigStore::default_path()?;
    let config = store.load_or_create()?;
    let runtime = AppRuntime::new(config.ui.last_preset_id);
    ipc::server::serve(runtime).await
}
