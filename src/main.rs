use clap::Parser;
use verba::{
    cli::{self, Cli, Command},
    daemon, ipc,
};

#[tokio::main]
async fn main() {
    verba::logging::init();

    let cli = Cli::parse();
    if let Err(err) = run(cli.command).await {
        tracing::error!("verba failed: {err}");
        std::process::exit(cli::exit_code_for_error(&err));
    }
}

async fn run(command: Command) -> verba::error::Result<()> {
    match command {
        Command::Daemon => {
            daemon::run().await?;
        }
        Command::Toggle => ipc::client::toggle().await?,
        Command::Show => ipc::client::show().await?,
        Command::Hide => ipc::client::hide().await?,
        Command::Settings => ipc::client::settings().await?,
        Command::Quit => ipc::client::quit().await?,
    };

    Ok(())
}
