use zbus::{connection::Builder, interface, object_server::SignalContext};

use crate::{
    app_runtime::AppRuntime,
    error::{Result, VerbaError},
    ipc::constants::{INTERFACE_NAME, OBJECT_PATH, SERVICE_NAME},
};

#[derive(Clone, Debug)]
pub struct VerbaIpc {
    runtime: AppRuntime,
}

impl VerbaIpc {
    pub fn new(runtime: AppRuntime) -> Self {
        Self { runtime }
    }
}

#[interface(name = "dev.aronov.Verba")]
impl VerbaIpc {
    async fn toggle_main_window(&self) {
        self.runtime.toggle_main_window();
    }

    async fn show_main_window(&self) {
        self.runtime.show_main_window();
    }

    async fn hide_main_window(&self) {
        self.runtime.hide_main_window();
    }

    async fn open_settings(&self) {
        self.runtime.open_settings();
    }

    async fn quit(&self) {
        self.runtime.quit();
    }

    async fn reload_config(&self) {
        self.runtime.reload_config();
    }

    #[zbus(property)]
    fn main_window_visible(&self) -> bool {
        self.runtime.main_window_visible()
    }

    #[zbus(property)]
    fn busy(&self) -> bool {
        self.runtime.busy()
    }

    #[zbus(property)]
    fn current_preset(&self) -> String {
        self.runtime.current_preset()
    }

    #[zbus(signal)]
    async fn config_changed(ctxt: &SignalContext<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn translation_started(ctxt: &SignalContext<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn translation_finished(ctxt: &SignalContext<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn translation_failed(
        ctxt: &SignalContext<'_>,
        code: &str,
        message: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn main_window_visibility_changed(
        ctxt: &SignalContext<'_>,
        visible: bool,
    ) -> zbus::Result<()>;
}

pub async fn serve(runtime: AppRuntime) -> Result<()> {
    let _connection = Builder::session()
        .map_err(|err| VerbaError::Dbus(err.to_string()))?
        .name(SERVICE_NAME)
        .map_err(|err| VerbaError::Dbus(err.to_string()))?
        .serve_at(OBJECT_PATH, VerbaIpc::new(runtime.clone()))
        .map_err(|err| VerbaError::Dbus(err.to_string()))?
        .build()
        .await
        .map_err(|err| VerbaError::Dbus(err.to_string()))?;

    tracing::info!(
        service = SERVICE_NAME,
        path = OBJECT_PATH,
        interface = INTERFACE_NAME,
        "D-Bus service is running"
    );

    while !runtime.is_exiting() {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    Ok(())
}
