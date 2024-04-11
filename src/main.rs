use std::io::{stdin, stdout, Read, Write};
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use clipboard::Entry;
use thiserror::Error;

mod backend;
mod client;
mod clipboard;
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
    /// Communication Socket
    #[clap(short, long, default_value_t = String::from(DEFAULT_SOCK))]
    socket: String,
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
    /// Communication Socket
    #[clap(short, long, default_value_t = String::from(DEFAULT_SOCK))]
    socket: String,
    /// Do not append a newline character
    #[arg(short, long, default_value_t = false)]
    no_newline: bool,
    /// Instead of pasting, list offered types
    #[arg(short, long, default_value_t = false)]
    list_types: bool,
}

#[derive(Debug, Args)]
struct ClientArgs {
    #[clap(short, long, default_value_t = String::from(DEFAULT_SOCK))]
    socket: String,
}

/// Arguments for Daemon Command
#[derive(Debug, Args)]
struct DaemonArgs {
    /// Communication Socket
    #[clap(short, long, default_value_t = String::from(DEFAULT_SOCK))]
    socket: String,
}

/*
1. Option to Kill Existing Daemon rather than Exit if exists
2. Choose Storage Option
3. Choose Max Clipboard Entries
4. Choose Clipboard Entry Lifetime
5. Output Preview to Rmenu Format?
6. Reimplement Backend Clipboard Libraries
*/

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
    #[error("Read Error")]
    ReadError(#[from] std::io::Error),
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
        Command::Copy(args) => {
            let path = expand(&args.socket);
            let mut client = Client::new(path)?;
            let entry = match args.text.is_empty() {
                false => Entry::text(args.text.join(" "), args.mime),
                true => {
                    log::debug!("copying from stdin");
                    let mut buffer = Vec::new();
                    let n = stdin().read_to_end(&mut buffer)?;
                    Entry::data(&buffer[..n], args.mime)
                }
            };
            log::debug!("sending entry {}", entry.preview(100));
            client.copy(entry)?;
        }
        Command::Paste(args) => {
            let path = expand(&args.socket);
            let mut client = Client::new(path)?;
            let entry = client.find(args.entry_num)?;
            if args.list_types {
                for mime in entry.mime {
                    println!("{mime}");
                }
                return Ok(());
            }
            let mut out = stdout();
            out.write(entry.as_bytes())?;
            if !args.no_newline {
                out.write(&['\n' as u8])?;
            }
        }
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
                println!("{}.\t{}", item.index, item.preview);
            }
        }
        Command::Daemon(args) => {
            let path = expand(&args.socket);
            let mut server = Daemon::new(path)?;
            server.run()?;
        }
    };
    Ok(())
}
