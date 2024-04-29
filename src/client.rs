//! Daemon Client Implementation

use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use thiserror::Error;

use crate::clipboard::{Entry, Preview};
use crate::message::*;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Socket Error")]
    SocketError(#[from] io::Error),
    #[error("Message Error")]
    MessageError(#[from] serde_json::Error),
    #[error("Unexpected Response")]
    Unexpected(Response),
}

/// Client to Clipboard Daemon
pub struct Client {
    socket: UnixStream,
}

impl Client {
    /// Spawn Daemon Client Instance
    pub fn new(path: PathBuf) -> Result<Self, ClientError> {
        Ok(Self {
            socket: UnixStream::connect(path)?,
        })
    }

    pub fn send(&mut self, request: Request) -> Result<Response, ClientError> {
        // write request to socket
        let mut message = serde_json::to_vec(&request)?;
        message.push('\n' as u8);
        self.socket.write(&message)?;
        // read response from socket
        let mut buffer = String::new();
        let mut reader = BufReader::new(&mut self.socket);
        let n = reader.read_line(&mut buffer)?;
        let response = serde_json::from_str(&buffer[..n])?;
        Ok(response)
    }

    /// Send Request and Expect `Ok` Response
    fn send_ok(&mut self, request: Request) -> Result<(), ClientError> {
        let response = self.send(request)?;
        if let Response::Ok = response {
            return Ok(());
        }
        Err(ClientError::Unexpected(response))
    }

    #[inline]
    pub fn ping(&mut self) -> Result<(), ClientError> {
        self.send_ok(Request::Ping)
    }

    #[inline]
    pub fn stop(&mut self) -> Result<(), ClientError> {
        self.send_ok(Request::Stop)
    }

    #[inline]
    pub fn clear(&mut self) -> Result<(), ClientError> {
        self.send_ok(Request::Clear)
    }

    #[inline]
    pub fn copy(
        &mut self,
        entry: Entry,
        primary: bool,
        group: Grp,
        index: Idx,
    ) -> Result<(), ClientError> {
        self.send_ok(Request::Copy {
            entry,
            primary,
            group,
            index,
        })
    }

    #[inline]
    pub fn select(&mut self, index: usize, primary: bool, group: Grp) -> Result<(), ClientError> {
        self.send_ok(Request::Select {
            index,
            primary,
            group,
        })
    }

    #[inline]
    pub fn delete(&mut self, index: usize, group: Grp) -> Result<(), ClientError> {
        self.send_ok(Request::Delete { index, group })
    }

    pub fn groups(&mut self) -> Result<Vec<String>, ClientError> {
        let response = self.send(Request::Groups)?;
        if let Response::Groups { groups } = response {
            return Ok(groups);
        }
        Err(ClientError::Unexpected(response))
    }

    pub fn find(&mut self, index: Option<usize>, group: Grp) -> Result<Entry, ClientError> {
        let response = self.send(Request::Find { index, group })?;
        if let Response::Entry { entry } = response {
            return Ok(entry);
        }
        Err(ClientError::Unexpected(response))
    }

    pub fn list(&mut self, length: usize, group: Grp) -> Result<Vec<Preview>, ClientError> {
        let response = self.send(Request::List { length, group })?;
        if let Response::Previews { previews } = response {
            return Ok(previews);
        }
        Err(ClientError::Unexpected(response))
    }
}
