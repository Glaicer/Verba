use std::time::Duration;

use tokio::{process::Command as TokioCommand, time::sleep};
use zbus::{proxy, Connection};

use crate::{
    cli::Command,
    error::{Result, VerbaError},
    ipc::constants::{INTERFACE_NAME, OBJECT_PATH, SERVICE_NAME},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IpcCommand {
    Toggle,
    Show,
    Hide,
    Settings,
    Quit,
    ReloadConfig,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("CLI command cannot be sent over IPC")]
pub struct NotIpcCommand;

impl TryFrom<Command> for IpcCommand {
    type Error = NotIpcCommand;

    fn try_from(value: Command) -> std::result::Result<Self, Self::Error> {
        match value {
            Command::Toggle => Ok(Self::Toggle),
            Command::Show => Ok(Self::Show),
            Command::Hide => Ok(Self::Hide),
            Command::Settings => Ok(Self::Settings),
            Command::Quit => Ok(Self::Quit),
            Command::Daemon => Err(NotIpcCommand),
        }
    }
}

pub async fn toggle() -> Result<()> {
    call(IpcCommand::Toggle).await
}

pub async fn show() -> Result<()> {
    call(IpcCommand::Show).await
}

pub async fn hide() -> Result<()> {
    call(IpcCommand::Hide).await
}

pub async fn settings() -> Result<()> {
    call(IpcCommand::Settings).await
}

pub async fn quit() -> Result<()> {
    call(IpcCommand::Quit).await
}

pub async fn reload_config() -> Result<()> {
    call(IpcCommand::ReloadConfig).await
}

pub async fn call(command: IpcCommand) -> Result<()> {
    match call_once(command).await {
        Ok(()) => Ok(()),
        Err(first_err) => {
            start_daemon().await?;
            sleep(Duration::from_millis(100)).await;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
            loop {
                match call_once(command).await {
                    Ok(()) => return Ok(()),
                    Err(err) if tokio::time::Instant::now() < deadline => {
                        let _ = err;
                        sleep(Duration::from_millis(100)).await;
                    }
                    Err(err) => {
                        return Err(VerbaError::Dbus(format!(
                            "D-Bus call failed after starting daemon: {err}; first error: {first_err}"
                        )));
                    }
                }
            }
        }
    }
}

async fn call_once(command: IpcCommand) -> std::result::Result<(), zbus::Error> {
    let connection = Connection::session().await?;
    let proxy = VerbaProxy::new(&connection).await?;
    match command {
        IpcCommand::Toggle => proxy.toggle_main_window().await,
        IpcCommand::Show => proxy.show_main_window().await,
        IpcCommand::Hide => proxy.hide_main_window().await,
        IpcCommand::Settings => proxy.open_settings().await,
        IpcCommand::Quit => proxy.quit().await,
        IpcCommand::ReloadConfig => proxy.reload_config().await,
    }
}

async fn start_daemon() -> Result<()> {
    let status = TokioCommand::new("systemctl")
        .args(["--user", "start", "verba.service"])
        .status()
        .await
        .map_err(|err| VerbaError::SystemdStart(err.to_string()))?;

    if status.success() {
        Ok(())
    } else {
        Err(VerbaError::SystemdStart(format!(
            "systemctl exited with {status}"
        )))
    }
}

#[proxy(
    interface = "dev.aronov.Verba",
    default_service = "dev.aronov.Verba",
    default_path = "/dev/aronov/Verba"
)]
trait Verba {
    async fn toggle_main_window(&self) -> zbus::Result<()>;
    async fn show_main_window(&self) -> zbus::Result<()>;
    async fn hide_main_window(&self) -> zbus::Result<()>;
    async fn open_settings(&self) -> zbus::Result<()>;
    async fn quit(&self) -> zbus::Result<()>;
    async fn reload_config(&self) -> zbus::Result<()>;

    #[zbus(property)]
    fn main_window_visible(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn busy(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn current_preset(&self) -> zbus::Result<String>;
}

#[allow(dead_code)]
fn _assert_proxy_constants_are_in_sync() {
    let _ = (SERVICE_NAME, OBJECT_PATH, INTERFACE_NAME);
}
