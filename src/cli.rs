use clap::{Parser, Subcommand};

use crate::error::VerbaError;

#[derive(Debug, Parser)]
#[command(name = "verba", about = "Tray-based LLM translation utility")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
pub enum Command {
    Daemon,
    Toggle,
    Show,
    Hide,
    Settings,
    Quit,
}

pub fn exit_code_for_error(error: &VerbaError) -> i32 {
    match error {
        VerbaError::Dbus(_) => 3,
        VerbaError::SystemdStart(_) => 4,
        _ => 1,
    }
}
