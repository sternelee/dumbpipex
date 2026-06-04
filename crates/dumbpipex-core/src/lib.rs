use std::fmt::{Display, Formatter};
use std::str::FromStr;

use anyhow::{Context, Result};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use iroh::EndpointAddr;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const ALPN: &[u8] = b"dumbpipex-terminal-v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectTicket {
    pub version: u8,
    pub label: String,
    pub endpoint_addr: EndpointAddr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtySessionInfo {
    pub pty_id: String,
    pub shell: String,
    pub cols: u16,
    pub rows: u16,
    pub resume_token: String,
}

impl ConnectTicket {
    pub fn new(label: impl Into<String>, endpoint_addr: EndpointAddr) -> Self {
        Self {
            version: 1,
            label: label.into(),
            endpoint_addr,
        }
    }
}

impl Display for ConnectTicket {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_vec(self).map_err(|_| std::fmt::Error)?;
        f.write_str(&URL_SAFE_NO_PAD.encode(json))
    }
}

impl FromStr for ConnectTicket {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let bytes = URL_SAFE_NO_PAD
            .decode(value.trim())
            .context("failed to decode ticket")?;
        let ticket = serde_json::from_slice(&bytes).context("failed to parse ticket")?;
        Ok(ticket)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Hello { client_name: String },
    ListPtys,
    CreatePty {
        shell: Option<String>,
        cols: u16,
        rows: u16,
    },
    ResumePty {
        pty_id: String,
        resume_token: String,
        cols: u16,
        rows: u16,
    },
    PtyInput { pty_id: String, data: String },
    ResizePty { pty_id: String, cols: u16, rows: u16 },
    ClosePty { pty_id: String },
    Upload {
        name: String,
        mime: String,
        size: u64,
        data: String,
    },
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Hello { agent_name: String },
    PtyList { ptys: Vec<PtySessionInfo> },
    PtyCreated {
        pty_id: String,
        shell: String,
        cols: u16,
        rows: u16,
        resume_token: String,
        resumed: bool,
    },
    PtyOutput { pty_id: String, data: String },
    PtyExited { pty_id: String, exit_code: Option<i32> },
    /// Sent to a client that was attached to a PTY but lost the slot to
    /// another attaching client. The client should treat the PTY as detached
    /// and stop expecting live output for it.
    PtyDetached { pty_id: String, reason: String },
    Error { message: String },
    UploadAccepted {
        name: String,
        path: String,
    },
    UploadError {
        name: String,
        message: String,
    },
    Pong,
}

pub fn encode_bytes(data: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(data)
}

pub fn decode_bytes(data: &str) -> Result<Vec<u8>> {
    URL_SAFE_NO_PAD
        .decode(data)
        .context("failed to decode base64 payload")
}

pub async fn write_frame<W, T>(writer: &mut W, value: &T) -> Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let payload = serde_json::to_vec(value).context("failed to serialize frame")?;
    let size = u32::try_from(payload.len()).context("frame too large")?;
    writer.write_u32(size).await.context("failed to write frame size")?;
    writer
        .write_all(&payload)
        .await
        .context("failed to write frame payload")?;
    writer.flush().await.context("failed to flush frame")?;
    Ok(())
}

pub async fn read_frame<R, T>(reader: &mut R) -> Result<T>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned,
{
    let size = reader.read_u32().await.context("failed to read frame size")?;
    let mut payload = vec![0; size as usize];
    reader
        .read_exact(&mut payload)
        .await
        .context("failed to read frame payload")?;
    serde_json::from_slice(&payload).context("failed to decode frame")
}
