use std::fmt::{Display, Formatter};
use std::str::FromStr;

use anyhow::{Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use iroh::EndpointAddr;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const ALPN: &[u8] = b"dumbpipex-terminal-v1";
pub const MAX_INPUT_CHUNK_BYTES: usize = 16 * 1024;

/// Environment variable that overrides the iroh relay URL used by both the
/// CLI agent and the Tauri viewer.
///
/// When set to a non-empty value it must be a valid [`iroh::RelayUrl`] (e.g.
/// `https://relay.example.com`). When unset or empty, the official number0
/// (`n0`) iroh relay is used (the same set baked into iroh's `presets::N0`).
pub const RELAY_URL_ENV: &str = "DUMBPIPEX_RELAY_URL";

/// Hostname of the official number0 NA-East iroh relay. Mirrors
/// `iroh::defaults::NA_EAST_RELAY_HOSTNAME` from the iroh crate. iroh does
/// not currently re-export the full default URL as a single string, so we
/// document it here and use `iroh::RelayMode::Default` in callers to pick up
/// the whole four-region set.
pub const OFFICIAL_RELAY_URL: &str = "https://use1-1.relay.n0.iroh-canary.iroh.link";

/// Resolve the iroh relay URL to use, honoring [`RELAY_URL_ENV`] when set.
///
/// Returns the official iroh relay URL by default. Callers are expected to
/// pass the result into `iroh::RelayMap::from(...)` and wrap it in
/// `iroh::RelayMode::Custom(...)`; when the environment override is absent
/// callers may also choose `iroh::RelayMode::Default` to use iroh's full
/// four-region map.
pub fn resolve_relay_url() -> Result<iroh::RelayUrl> {
    let raw = std::env::var(RELAY_URL_ENV)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    match raw {
        Some(value) => value
            .parse::<iroh::RelayUrl>()
            .with_context(|| format!("invalid {RELAY_URL_ENV} value: {value:?}")),
        None => OFFICIAL_RELAY_URL
            .parse::<iroh::RelayUrl>()
            .context("invalid official iroh relay URL constant"),
    }
}

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
    #[serde(default)]
    pub bytes_dropped: u64,
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
    Hello {
        client_name: String,
    },
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
    PtyInput {
        pty_id: String,
        data: String,
    },
    ResizePty {
        pty_id: String,
        cols: u16,
        rows: u16,
    },
    ClosePty {
        pty_id: String,
    },
    Upload {
        name: String,
        mime: String,
        size: u64,
        data: String,
    },
    Ping {
        #[serde(default)]
        nonce: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Hello {
        agent_name: String,
    },
    PtyList {
        ptys: Vec<PtySessionInfo>,
    },
    PtyCreated {
        pty_id: String,
        shell: String,
        cols: u16,
        rows: u16,
        resume_token: String,
        resumed: bool,
        #[serde(default)]
        bytes_dropped: u64,
    },
    PtyOutput {
        pty_id: String,
        data: String,
    },
    PtyExited {
        pty_id: String,
        exit_code: Option<i32>,
    },
    /// Sent to a client that was attached to a PTY but lost the slot to
    /// another attaching client. The client should treat the PTY as detached
    /// and stop expecting live output for it.
    PtyDetached {
        pty_id: String,
        reason: String,
    },
    Error {
        message: String,
    },
    UploadAccepted {
        name: String,
        path: String,
    },
    UploadError {
        name: String,
        message: String,
    },
    Pong {
        #[serde(default)]
        nonce: u64,
    },
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
    writer
        .write_u32(size)
        .await
        .context("failed to write frame size")?;
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
    let size = reader
        .read_u32()
        .await
        .context("failed to read frame size")?;
    let mut payload = vec![0; size as usize];
    reader
        .read_exact(&mut payload)
        .await
        .context("failed to read frame payload")?;
    serde_json::from_slice(&payload).context("failed to decode frame")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_and_pong_round_trip_with_nonce() {
        let ping = ClientMessage::Ping { nonce: 42 };
        let encoded = serde_json::to_string(&ping).expect("serialize ping");
        assert_eq!(encoded, r#"{"type":"ping","nonce":42}"#);

        let decoded: ClientMessage = serde_json::from_str(&encoded).expect("decode ping");
        match decoded {
            ClientMessage::Ping { nonce } => assert_eq!(nonce, 42),
            other => panic!("unexpected message: {other:?}"),
        }

        let pong = ServerMessage::Pong { nonce: 42 };
        let encoded = serde_json::to_string(&pong).expect("serialize pong");
        assert_eq!(encoded, r#"{"type":"pong","nonce":42}"#);

        let decoded: ServerMessage = serde_json::from_str(&encoded).expect("decode pong");
        match decoded {
            ServerMessage::Pong { nonce } => assert_eq!(nonce, 42),
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn old_ping_and_pong_frames_default_nonce_to_zero() {
        let decoded: ClientMessage =
            serde_json::from_str(r#"{"type":"ping"}"#).expect("decode old ping");
        match decoded {
            ClientMessage::Ping { nonce } => assert_eq!(nonce, 0),
            other => panic!("unexpected message: {other:?}"),
        }

        let decoded: ServerMessage =
            serde_json::from_str(r#"{"type":"pong"}"#).expect("decode old pong");
        match decoded {
            ServerMessage::Pong { nonce } => assert_eq!(nonce, 0),
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn old_pty_frames_default_bytes_dropped_to_zero() {
        let decoded: ServerMessage = serde_json::from_str(
            r#"{"type":"pty_created","pty_id":"pty-1","shell":"/bin/sh","cols":80,"rows":24,"resume_token":"token","resumed":false}"#,
        )
        .expect("decode old pty_created");

        match decoded {
            ServerMessage::PtyCreated { bytes_dropped, .. } => assert_eq!(bytes_dropped, 0),
            other => panic!("unexpected message: {other:?}"),
        }

        let decoded: PtySessionInfo = serde_json::from_str(
            r#"{"pty_id":"pty-1","shell":"/bin/sh","cols":80,"rows":24,"resume_token":"token"}"#,
        )
        .expect("decode old pty info");
        assert_eq!(decoded.bytes_dropped, 0);
    }

    /// Serial tests that mutate the `DUMBPIPEX_RELAY_URL` env var. Run with
    /// `cargo test -p dumbpipex-core -- --test-threads=1` to avoid races, or
    /// rely on the unique `serial_test`-style helper below which guards the
    /// env mutation with a process-wide mutex.
    mod relay_url_env_tests {
        use super::*;
        use std::sync::Mutex;

        // Single global guard so the env-mutating tests never race with
        // each other (cargo runs tests in parallel by default).
        static ENV_LOCK: Mutex<()> = Mutex::new(());

        fn with_env_lock<R>(f: impl FnOnce() -> R) -> R {
            let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            f()
        }

        fn clear_env() {
            // SAFETY: tests in this module serialize access through ENV_LOCK.
            unsafe {
                std::env::remove_var(RELAY_URL_ENV);
            }
        }

        fn set_env(value: &str) {
            // SAFETY: tests in this module serialize access through ENV_LOCK.
            unsafe {
                std::env::set_var(RELAY_URL_ENV, value);
            }
        }

        #[test]
        fn default_relay_url_is_official_iroh() {
            with_env_lock(|| {
                clear_env();
                let url = resolve_relay_url().expect("default relay URL parses");
                let s = url.to_string();
                // The official number0 NA-East relay, mirroring
                // `iroh::defaults::NA_EAST_RELAY_HOSTNAME`.
                assert!(
                    s.starts_with("https://use1-1.relay.n0.iroh-canary.iroh.link"),
                    "expected default to point at official iroh relay, got {s}"
                );
            });
        }

        #[test]
        fn env_var_overrides_default() {
            with_env_lock(|| {
                set_env("https://relay.example.com");
                let url = resolve_relay_url().expect("override parses");
                // iroh's RelayUrl normalizes by adding a trailing slash.
                assert_eq!(url.to_string(), "https://relay.example.com/");
                clear_env();
            });
        }

        #[test]
        fn empty_env_var_falls_back_to_default() {
            with_env_lock(|| {
                set_env("   ");
                let url = resolve_relay_url().expect("whitespace env falls back to default");
                assert!(url
                    .to_string()
                    .starts_with("https://use1-1.relay.n0.iroh-canary.iroh.link"));
                clear_env();
            });
        }

        #[test]
        fn invalid_env_var_returns_error() {
            with_env_lock(|| {
                set_env("not a url");
                let err = resolve_relay_url().expect_err("invalid URL should error");
                let msg = format!("{err:?}");
                assert!(
                    msg.contains(RELAY_URL_ENV),
                    "error should mention {RELAY_URL_ENV}, got: {msg}"
                );
                clear_env();
            });
        }
    }
}
