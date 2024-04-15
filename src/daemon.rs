///! Clipboard Daemon Implementation
use std::fs::remove_file;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Barrier, RwLock};
use std::thread;

use thiserror::Error;
use wayland_clipboard_listener::WlClipboardPasteStream;
use wayland_clipboard_listener::WlListenType;
use wayland_clipboard_listener::{WlClipboardCopyStream, WlClipboardListenerError};

use crate::backend::{Backend, BackendOpts};
use crate::client::Client;
use crate::clipboard::Entry;
use crate::config::DaemonConfig;
use crate::message::*;

#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("Server Already Running Elsewhere")]
    AlreadyRunning,
    #[error("Socket Error")]
    SocketError(#[from] std::io::Error),
    #[error("Message Error")]
    MessageError(#[from] serde_json::Error),
    #[error("Clipboard Error")]
    ClipboardError(#[from] WlClipboardListenerError),
}

/// Clipboard Daemon Implementation
pub struct Daemon {
    kill: bool,
    addr: PathBuf,
    backend: Arc<RwLock<Box<dyn Backend>>>,
    stopped: Arc<Barrier>,
}

impl Daemon {
    /// Spawn New Clipboard Daemon
    pub fn new(path: PathBuf, cfg: DaemonConfig) -> Result<Self, DaemonError> {
        let options = BackendOpts {
            backend: cfg.backend,
            lifetime: cfg.lifetime,
            max_entries: cfg.max_entries,
        };
        let backend = options.build();
        Ok(Self {
            kill: cfg.kill,
            addr: path,
            backend: Arc::new(RwLock::new(backend)),
            stopped: Arc::new(Barrier::new(2)),
        })
    }

    /// Process Incoming Request for Daemon
    pub fn process_request(&mut self, message: Request) -> Result<Response, DaemonError> {
        let mut backend = self.backend.write().expect("rwlock write failed");
        Ok(match message {
            Request::Ping => Response::Ok,
            Request::Stop => {
                self.stopped.wait();
                Response::Ok
            }
            Request::Copy { entry, primary } => {
                let mut stream = WlClipboardCopyStream::init()?;
                thread::spawn(move || {
                    let context = entry.body.as_bytes().to_vec();
                    let mimetypes = entry.mime.iter().map(|s| s.as_str()).collect();
                    stream
                        .copy_to_clipboard(context, mimetypes, primary)
                        .expect("clipboard copy failed");
                });
                Response::Ok
            }
            Request::List { length } => {
                let previews = backend.preview(length);
                Response::Previews { previews }
            }
            Request::Find { index } => {
                let entry = backend.find(index);
                match entry {
                    Some(entry) => Response::Entry {
                        entry: entry.entry.clone(),
                    },
                    None => Response::Error {
                        error: format!("No Such Index {index:?})"),
                    },
                }
            }
            Request::Wipe(wipe) => {
                match wipe {
                    Wipe::All => backend.clear(),
                    Wipe::Single { index } => backend.delete(index),
                }
                Response::Ok
            }
        })
    }

    /// Process Socket Connection
    fn process_conn(&mut self, mut stream: UnixStream) -> Result<(), DaemonError> {
        loop {
            // read and parse request from client
            let mut buffer = String::new();
            let mut reader = BufReader::new(&mut stream);
            let n = reader.read_line(&mut buffer)?;
            if n == 0 {
                break;
            }
            let request = serde_json::from_str(&buffer[..n])?;
            // generate, pack, and send response to client
            let response = self.process_request(request)?;
            let mut content = serde_json::to_vec(&response)?;
            content.push('\n' as u8);
            stream.write(&content)?;
        }
        Ok(())
    }

    /// Listen for Incoming Server Requests Forever
    fn server(&mut self) -> Result<(), DaemonError> {
        log::debug!("listening for socket messages");
        // cleanup any remnants of dead daemon/socket
        if self.addr.exists() {
            // halt if existing daemon is already running
            if let Ok(mut client) = Client::new(self.addr.clone()) {
                if client.ping().is_ok() {
                    match self.kill {
                        true => {
                            log::warn!("daemon already running. killing it");
                            let _ = client.stop().expect("failed to kill daemon");
                        }
                        false => {
                            log::error!("daemon already running! exiting");
                            self.stopped.wait();
                            return Ok(());
                        }
                    };
                };
            };
        }
        let _ = remove_file(&self.addr);
        // spawn new socket server
        let listener = UnixListener::bind(&self.addr)?;
        for stream in listener.incoming() {
            let result = match stream {
                Ok(stream) => self.process_conn(stream),
                Err(err) => {
                    log::error!("connection error: {err:?}");
                    continue;
                }
            };
            if let Err(err) = result {
                log::error!("stream error: {err:?}");
            }
        }
        Ok(())
    }

    /// Watch for Clipboard Updates and Save Non-Empty Copies
    fn watch_clipboard(&mut self) {
        log::debug!("watching clipboard for activity");
        let mut stream = WlClipboardPasteStream::init(WlListenType::ListenOnCopy).unwrap();
        for message in stream.paste_stream().flatten() {
            let Some(msg) = message else { continue };
            let mime = msg
                .mime_types
                .get(0)
                .cloned()
                .unwrap_or_else(|| "N/A".to_owned());
            let entry = Entry::from(msg);
            if !entry.is_empty() {
                let mut backend = self.backend.write().expect("rwlock write failed");
                let index = backend.push(entry);
                log::info!("new clipboard entry (index={index:?}) {mime:?}");
            }
        }
    }

    /// Listen for Incoming Events and Send Responses
    pub fn run(&mut self) -> Result<(), DaemonError> {
        let mut wdaemon = self.clone();
        thread::spawn(move || wdaemon.watch_clipboard());
        let mut sdaemon = self.clone();
        thread::spawn(move || sdaemon.server());
        log::info!("daemon running");
        self.stopped.wait();
        log::info!("daemon stopped");
        Ok(())
    }
}

impl Clone for Daemon {
    fn clone(&self) -> Self {
        Self {
            kill: self.kill,
            addr: self.addr.clone(),
            backend: Arc::clone(&self.backend),
            stopped: Arc::clone(&self.stopped),
        }
    }
}
