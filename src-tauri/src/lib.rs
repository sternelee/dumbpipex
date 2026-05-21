use std::str::FromStr;
use std::sync::Arc;

use std::time::Duration;

use anyhow::{Context, Result};
use dumbpipex_core::{
    ALPN, ClientMessage, ConnectTicket, PtySessionInfo, ServerMessage, read_frame, write_frame,
};
use iroh::Endpoint;
use iroh::endpoint::{Connection, presets};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{Mutex, mpsc};
use tracing::{info, warn};

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
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RemoteEvent {
    Disconnected { reason: Option<String> },
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
    Error { message: String },
}

#[derive(Debug, Clone, Serialize)]
struct ConnectTicketResponse {
    agent_name: String,
    label: String,
    sessions: Vec<PtySessionInfo>,
}

#[tauri::command]
async fn connect_ticket(
    app: AppHandle,
    state: State<'_, RemoteManager>,
    ticket: String,
) -> Result<ConnectTicketResponse, String> {
    disconnect_inner(&state.inner).await.map_err(err_to_string)?;

    let ticket = ConnectTicket::from_str(&ticket).map_err(err_to_string)?;

    info!("binding iroh endpoint...");
    let endpoint = tokio::time::timeout(
        Duration::from_secs(10),
        Endpoint::bind(presets::N0),
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

    info!("connecting to remote agent at {:?}...", ticket.endpoint_addr);
    let connection = endpoint
        .connect(ticket.endpoint_addr.clone(), ALPN)
        .await
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
        message => return Err(format!("unexpected PTY list response from agent: {message:?}")),
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
async fn create_pty(
    state: State<'_, RemoteManager>,
    shell: Option<String>,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    send_command(
        &state.inner,
        ClientMessage::CreatePty { shell, cols, rows },
    )
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
    send_command(&state.inner, ClientMessage::ResizePty { pty_id, cols, rows })
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
async fn ping_remote(state: State<'_, RemoteManager>) -> Result<(), String> {
    send_command(&state.inner, ClientMessage::Ping)
        .await
        .map_err(err_to_string)
}

async fn send_command(inner: &RemoteManagerInner, message: ClientMessage) -> Result<()> {
    let sender = {
        let guard = inner.session.lock().await;
        guard
            .as_ref()
            .map(|session| session.writer.clone())
            .context("not connected to a remote agent")?
    };
    sender
        .send(message)
        .await
        .context("failed to queue command")
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
        } => RemoteEvent::PtyCreated {
            pty_id,
            shell,
            cols,
            rows,
            resume_token,
            resumed,
        },
        ServerMessage::PtyOutput { pty_id, data } => RemoteEvent::PtyOutput { pty_id, data },
        ServerMessage::PtyExited { pty_id, exit_code } => {
            RemoteEvent::PtyExited { pty_id, exit_code }
        }
        ServerMessage::Error { message } => RemoteEvent::Error { message },
        ServerMessage::Pong => return Ok(()),
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
            create_pty,
            resume_pty,
            send_pty_input,
            resize_pty,
            close_pty,
            ping_remote
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
