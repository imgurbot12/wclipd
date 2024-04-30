use std::fs::read_to_string;
use std::io::{stdin, stdout, Read, Write};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use clap::{Args, Parser, Subcommand};
use thiserror::Error;

mod backend;
mod client;
mod clipboard;
mod config;
mod daemon;
mod message;
mod mime;
mod table;

use crate::client::{Client, ClientError};
use crate::clipboard::{ClipBody, Entry};
use crate::config::Config;
use crate::daemon::{Daemon, DaemonError};
use crate::message::Wipe;
use crate::mime::is_text;
use crate::table::*;

static XDG_PREFIX: &'static str = "wclipd";
static DEFAULT_SOCK: &'static str = "daemon.sock";
static DEFAULT_CONFIG: &'static str = "config.yaml";
static DEFAULT_DISK_STORE: &'static str = "db";

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
    #[error("Conflict Error")]
    ConflictError(String),
    #[error("Edit Error")]
    EditError(String),
}

/// Arguments for Copy Command
#[derive(Debug, Clone, Args)]
struct CopyArgs {
    /// Text to copy
    text: Vec<String>,
    /// FilePath to copy
    #[clap(short, long)]
    file: Option<PathBuf>,
    /// Specific Index to Copy Into
    #[clap(short, long)]
    index: Option<usize>,
    /// Specific Group To Copy Into
    #[clap(short, long)]
    group: Option<String>,
    /// Override the inferred MIME type
    #[arg(short, long)]
    mime: Option<String>,
    /// Copy to Primary Selection
    #[arg(short, long, default_value_t = false)]
    primary: bool,
    /// Clear Clipboard rather than copy anything
    #[arg(short, long, default_value_t = false)]
    clear: bool,
}

/// Arguments for Select Command
#[derive(Debug, Clone, Args)]
struct SelectArgs {
    /// Clipboard entry index within manager
    entry_num: usize,
    /// Copy to primary-selection
    #[arg(short, long, default_value_t = false)]
    primary: bool,
    /// Group to Select from
    #[clap(short, long)]
    group: Option<String>,
}

/// Arguments for Paste Command
#[derive(Debug, Clone, Args)]
struct PasteArgs {
    /// Clipboard entry index within manager
    entry_num: Option<usize>,
    /// Do not append a newline character
    #[arg(short, long, default_value_t = false)]
    no_newline: bool,
    /// Instead of pasting, list offered types
    #[arg(short, long, default_value_t = false)]
    list_types: bool,
    /// Group to Paste from
    #[clap(short, long)]
    group: Option<String>,
}

/// Arguments for Select Command
#[derive(Debug, Clone, Args)]
struct EditArgs {
    /// Clipboard entry index within manager
    entry_num: Option<usize>,
    /// Copy to primary-selection after edit
    #[arg(short, long, default_value_t = false)]
    primary: bool,
    /// Group to Edit from
    #[clap(short, long)]
    group: Option<String>,
}

/// Arguments for List Command
#[derive(Debug, Clone, Args)]
struct ListArgs {
    /// List of Groups to List
    groups: Vec<String>,
    /// Clipboard Preview Max-Length
    #[clap(short, long)]
    length: Option<usize>,
    /// List All Groups if Specified
    #[clap(short, long)]
    all: bool,
    /// Override Table Style
    #[clap(short = 's', long)]
    table_style: Option<Style>,
}

#[derive(Debug, Clone, Args)]
struct DeleteArgs {
    /// Clipboard entry index within manager
    entry_num: Option<usize>,
    /// Group to Delete From
    #[clap(short, long)]
    group: Option<String>,
    /// Delete All Records (if enabled)
    #[clap(short, long)]
    clear: bool,
}

/// Arguments for Daemon Command
#[derive(Debug, Clone, Args)]
struct DaemonArgs {
    /// Kill existing Daemon (if running)
    #[clap(short, long, default_value_t = false)]
    kill: bool,
    /// Toggle capturing of live clipboard events
    #[clap(short, long)]
    live: Option<bool>,
}

/// Valid CLI Command Actions
#[derive(Debug, Clone, Subcommand)]
enum Command {
    /// Copy input to clipboard and manager
    #[clap(visible_alias = "c")]
    Copy(CopyArgs),
    /// Recopy entry within manager
    #[clap(visible_alias = "r")]
    ReCopy(SelectArgs),
    /// Paste entries tracked within manager
    #[clap(visible_alias = "p")]
    Paste(PasteArgs),
    /// Edit an existing entry
    #[clap(visible_alias = "e")]
    Edit(EditArgs),
    /// Check current status of daemon
    Check,
    /// Show clipboard group entries within manager
    #[clap(visible_alias = "s")]
    Show(ListArgs),
    /// Delete entry within manager
    #[clap(visible_alias = "d")]
    Delete(DeleteArgs),
    /// Run clipboard manager daemon
    Daemon(DaemonArgs),
}

/// Cli Application Flags and Command Configuration
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
        if args.clear {
            if !args.text.is_empty() || args.file.is_some() {
                return Err(CliError::ConflictError(
                    "Cannot specify input when clearing clipboard".to_owned(),
                ));
            }
            return Ok(client.clear()?);
        }
        let entry = match args.text.is_empty() {
            false => Entry::text(args.text.join(" "), args.mime),
            true => match args.file {
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
        client.copy(entry, args.primary, args.group, args.index)?;
        Ok(())
    }

    /// Select Command Handler
    fn select(&self, args: SelectArgs) -> Result<(), CliError> {
        let path = self.get_socket();
        let mut client = Client::new(path)?;
        client.select(args.entry_num, args.primary, args.group)?;
        Ok(())
    }

    /// Paste Command Handler
    fn paste(&self, args: PasteArgs) -> Result<(), CliError> {
        let path = self.get_socket();
        let mut client = Client::new(path)?;
        let (entry, _) = client.find(args.entry_num, args.group)?;
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

    /// Edit an Existing Clipboard Entry
    fn edit(&self, args: EditArgs) -> Result<(), CliError> {
        let path = self.get_socket();
        let mut client = Client::new(path)?;
        // retrieve entry and confirm entry is text
        let (mut entry, index) = client.find(args.entry_num, args.group.clone())?;
        if !entry.mime.iter().any(|m| is_text(m)) {
            return Err(CliError::EditError("Can Only Edit Text".to_owned()));
        }
        // edit contents and move back to text
        let data = edit::edit_bytes(entry.as_bytes())?;
        let text = String::from_utf8(data)
            .map_err(|e| CliError::EditError(format!("failed to read clip: {e:?}")))?;
        entry.body = ClipBody::Text(text);
        // resubmit entry to clipboard
        client.copy(entry, args.primary, args.group, Some(index))?;
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
    fn list(&self, mut config: Config, mut args: ListArgs) -> Result<(), CliError> {
        // override daemon cli arguments
        config.list.preview_length = args.length.unwrap_or(config.list.preview_length);
        config.list.table.style = args.table_style.unwrap_or(config.list.table.style);
        // complete rendering of requested lists
        let path = self.get_socket();
        let mut client = Client::new(path)?;
        if args.groups.is_empty() {
            args.groups = args.all.then(|| client.groups()).unwrap_or_else(|| {
                Ok(vec![config
                    .list
                    .default_group
                    .unwrap_or_else(|| "default".to_owned())])
            })?;
        }
        let now = SystemTime::now();
        for (i, group) in args.groups.into_iter().enumerate() {
            // generate preview into table structure
            let mut previews = client.list(config.list.preview_length, Some(group.clone()))?;
            previews.sort_by_key(|p| p.last_used);
            let data: Table = previews
                .into_iter()
                .map(|p| {
                    let since = now.duration_since(p.last_used).unwrap_or_default();
                    let since = Duration::from_secs(since.as_secs());
                    let human = humantime::format_duration(since).to_string();
                    vec![format!("{}", p.index), p.preview, human]
                })
                .collect();
            // skip empty record-sets
            if data.is_empty() {
                continue;
            }
            // add extra space between tables
            if i > 0 {
                println!("");
            }
            // build ascii table
            let mut table = AsciiTable::new(group, config.list.table.style.clone());
            table.align_column(0, config.list.table.index_align.clone());
            table.align_column(1, config.list.table.preview_align.clone());
            table.align_column(2, config.list.table.time_align.clone());
            table.print(data);
        }
        Ok(())
    }

    /// Delete Command Handler
    fn delete(&self, args: DeleteArgs) -> Result<(), CliError> {
        let path = self.get_socket();
        let mut client = Client::new(path)?;
        if args.clear {
            let name = args.group.clone().unwrap_or_else(|| "default".to_owned());
            log::info!("clearing all records for group: {name:?}");
            client.wipe(Wipe::All, args.group)?;
            return Ok(());
        }
        let index = match args.entry_num {
            Some(index) => index,
            None => client
                .list(0, args.group.clone())?
                .into_iter()
                .map(|p| p.index)
                .max()
                .unwrap_or(0),
        };
        log::info!("deleting index {index} for group {:?}", args.group);
        client.wipe(Wipe::Single { index }, args.group)?;
        Ok(())
    }

    /// Daemon Service Command Handler
    fn daemon(&self, mut config: Config, args: DaemonArgs) -> Result<(), CliError> {
        // override daemon cli arguments
        config.daemon.kill = args.kill;
        config.daemon.capture_live = args.live.unwrap_or(config.daemon.capture_live);
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
        Command::ReCopy(args) => cli.select(args),
        Command::Paste(args) => cli.paste(args),
        Command::Edit(args) => cli.edit(args),
        Command::Check => cli.check(),
        Command::Show(args) => cli.list(config, args),
        Command::Delete(args) => cli.delete(args),
        Command::Daemon(args) => cli.daemon(config, args),
    }
}
