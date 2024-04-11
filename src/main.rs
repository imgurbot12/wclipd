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
mod mime;

use crate::backend::Storage;
use crate::client::{Client, ClientError};
use crate::daemon::{Daemon, DaemonError};

static DEFAULT_SOCK: &'static str = "~/.var/run/clapd.sock";

/// Possible CLI Errors
#[derive(Debug, Error)]
pub enum CliError {
    #[error("Read Error")]
    ReadError(#[from] std::io::Error),
    #[error("Client Error")]
    ClientError(#[from] ClientError),
    #[error("Daemon Error")]
    DaemonError(#[from] DaemonError),
}

/// Arguments for Copy Command
#[derive(Debug, Clone, Args)]
struct CopyArgs {
    /// Text to copy
    text: Vec<String>,
    /// FilePath to copy
    #[clap(short, long)]
    input: Option<PathBuf>,
    /// Communication socket
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
#[derive(Debug, Clone, Args)]
struct PasteArgs {
    /// Clipboard entry-number (from Daemon)
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

#[derive(Debug, Clone, Args)]
struct ClientArgs {
    #[clap(short, long, default_value_t = String::from(DEFAULT_SOCK))]
    socket: String,
}

/// Arguments for Daemon Command
#[derive(Debug, Clone, Args)]
struct DaemonArgs {
    /// Communication socket
    #[clap(short, long, default_value_t = String::from(DEFAULT_SOCK))]
    socket: String,
    /// Kill existing Daemon (if running)
    #[clap(short, long, default_value_t = false)]
    kill: bool,

    /// Backend storage implementation
    #[clap(short, long, default_value_t = Storage::Memory)]
    backend: Storage,
    /// Max number of clipboard entries stored
    #[clap(short, long)]
    max_entries: Option<usize>,
    /// Max lifetime of clipboard entry
    #[clap(short, long)]
    lifetime: Option<String>,
}

/*
pipe copying Cargo.toml fails, why?

1. Option to Kill Existing Daemon rather than Exit if exists
2. Choose Storage Option
3. Choose Max Clipboard Entries
4. Choose Clipboard Entry Lifetime
5. Output Preview to Rmenu Format?
6. Reimplement Backend Clipboard Libraries
*/

/// Valid CLI Command Actions
#[derive(Debug, Clone, Subcommand)]
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

#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

impl Cli {
    /// Expand Path and Convert to PathBuf
    #[inline]
    fn expand(&self, path: &str) -> PathBuf {
        PathBuf::from(shellexpand::tilde(path).to_string())
    }
    /// Copy Command Handler
    fn copy(&self, args: CopyArgs) -> Result<(), CliError> {
        let path = self.expand(&args.socket);
        let mut client = Client::new(path)?;
        let entry = match args.text.is_empty() {
            false => Entry::text(args.text.join(" "), args.mime),
            true => match args.input {
                Some(input) => {
                    let mime = args.mime.unwrap_or_else(|| mime::guess_mime_path(&input));
                    let content = std::fs::read(&input)?;
                    Entry::data(&content, Some(mime))
                }
                None => {
                    log::debug!("copying from stdin");
                    let mut buffer = Vec::new();
                    let n = stdin().read_to_end(&mut buffer)?;
                    Entry::data(&buffer[..n], args.mime)
                }
            },
        };
        log::debug!("sending entry {}", entry.preview(100));
        client.copy(entry)?;
        Ok(())
    }
    /// Paste Command Handler
    fn paste(&self, args: PasteArgs) -> Result<(), CliError> {
        let path = self.expand(&args.socket);
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
        Ok(())
    }
    /// Check-Daemon Command Handler
    fn check(&self, args: ClientArgs) -> Result<(), CliError> {
        let path = self.expand(&args.socket);
        if let Ok(mut client) = Client::new(path) {
            if let Ok(_) = client.ping() {
                return Ok(());
            }
        }
        std::process::exit(1)
    }
    /// List Clipboard Entry Previews Command Handler
    fn list(&self, args: ClientArgs) -> Result<(), CliError> {
        let path = self.expand(&args.socket);
        let mut client = Client::new(path)?;
        let list = client.list()?;
        for item in list {
            println!("{}.\t{}", item.index, item.preview);
        }
        Ok(())
    }
    /// Daemon Service Command Handler
    fn daemon(&self, args: DaemonArgs) -> Result<(), CliError> {
        let path = self.expand(&args.socket);
        let mut server = Daemon::new(path, args)?;
        server.run()?;
        Ok(())
    }
}

fn main() -> Result<(), CliError> {
    // enable log and set default level
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    // handle cli
    let cli = Cli::parse();
    match cli.command.clone() {
        Command::Copy(args) => cli.copy(args),
        Command::Paste(args) => cli.paste(args),
        Command::Check(args) => cli.check(args),
        Command::List(args) => cli.list(args),
        Command::Daemon(args) => cli.daemon(args),
    }
}
