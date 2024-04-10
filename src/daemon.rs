///! Clipboard Daemon Implementation
use std::fs::remove_file;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Barrier, RwLock};
use std::thread;

use thiserror::Error;
use wayland_clipboard_listener::WlClipboardPasteStream;
use wayland_clipboard_listener::WlListenType;

use crate::backend::{Backend, ClipboardRecord, MemoryStore};
use crate::client::Client;
use crate::message::*;

#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("Server Already Running Elsewhere")]
    AlreadyRunning,
    #[error("Socket Error")]
    SocketError(#[from] std::io::Error),
    #[error("Message Error")]
    MessageError(#[from] serde_json::Error),
}

/// Clipboard Daemon Implementation
pub struct Daemon {
    addr: PathBuf,
    backend: Arc<RwLock<dyn Backend>>,
    stopped: Arc<Barrier>,
}

impl Daemon {
    /// Spawn New Clipboard Daemon
    pub fn new(path: PathBuf) -> Result<Self, std::io::Error> {
        Ok(Self {
            addr: path,
            backend: Arc::new(RwLock::new(MemoryStore::new())),
            stopped: Arc::new(Barrier::new(2)),
        })
    }

    /// Process Incoming Request for Daemon
    pub fn process_request(&mut self, message: &Request) -> Response {
        log::debug!("incoming request: {message:?}");
        let mut backend = self.backend.write().expect("rwlock write failed");
        match message {
            Request::Ping => Response::Ok,
            Request::Stop => {
                self.stopped.wait();
                Response::Ok
            }
            Request::List => {
                let previews = backend.list();
                Response::Previews { previews }
            }
            Request::Find { index } => {
                let entry = backend.find(*index);
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
                    Wipe::Single { index } => backend.delete(*index),
                }
                Response::Ok
            }
        }
    }

    /// Process Socket Connection
    fn process_conn(
        &mut self,
        mut stream: UnixStream,
        buffer: &mut [u8],
    ) -> Result<(), DaemonError> {
        // read and parse request from client
        let n = stream.read(buffer)?;
        let request = serde_json::from_slice(&buffer[..n])?;
        // generate, pack, and send response to client
        let response = self.process_request(&request);
        let content = serde_json::to_vec(&response)?;
        stream.write(&content)?;
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
                    log::error!("daemon already running! exiting");
                    self.stopped.wait();
                    return Ok(());
                };
            };
        }
        let _ = remove_file(&self.addr);
        // spawn new socket server
        let listener = UnixListener::bind(&self.addr)?;
        let mut buffer = [0; 4196];
        for stream in listener.incoming() {
            let result = match stream {
                Ok(stream) => self.process_conn(stream, &mut buffer),
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
            let entry = ClipboardEntry::from(msg);
            if !entry.is_empty() {
                let record = ClipboardRecord::new(entry);
                let mut backend = self.backend.write().expect("rwlock write failed");
                let index = backend.add(record);
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
        log::debug!("threads initialized. waiting");
        self.stopped.wait();
        log::info!("daemon stopped");
        Ok(())
    }
}

impl Clone for Daemon {
    fn clone(&self) -> Self {
        Self {
            addr: self.addr.clone(),
            backend: Arc::clone(&self.backend),
            stopped: Arc::clone(&self.stopped),
        }
    }
}
