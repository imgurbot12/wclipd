use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use thiserror::Error;

mod backend;
mod client;
mod daemon;
mod message;

use crate::client::{Client, ClientError};
use crate::daemon::{Daemon, DaemonError};

static DEFAULT_SOCK: &'static str = "~/.var/run/clapd.sock";

/// Arguments for Copy Command
#[derive(Debug, Args)]
struct CopyArgs {
    /// Text to copy
    text: Vec<String>,
    /// Override the inferred MIME type
    #[arg(short, long)]
    mime: Option<String>,
    /// Clear Clipboard rather than copy anything
    #[arg(short, long, default_value_t = false)]
    clear: bool,
}

/// Arguments for Paste Command
#[derive(Debug, Args)]
struct PasteArgs {
    /// Clipboard Entry-Number (from Daemon)
    entry_num: Option<usize>,
    /// Do not append a newline character
    #[arg(short, long, default_value_t = false)]
    no_newline: bool,
    /// Instead of pasting, list offered types
    #[arg(short, long, default_value_t = false)]
    list_types: bool,
}

#[derive(Debug, Args)]
struct ClientArgs {
    #[clap(default_value_t = String::from(DEFAULT_SOCK))]
    socket: String,
}

/// Arguments for Daemon Command
#[derive(Debug, Args)]
struct DaemonArgs {
    /// Communication Socket
    #[clap(default_value_t = String::from(DEFAULT_SOCK))]
    socket: String,
}

/// Valid CLI Command Actions
#[derive(Debug, Subcommand)]
enum Command {
    /// Copy input to Clipboard
    Copy(CopyArgs),
    /// Paste entries from Clipboard
    Paste(PasteArgs),
    /// Check Current Status of Daemon
    Check(ClientArgs),
    /// List entries tracked within Daemon
    List(ClientArgs),
    /// Clipboard Management Daemon
    Daemon(DaemonArgs),
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Error)]
pub enum ClapdError {
    #[error("Client Error")]
    ClientError(#[from] ClientError),
    #[error("Daemon Error")]
    DaemonError(#[from] DaemonError),
}

#[inline]
fn expand(path: &str) -> PathBuf {
    PathBuf::from(shellexpand::tilde(path).to_string())
}

/*
examples:
  clipd show
    ...
    1. <preview>
    2. <preview>
    3. <preview>
    ...
  clipd copy <text...>
  clipd select [index] (index required)
  clipd paste <index> (index not required - defaults to last index)
*/

fn main() -> Result<(), ClapdError> {
    // enable log and set default level
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    // handle cli
    let cli = Cli::parse();
    match cli.command {
        Command::Copy(_) => todo!(),
        Command::Paste(_) => todo!(),
        Command::Check(args) => {
            let path = expand(&args.socket);
            if let Ok(mut client) = Client::new(path) {
                if let Ok(_) = client.ping() {
                    return Ok(());
                }
            }
            std::process::exit(1)
        }
        Command::List(args) => {
            let path = expand(&args.socket);
            let mut client = Client::new(path)?;
            let list = client.list()?;
            for item in list {
                println!("{item:?}");
            }
        }
        Command::Daemon(args) => {
            let path = expand(&args.socket);
            let mut server = Daemon::new(path).expect("daemon start failed");
            server.run()?;
        }
    };
    Ok(())
}
