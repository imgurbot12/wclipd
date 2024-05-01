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

use crate::backend::{Backend, BackendGroup, Manager, Record};
use crate::client::Client;
use crate::clipboard::Entry;
use crate::config::DaemonConfig;
use crate::message::*;

fn copy(entry: Entry, primary: bool) -> Result<(), DaemonError> {
    let mut stream = WlClipboardCopyStream::init()?;
    thread::spawn(move || {
        let context = entry.body.as_bytes().to_vec();
        let mimetypes = entry.mime.iter().map(|s| s.as_str()).collect();
        stream
            .copy_to_clipboard(context, mimetypes, primary)
            .expect("clipboard copy failed");
    });
    Ok(())
}

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

/// Shared Internal State between Threads
struct Shared {
    pub ignore: Option<Entry>,
    pub backend: Box<dyn Backend>,
    pub term_group: Grp,
    pub live_group: Grp,
}

impl Shared {
    pub fn new(cfg: DaemonConfig) -> Self {
        Self {
            ignore: None,
            backend: Box::new(Manager::new(cfg.backends)),
            term_group: cfg.term_backend,
            live_group: cfg.live_backend,
        }
    }
    #[inline]
    pub fn group(&mut self, group: Grp) -> Box<dyn BackendGroup> {
        self.backend.group(group.as_deref())
    }
}

/// Clipboard Daemon Implementation
pub struct Daemon {
    kill: bool,
    live: bool,
    recopy: bool,
    addr: PathBuf,
    shared: Arc<RwLock<Shared>>,
    start_wg: Arc<Barrier>,
    stop_wg: Arc<Barrier>,
}

impl Daemon {
    /// Spawn New Clipboard Daemon
    pub fn new(path: PathBuf, cfg: DaemonConfig) -> Result<Self, DaemonError> {
        let waiting = cfg.capture_live.then_some(3).unwrap_or(2);
        Ok(Self {
            kill: cfg.kill,
            live: cfg.capture_live,
            recopy: cfg.recopy_live,
            addr: path,
            shared: Arc::new(RwLock::new(Shared::new(cfg))),
            start_wg: Arc::new(Barrier::new(waiting)),
            stop_wg: Arc::new(Barrier::new(2)),
        })
    }

    /// Clear Active Clipboard
    pub fn clear(&self) -> Result<(), DaemonError> {
        let entry = Entry::text("".to_string(), None);
        copy(entry.clone(), true)?;
        copy(entry, false)
    }

    /// Add Entry To Clipboard with Following Settings
    pub fn copy(
        &mut self,
        entry: Entry,
        primary: bool,
        group: Grp,
        index: Idx,
    ) -> Result<(), DaemonError> {
        // update ignore tracking for live-updates to avoid double-copy
        let mut shared = self.shared.write().expect("rwlock write failed");
        shared.ignore = Some(entry.clone());
        // add entry to specified group
        let mime = entry.mime();
        let name = group.or(shared.term_group.clone());
        let mut group = shared.group(name.clone());
        let index = match index {
            Some(idx) => {
                group.insert(idx, Record::new(idx, entry.clone()));
                idx
            }
            None => group.push(entry.clone()),
        };
        // add to live clipboard
        copy(entry, primary)?;
        // log entry
        let name = name.unwrap_or_else(|| "default".to_owned());
        log::info!("copied term entry (group={name} index={index}) {mime:?}");
        Ok(())
    }

    /// Process Incoming Request for Daemon
    pub fn process_request(&mut self, message: Request) -> Result<Response, DaemonError> {
        Ok(match message {
            Request::Ping => Response::Ok,
            Request::Stop => {
                self.stop_wg.wait();
                Response::Ok
            }
            Request::Clear => {
                self.clear()?;
                Response::Ok
            }
            Request::Copy {
                entry,
                primary,
                group,
                index,
            } => {
                self.copy(entry, primary, group, index)?;
                Response::Ok
            }
            Request::Select {
                index,
                primary,
                group,
            } => {
                let record = {
                    let mut shared = self.shared.write().expect("rwlock write failed");
                    let group = group.clone().or(shared.term_group.clone());
                    shared.group(group).select(Some(index))
                };
                match record {
                    Some(record) => {
                        self.copy(record.entry, primary, group, None)?;
                        Response::Ok
                    }
                    None => Response::error(format!("No Such Index {index:?})")),
                }
            }
            Request::Groups => {
                let shared = self.shared.write().expect("rwlock read failed");
                let groups = shared.backend.groups();
                Response::Groups { groups }
            }
            Request::List { length, group } => {
                let mut shared = self.shared.write().expect("rwlock read failed");
                let group = group.or(shared.term_group.clone());
                let previews = shared.group(group.clone()).preview(length);
                Response::Previews { previews }
            }
            Request::Find { index, group } => {
                let mut shared = self.shared.write().expect("rwlock read failed");
                let group = group.or(shared.term_group.clone());
                match shared.group(group).find(index) {
                    Some(record) => Response::Entry {
                        entry: record.entry,
                        index: record.index,
                    },
                    None => Response::error(format!("No Such Index {index:?})")),
                }
            }
            Request::Wipe { wipe, group } => {
                let mut shared = self.shared.write().expect("rwlock write failed");
                let group = group.or(shared.term_group.clone());
                let mut group = shared.group(group);
                match wipe {
                    Wipe::All => {
                        group.clear();
                        Response::Ok
                    }
                    Wipe::Single { index } => match group.find(Some(index)) {
                        Some(_) => {
                            group.delete(&index);
                            Response::Ok
                        }
                        None => Response::error(format!("No Such Index {index:?})")),
                    },
                }
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
    fn server(&mut self) {
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
                            self.start_wg.wait();
                            log::error!("daemon already running! exiting");
                            self.stop_wg.wait();
                            return;
                        }
                    };
                };
            };
        }
        let _ = remove_file(&self.addr);
        // spawn new socket server
        self.start_wg.wait();
        let listener = UnixListener::bind(&self.addr).expect("failed to open socket listener");
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
    }

    /// Watch for Clipboard Updates and Save Non-Empty Copies
    fn watch_clipboard(&mut self) {
        log::debug!("watching clipboard for activity");
        let mut stream = WlClipboardPasteStream::init(WlListenType::ListenOnCopy)
            .expect("failed to open clipboard listener");
        self.start_wg.wait();
        for message in stream.paste_stream().flatten() {
            // collect clipboard entry object
            let Some(msg) = message else { continue };
            let entry = Entry::from(msg);
            // determine if entry should be ignored
            let mut shared = self.shared.write().expect("rwlock write failed");
            let group = shared.live_group.clone();
            if entry.is_empty() || shared.ignore.as_ref().map(|i| i == &entry).unwrap_or(false) {
                continue;
            }
            // copy into manager
            let mime = entry.mime();
            let name = group.clone().unwrap_or_else(|| "default".to_owned());
            let index = shared.group(group).push(entry.clone());
            log::info!("copied live entry (group={name} index={index}) {mime:?}");
            // recopy clipboard if enabled
            if self.recopy {
                shared.ignore = Some(entry.clone());
                if let Err(err) = copy(entry, false) {
                    log::error!("failed to re-copy clipboard: {err:?}");
                };
            }
        }
    }

    /// Listen for Incoming Events and Send Responses
    pub fn run(&mut self) -> Result<(), DaemonError> {
        // spawn threads
        if self.live {
            let mut wdaemon = self.clone();
            thread::spawn(move || wdaemon.watch_clipboard());
        }
        let mut sdaemon = self.clone();
        thread::spawn(move || sdaemon.server());
        // wait for services to start
        self.start_wg.wait();
        log::info!("daemon running");
        // wait for services to end
        self.stop_wg.wait();
        log::info!("daemon stopped");
        Ok(())
    }
}

impl Clone for Daemon {
    fn clone(&self) -> Self {
        Self {
            kill: self.kill,
            live: self.live,
            recopy: self.recopy,
            addr: self.addr.clone(),
            shared: Arc::clone(&self.shared),
            start_wg: Arc::clone(&self.start_wg),
            stop_wg: Arc::clone(&self.stop_wg),
        }
    }
}
