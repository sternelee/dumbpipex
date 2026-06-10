use std::fs;
use std::io::{ErrorKind, Write};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use dumbpipex_core::{
    read_frame, resolve_relay_url, write_frame, ClientMessage, ConnectTicket, PtySessionInfo,
    ServerMessage, ALPN,
};
use iroh::endpoint::{presets, Connection};
use iroh::{Endpoint, SecretKey};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const REMOTE_EVENT: &str = "remote-event";

#[derive(Default)]
struct RemoteManager {
    inner: Arc<RemoteManagerInner>,
}

#[derive(Default)]
struct RemoteManagerInner {
    session: Mutex<Option<RemoteSession>>,
}

struct RemoteSession {
    endpoint: Endpoint,
    connection: Connection,
    writer: mpsc::Sender<ClientMessage>,
    read_only: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RemoteEvent {
    Disconnected {
        reason: Option<String>,
    },
    PtyCreated {
        pty_id: String,
        shell: String,
        cols: u16,
        rows: u16,
        resume_token: String,
        resumed: bool,
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
        nonce: u64,
    },
}

#[derive(Debug, Clone, Serialize)]
struct ConnectTicketResponse {
    agent_name: String,
    label: String,
    sessions: Vec<PtySessionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredTicket {
    ticket: String,
}

#[tauri::command]
async fn connect_ticket(
    app: AppHandle,
    state: State<'_, RemoteManager>,
    ticket: String,
    viewer: Option<bool>,
) -> Result<ConnectTicketResponse, String> {
    tokio::time::timeout(
        Duration::from_secs(75),
        connect_ticket_inner(app, state, ticket, viewer),
    )
    .await
    .map_err(|_| "connect_ticket timed out after 75s".to_string())?
}

async fn connect_ticket_inner(
    app: AppHandle,
    state: State<'_, RemoteManager>,
    ticket: String,
    viewer: Option<bool>,
) -> Result<ConnectTicketResponse, String> {
    disconnect_inner(&state.inner)
        .await
        .map_err(err_to_string)?;

    let ticket = ConnectTicket::from_str(&ticket).map_err(err_to_string)?;
    let read_only = viewer.unwrap_or(false);

    info!("binding iroh endpoint...");
    let secret_key = SecretKey::generate();
    let relay_url = resolve_relay_url().map_err(err_to_string)?;
    info!(relay_url = %relay_url, "configuring iroh relay");
    let relay_map =
        iroh::RelayMap::from(relay_url).with_auth_token(secret_key.public().to_string());
    let endpoint = tokio::time::timeout(
        Duration::from_secs(10),
        Endpoint::builder(presets::N0)
            .secret_key(secret_key)
            .relay_mode(iroh::RelayMode::Custom(relay_map))
            .bind(),
    )
    .await
    .map_err(|_| "iroh endpoint bind timed out after 10s".to_string())?
    .context("failed to bind local iroh endpoint")
    .map_err(err_to_string)?;
    info!("endpoint bound, waiting for online...");

    tokio::time::timeout(Duration::from_secs(15), endpoint.online())
        .await
        .map_err(|_| "iroh endpoint online timed out after 15s".to_string())?;
    info!("endpoint online");

    info!(
        "connecting to remote agent at {:?}...",
        ticket.endpoint_addr
    );
    let connection = tokio::time::timeout(
        Duration::from_secs(20),
        endpoint.connect(ticket.endpoint_addr.clone(), ALPN),
    )
    .await
    .map_err(|_| "iroh connect timed out after 20s".to_string())?
    .context("failed to connect to remote agent")
    .map_err(err_to_string)?;
    info!("connection established");
    let (mut send, mut recv) = connection
        .open_bi()
        .await
        .context("failed to open control stream")
        .map_err(err_to_string)?;

    write_frame(
        &mut send,
        &ClientMessage::Hello {
            client_name: String::from("dumbpipex-tauri"),
        },
    )
    .await
    .map_err(err_to_string)?;

    let hello = read_frame::<_, ServerMessage>(&mut recv)
        .await
        .context("failed to read remote hello")
        .map_err(err_to_string)?;
    let agent_name = match hello {
        ServerMessage::Hello { agent_name } => agent_name,
        message => return Err(format!("unexpected first message from agent: {message:?}")),
    };

    write_frame(&mut send, &ClientMessage::ListPtys)
        .await
        .context("failed to request existing PTYs")
        .map_err(err_to_string)?;
    let sessions = match read_frame::<_, ServerMessage>(&mut recv)
        .await
        .context("failed to read PTY list")
        .map_err(err_to_string)?
    {
        ServerMessage::PtyList { ptys } => ptys,
        message => {
            return Err(format!(
                "unexpected PTY list response from agent: {message:?}"
            ))
        }
    };

    let (writer_tx, mut writer_rx) = mpsc::channel::<ClientMessage>(128);
    tauri::async_runtime::spawn(async move {
        while let Some(message) = writer_rx.recv().await {
            if let Err(err) = write_frame(&mut send, &message).await {
                warn!("failed to send command to remote agent: {err:#}");
                break;
            }
        }
    });

    let inner = state.inner.clone();
    let app_for_reader = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            match read_frame::<_, ServerMessage>(&mut recv).await {
                Ok(message) => {
                    if let Err(err) = emit_server_message(&app_for_reader, message) {
                        warn!("failed to emit remote event: {err:#}");
                    }
                }
                Err(err) => {
                    let reason = Some(err.to_string());
                    let _ = emit_event(&app_for_reader, RemoteEvent::Disconnected { reason });
                    let _ = disconnect_inner(&inner).await;
                    break;
                }
            }
        }
    });

    {
        let mut guard = state.inner.session.lock().await;
        *guard = Some(RemoteSession {
            endpoint,
            connection,
            writer: writer_tx,
            read_only,
        });
    }

    Ok(ConnectTicketResponse {
        agent_name,
        label: ticket.label,
        sessions,
    })
}

#[tauri::command]
async fn disconnect_ticket(state: State<'_, RemoteManager>) -> Result<(), String> {
    disconnect_inner(&state.inner).await.map_err(err_to_string)
}

#[tauri::command]
async fn save_ticket(app: AppHandle, ticket: String) -> Result<(), String> {
    save_ticket_inner(&app, &ticket).map_err(err_to_string)
}

#[tauri::command]
async fn load_ticket(app: AppHandle) -> Result<Option<String>, String> {
    load_ticket_inner(&app).map_err(err_to_string)
}

#[tauri::command]
async fn clear_ticket(app: AppHandle) -> Result<(), String> {
    clear_ticket_inner(&app).map_err(err_to_string)
}

#[tauri::command]
async fn create_pty(
    state: State<'_, RemoteManager>,
    shell: Option<String>,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    send_command(&state.inner, ClientMessage::CreatePty { shell, cols, rows })
        .await
        .map_err(err_to_string)
}

#[tauri::command]
async fn resume_pty(
    state: State<'_, RemoteManager>,
    pty_id: String,
    resume_token: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    send_command(
        &state.inner,
        ClientMessage::ResumePty {
            pty_id,
            resume_token,
            cols,
            rows,
        },
    )
    .await
    .map_err(err_to_string)
}

#[tauri::command]
async fn send_pty_input(
    state: State<'_, RemoteManager>,
    pty_id: String,
    data: String,
) -> Result<(), String> {
    send_command(&state.inner, ClientMessage::PtyInput { pty_id, data })
        .await
        .map_err(err_to_string)
}

#[tauri::command]
async fn resize_pty(
    state: State<'_, RemoteManager>,
    pty_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    send_command(
        &state.inner,
        ClientMessage::ResizePty { pty_id, cols, rows },
    )
    .await
    .map_err(err_to_string)
}

#[tauri::command]
async fn close_pty(state: State<'_, RemoteManager>, pty_id: String) -> Result<(), String> {
    send_command(&state.inner, ClientMessage::ClosePty { pty_id })
        .await
        .map_err(err_to_string)
}

#[tauri::command]
async fn ping_remote(state: State<'_, RemoteManager>, nonce: u64) -> Result<(), String> {
    send_command(&state.inner, ClientMessage::Ping { nonce })
        .await
        .map_err(err_to_string)
}

#[tauri::command]
async fn upload_file(
    state: State<'_, RemoteManager>,
    name: String,
    mime: String,
    data: String,
) -> Result<(), String> {
    let size = data.len() as u64;
    send_command(
        &state.inner,
        ClientMessage::Upload {
            name,
            mime,
            size,
            data,
        },
    )
    .await
    .map_err(err_to_string)
}

async fn send_command(inner: &RemoteManagerInner, message: ClientMessage) -> Result<()> {
    let (sender, read_only) = {
        let guard = inner.session.lock().await;
        let session = guard.as_ref().context("not connected to a remote agent")?;
        (session.writer.clone(), session.read_only)
    };
    if read_only && is_read_only_blocked(&message) {
        return Err(anyhow!(
            "viewer mode is read-only; this action is not allowed"
        ));
    }
    sender
        .send(message)
        .await
        .context("failed to queue command")
}

fn is_read_only_blocked(message: &ClientMessage) -> bool {
    matches!(
        message,
        ClientMessage::CreatePty { .. }
            | ClientMessage::PtyInput { .. }
            | ClientMessage::ResizePty { .. }
            | ClientMessage::ClosePty { .. }
            | ClientMessage::Upload { .. }
    )
}

fn ticket_file_path(app: &AppHandle) -> Result<PathBuf> {
    Ok(app
        .path()
        .app_data_dir()
        .context("failed to resolve app data directory")?
        .join("ticket.json"))
}

fn save_ticket_inner(app: &AppHandle, ticket: &str) -> Result<()> {
    let ticket = ticket.trim();
    ConnectTicket::from_str(ticket).context("refusing to save invalid ticket")?;
    let path = ticket_file_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let payload = serde_json::to_string(&StoredTicket {
        ticket: ticket.to_string(),
    })
    .context("failed to serialize stored ticket")?;

    let mut options = fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        options.mode(0o600);
    }
    let mut file = options
        .open(&path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    file.write_all(payload.as_bytes())
        .with_context(|| format!("failed to write {}", path.display()))?;
    file.write_all(b"\n")
        .with_context(|| format!("failed to finish {}", path.display()))?;

    #[cfg(unix)]
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("failed to chmod {}", path.display()))?;

    Ok(())
}

fn load_ticket_inner(app: &AppHandle) -> Result<Option<String>> {
    let path = ticket_file_path(app)?;
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err).with_context(|| format!("failed to read {}", path.display())),
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let ticket = serde_json::from_str::<StoredTicket>(trimmed)
        .map(|stored| stored.ticket)
        .unwrap_or_else(|_| trimmed.to_string());
    let ticket = ticket.trim().to_string();
    ConnectTicket::from_str(&ticket).context("stored ticket is invalid")?;
    Ok(Some(ticket))
}

fn clear_ticket_inner(app: &AppHandle) -> Result<()> {
    let path = ticket_file_path(app)?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("failed to remove {}", path.display())),
    }
}

async fn disconnect_inner(inner: &RemoteManagerInner) -> Result<()> {
    let maybe_session = {
        let mut guard = inner.session.lock().await;
        guard.take()
    };
    if let Some(session) = maybe_session {
        session.connection.close(0u8.into(), b"client disconnect");
        session.endpoint.close().await;
    }
    Ok(())
}

fn emit_server_message(app: &AppHandle, message: ServerMessage) -> Result<()> {
    let event = match message {
        ServerMessage::Hello { .. } => return Ok(()),
        ServerMessage::PtyList { .. } => return Ok(()),
        ServerMessage::PtyCreated {
            pty_id,
            shell,
            cols,
            rows,
            resume_token,
            resumed,
            bytes_dropped,
        } => RemoteEvent::PtyCreated {
            pty_id,
            shell,
            cols,
            rows,
            resume_token,
            resumed,
            bytes_dropped,
        },
        ServerMessage::PtyOutput { pty_id, data } => RemoteEvent::PtyOutput { pty_id, data },
        ServerMessage::PtyExited { pty_id, exit_code } => {
            RemoteEvent::PtyExited { pty_id, exit_code }
        }
        ServerMessage::PtyDetached { pty_id, reason } => {
            RemoteEvent::PtyDetached { pty_id, reason }
        }
        ServerMessage::Error { message } => RemoteEvent::Error { message },
        ServerMessage::UploadAccepted { name, path } => RemoteEvent::UploadAccepted { name, path },
        ServerMessage::UploadError { name, message } => RemoteEvent::UploadError { name, message },
        ServerMessage::Pong { nonce } => RemoteEvent::Pong { nonce },
    };
    emit_event(app, event)
}

fn emit_event(app: &AppHandle, event: RemoteEvent) -> Result<()> {
    app.emit(REMOTE_EVENT, event)
        .context("failed to emit tauri event")
}

fn err_to_string(err: impl ToString) -> String {
    err.to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Android 上必须显式初始化 rustls crypto provider，否则 HTTP/TLS 请求会 panic
    #[cfg(target_os = "android")]
    {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }

    tauri::Builder::default()
        .manage(RemoteManager::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            connect_ticket,
            disconnect_ticket,
            save_ticket,
            load_ticket,
            clear_ticket,
            create_pty,
            resume_pty,
            send_pty_input,
            resize_pty,
            close_pty,
            ping_remote,
            upload_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
