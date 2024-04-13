use std::fs::read_to_string;
use std::io::{stdin, stdout, Read, Write};
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use clipboard::Entry;
use thiserror::Error;

mod backend;
mod client;
mod clipboard;
mod config;
mod daemon;
mod message;
mod mime;

use crate::backend::{Lifetime, Storage};
use crate::client::{Client, ClientError};
use crate::config::Config;
use crate::daemon::{Daemon, DaemonError};

static XDG_PREFIX: &'static str = "wclipd";
static DEFAULT_SOCK: &'static str = "daemon.sock";
static DEFAULT_CONFIG: &'static str = "config.yaml";

/// Possible CLI Errors
#[derive(Debug, Error)]
pub enum CliError {
    #[error("Read Error")]
    ReadError(#[from] std::io::Error),
    #[error("Invalid Config")]
    ConfigError(#[from] serde_yaml::Error),
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
    /// Do not append a newline character
    #[arg(short, long, default_value_t = false)]
    no_newline: bool,
    /// Instead of pasting, list offered types
    #[arg(short, long, default_value_t = false)]
    list_types: bool,
}

/// Arguments for List Command
#[derive(Debug, Clone, Args)]
struct ListArgs {
    /// Clipboard Preview Max-Length
    #[clap(short, long, default_value_t = 100)]
    length: usize,
}

/// Arguments for Daemon Command
#[derive(Debug, Clone, Args)]
struct DaemonArgs {
    /// Kill existing Daemon (if running)
    #[clap(short, long, default_value_t = false)]
    kill: bool,
    /// Backend storage implementation
    #[clap(short, long)]
    backend: Option<Storage>,
    /// Max lifetime of clipboard entry
    #[clap(short, long)]
    lifetime: Option<Lifetime>,
    /// Max number of clipboard entries stored
    #[clap(short, long)]
    max_entries: Option<usize>,
}

/*
1. Option to Kill Existing Daemon rather than Exit if exists [DONE]
2. Choose Storage Option
3. Choose Max Clipboard Entries [DONE]
4. Choose Clipboard Entry Lifetime [DONE]
5. Output Preview to Rmenu Format?
6. Reimplement Backend Clipboard Libraries

[X] Implement Shared Configuration File for Client/Daemon
[X] Use XDG Standard for Setting Socket by Default
[X] More Robust Clipboard Entry Controls
    - [X] Delete OnLogin
    - [X] Delete OnReboot
    - [X] Delete After N-Seconds
*/

/// Valid CLI Command Actions
#[derive(Debug, Clone, Subcommand)]
enum Command {
    /// Copy input to Clipboard
    Copy(CopyArgs),
    /// Paste entries from Clipboard
    Paste(PasteArgs),
    /// Check Current Status of Daemon
    Check,
    /// List entries tracked within Daemon
    List(ListArgs),
    /// Clipboard Management Daemon
    Daemon(DaemonArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Communication socket
    #[clap(short, long)]
    socket: Option<String>,
    /// Configuration for WClipD
    #[clap(short, long)]
    config: Option<PathBuf>,
    /// WClipD Command
    #[clap(subcommand)]
    command: Command,
}

impl Cli {
    /// Load Configuration and Overload Empty Cli Settings
    fn load_config(&mut self) -> Result<Config, CliError> {
        let path = self.config.clone().or_else(|| {
            xdg::BaseDirectories::with_prefix(XDG_PREFIX)
                .expect("Failed to read xdg base dirs")
                .find_config_file(DEFAULT_CONFIG)
        });
        let config = match path {
            Some(path) => {
                let config = read_to_string(path)?;
                serde_yaml::from_str(&config)?
            }
            None => Config::default(),
        };
        self.socket = self.socket.clone().or(config.socket.clone());
        Ok(config)
    }
    /// Expand Path and Convert to PathBuf
    #[inline]
    fn get_socket(&self) -> PathBuf {
        let path = match self.socket.as_ref() {
            Some(sock) => sock.to_owned(),
            None => xdg::BaseDirectories::with_prefix(XDG_PREFIX)
                .expect("Failed to read xdg base dirs")
                .place_runtime_file(DEFAULT_SOCK)
                .expect("Failed to create daemon unix socket")
                .to_string_lossy()
                .to_string(),
        };
        PathBuf::from(shellexpand::tilde(&path).to_string())
    }
    /// Copy Command Handler
    fn copy(&self, args: CopyArgs) -> Result<(), CliError> {
        let path = self.get_socket();
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
        let path = self.get_socket();
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
    fn check(&self) -> Result<(), CliError> {
        let path = self.get_socket();
        if let Ok(mut client) = Client::new(path) {
            if let Ok(_) = client.ping() {
                return Ok(());
            }
        }
        std::process::exit(1)
    }
    /// List Clipboard Entry Previews Command Handler
    fn list(&self, args: ListArgs) -> Result<(), CliError> {
        let path = self.get_socket();
        let mut client = Client::new(path)?;
        let list = client.list(args.length)?;
        for item in list {
            println!("{}.\t{}", item.index, item.preview);
        }
        Ok(())
    }
    /// Daemon Service Command Handler
    fn daemon(&self, mut config: Config, args: DaemonArgs) -> Result<(), CliError> {
        // override daemon cli arguments
        config.daemon.kill = args.kill;
        config.daemon.backend = args.backend.unwrap_or(config.daemon.backend);
        config.daemon.lifetime = args.lifetime.unwrap_or(config.daemon.lifetime);
        config.daemon.max_entries = args.max_entries.or(config.daemon.max_entries);
        // run daemon
        let path = self.get_socket();
        let mut server = Daemon::new(path, config.daemon)?;
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
    let mut cli = Cli::parse();
    let config = cli.load_config()?;
    match cli.command.clone() {
        Command::Copy(args) => cli.copy(args),
        Command::Paste(args) => cli.paste(args),
        Command::Check => cli.check(),
        Command::List(args) => cli.list(args),
        Command::Daemon(args) => cli.daemon(config, args),
    }
}
