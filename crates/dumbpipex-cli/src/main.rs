use std::collections::{HashMap, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow};
use clap::{ArgAction, Parser};
use dumbpipex_core::{
    ALPN, ClientMessage, ConnectTicket, PtySessionInfo, ServerMessage, decode_bytes, encode_bytes,
    read_frame, write_frame,
};
use iroh::endpoint::presets;
use iroh::{Endpoint, EndpointAddr, SecretKey, endpoint::Connection};
use iroh::Watcher;
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use tokio::sync::{Mutex, mpsc};
use tracing::{error, info, warn};

const DETACHED_SESSION_TTL: Duration = Duration::from_secs(30 * 60);
const BACKLOG_LIMIT_BYTES: usize = 256 * 1024;

#[derive(Parser, Debug)]
#[command(author, version, about = "Local PTY agent for dumbpipex")]
struct Args {
    #[arg(long)]
    shell: Option<String>,

    #[arg(long, default_value_t = default_agent_name())]
    name: String,

    #[arg(long, conflicts_with_all = ["secret_file", "persistent_ticket"])]
    secret: Option<String>,

    #[arg(long, value_name = "PATH", conflicts_with_all = ["secret", "persistent_ticket"])]
    secret_file: Option<PathBuf>,

    #[arg(long, action = ArgAction::SetTrue, conflicts_with_all = ["secret", "secret_file"])]
    persistent_ticket: bool,

    #[arg(long, action = ArgAction::SetTrue)]
    demand: bool,

    #[arg(long, action = ArgAction::SetTrue, conflicts_with_all = ["demand", "demand_child"])]
    stop_demand: bool,

    #[arg(long, hide = true, action = ArgAction::SetTrue)]
    demand_child: bool,

    #[arg(long, hide = true)]
    ticket_output: Option<PathBuf>,
}

struct PtyProcess {
    master: Arc<StdMutex<Box<dyn MasterPty + Send>>>,
    writer: Arc<StdMutex<Box<dyn Write + Send>>>,
    child: Arc<StdMutex<Box<dyn portable_pty::Child + Send + Sync>>>,
    shutting_down: Arc<AtomicBool>,
    reader_thread: Option<thread::JoinHandle<()>>,
    wait_thread: Option<thread::JoinHandle<()>>,
}

enum PtyEvent {
    Output { pty_id: String, data: String },
    Exited { pty_id: String, exit_code: Option<i32> },
    Error { pty_id: String, message: String },
}

#[derive(Clone)]
struct SessionManager {
    inner: Arc<SessionManagerInner>,
}

struct SessionManagerInner {
    default_shell: String,
    sessions: Mutex<HashMap<String, Arc<ManagedSession>>>,
    next_pty_id: AtomicUsize,
    event_tx: mpsc::Sender<PtyEvent>,
}

struct ManagedSession {
    pty_id: String,
    shell: String,
    resume_token: String,
    process: StdMutex<Option<PtyProcess>>,
    state: Mutex<ManagedState>,
}

struct ManagedState {
    cols: u16,
    rows: u16,
    backlog: VecDeque<String>,
    backlog_bytes: usize,
    attached: Option<AttachedClient>,
    detached_at: Option<Instant>,
    exited: bool,
    exit_code: Option<i32>,
}

#[derive(Clone)]
struct AttachedClient {
    client_id: String,
    sender: mpsc::Sender<ServerMessage>,
}

struct DispatchTarget {
    client_id: String,
    sender: mpsc::Sender<ServerMessage>,
}

#[derive(Clone)]
struct PtyCreatedDispatch {
    pty_id: String,
    shell: String,
    cols: u16,
    rows: u16,
    resume_token: String,
    resumed: bool,
}

struct ResolvedSecret {
    key: SecretKey,
    encoded: String,
}

impl PtyProcess {
    fn spawn(
        shell: String,
        cols: u16,
        rows: u16,
        pty_id: String,
        event_tx: mpsc::Sender<PtyEvent>,
    ) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("failed to allocate PTY")?;

        let mut cmd = CommandBuilder::new(shell);
        cmd.env("TERM", "xterm-256color");
        let child = pair
            .slave
            .spawn_command(cmd)
            .context("failed to spawn shell")?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .context("failed to clone PTY reader")?;
        let writer = pair.master.take_writer().context("failed to take PTY writer")?;

        let child = Arc::new(StdMutex::new(child));
        let child_for_wait = child.clone();
        let shutting_down = Arc::new(AtomicBool::new(false));

        let reader_thread = thread::spawn({
            let event_tx = event_tx.clone();
            let pty_id = pty_id.clone();
            let shutting_down = shutting_down.clone();
            move || {
                let mut buf = [0_u8; 8192];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(size) => {
                            if event_tx
                                .blocking_send(PtyEvent::Output {
                                    pty_id: pty_id.clone(),
                                    data: encode_bytes(&buf[..size]),
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(err) => {
                            if shutting_down.load(Ordering::Relaxed) {
                                break;
                            }
                            let _ = event_tx.blocking_send(PtyEvent::Error {
                                pty_id: pty_id.clone(),
                                message: format!("PTY read failed for {pty_id}: {err}"),
                            });
                            break;
                        }
                    }
                }
            }
        });

        let wait_thread = thread::spawn(move || {
            let exit_code = child_for_wait
                .lock()
                .ok()
                .and_then(|mut child| child.wait().ok())
                .map(|status| status.exit_code() as i32);
            let _ = event_tx.blocking_send(PtyEvent::Exited { pty_id, exit_code });
        });

        Ok(Self {
            master: Arc::new(StdMutex::new(pair.master)),
            writer: Arc::new(StdMutex::new(writer)),
            child,
            shutting_down,
            reader_thread: Some(reader_thread),
            wait_thread: Some(wait_thread),
        })
    }

    fn write_input(&self, data: &[u8]) -> Result<()> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| anyhow!("writer mutex poisoned"))?;
        writer
            .write_all(data)
            .context("failed to write data into PTY")?;
        writer.flush().context("failed to flush PTY input")?;
        Ok(())
    }

    fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.master
            .lock()
            .map_err(|_| anyhow!("master mutex poisoned"))?
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("failed to resize PTY")?;
        Ok(())
    }

    fn shutdown_blocking(mut self) {
        self.shutting_down.store(true, Ordering::Relaxed);

        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
        }

        drop(self.writer);
        drop(self.master);

        if let Some(handle) = self.reader_thread.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.wait_thread.take() {
            let _ = handle.join();
        }
    }
}

impl ManagedSession {
    fn new(
        pty_id: String,
        shell: String,
        cols: u16,
        rows: u16,
        resume_token: String,
        process: PtyProcess,
        client_id: String,
        sender: mpsc::Sender<ServerMessage>,
    ) -> Self {
        Self {
            pty_id,
            shell,
            resume_token,
            process: StdMutex::new(Some(process)),
            state: Mutex::new(ManagedState {
                cols,
                rows,
                backlog: VecDeque::new(),
                backlog_bytes: 0,
                attached: Some(AttachedClient { client_id, sender }),
                detached_at: None,
                exited: false,
                exit_code: None,
            }),
        }
    }

    async fn record_output(&self, data: String) -> Option<DispatchTarget> {
        let mut state = self.state.lock().await;
        push_backlog(&mut state, data.clone());
        state.attached.as_ref().map(|attached| DispatchTarget {
            client_id: attached.client_id.clone(),
            sender: attached.sender.clone(),
        })
    }

    async fn dispatch_error(&self, _message: String) -> Option<DispatchTarget> {
        let state = self.state.lock().await;
        state.attached.as_ref().map(|attached| DispatchTarget {
            client_id: attached.client_id.clone(),
            sender: attached.sender.clone(),
        })
    }

    async fn mark_exited(&self, exit_code: Option<i32>) -> Option<DispatchTarget> {
        let mut state = self.state.lock().await;
        state.exited = true;
        state.exit_code = exit_code;
        state.detached_at = Some(Instant::now());
        let attached = state.attached.take();
        attached.map(|attached| DispatchTarget {
            client_id: attached.client_id,
            sender: attached.sender,
        })
    }

    async fn list_info(&self) -> Option<PtySessionInfo> {
        let state = self.state.lock().await;
        if state.exited {
            return None;
        }
        Some(PtySessionInfo {
            pty_id: self.pty_id.clone(),
            shell: self.shell.clone(),
            cols: state.cols,
            rows: state.rows,
            resume_token: self.resume_token.clone(),
        })
    }

    async fn resume(
        &self,
        client_id: String,
        sender: mpsc::Sender<ServerMessage>,
        cols: u16,
        rows: u16,
    ) -> Result<(PtyCreatedDispatch, Vec<String>)> {
        self.resize(cols, rows).await?;

        let mut state = self.state.lock().await;
        if state.exited {
            return Err(anyhow!("PTY {} has already exited", self.pty_id));
        }

        state.cols = cols;
        state.rows = rows;
        state.attached = Some(AttachedClient {
            client_id,
            sender,
        });
        state.detached_at = None;

        Ok((
            PtyCreatedDispatch {
                pty_id: self.pty_id.clone(),
                shell: self.shell.clone(),
                cols,
                rows,
                resume_token: self.resume_token.clone(),
                resumed: true,
            },
            state.backlog.iter().cloned().collect(),
        ))
    }

    async fn detach_if_client(&self, client_id: &str) {
        let mut state = self.state.lock().await;
        if state
            .attached
            .as_ref()
            .is_some_and(|attached| attached.client_id == client_id)
        {
            state.attached = None;
            state.detached_at = Some(Instant::now());
        }
    }

    async fn should_expire(&self, now: Instant) -> bool {
        let state = self.state.lock().await;
        match state.detached_at {
            Some(detached_at) => now.duration_since(detached_at) >= DETACHED_SESSION_TTL,
            None => false,
        }
    }

    async fn write_input(&self, data: &[u8]) -> Result<()> {
        let state = self.state.lock().await;
        if state.exited {
            return Err(anyhow!("PTY {} has already exited", self.pty_id));
        }
        drop(state);

        let guard = self
            .process
            .lock()
            .map_err(|_| anyhow!("process mutex poisoned"))?;
        guard
            .as_ref()
            .context("PTY process is no longer available")?
            .write_input(data)
    }

    async fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        {
            let state = self.state.lock().await;
            if state.exited {
                return Err(anyhow!("PTY {} has already exited", self.pty_id));
            }
        }

        {
            let guard = self
                .process
                .lock()
                .map_err(|_| anyhow!("process mutex poisoned"))?;
            guard
                .as_ref()
                .context("PTY process is no longer available")?
                .resize(cols, rows)?;
        }

        let mut state = self.state.lock().await;
        state.cols = cols;
        state.rows = rows;
        Ok(())
    }

    async fn shutdown(self: &Arc<Self>) {
        let process = match self.process.lock() {
            Ok(mut guard) => guard.take(),
            Err(_) => None,
        };
        if let Some(process) = process {
            let _ = tokio::task::spawn_blocking(move || process.shutdown_blocking()).await;
        }
    }
}

impl SessionManager {
    fn new(default_shell: String) -> (Self, mpsc::Receiver<PtyEvent>) {
        let (event_tx, event_rx) = mpsc::channel::<PtyEvent>(256);
        (
            Self {
                inner: Arc::new(SessionManagerInner {
                    default_shell,
                    sessions: Mutex::new(HashMap::new()),
                    next_pty_id: AtomicUsize::new(1),
                    event_tx,
                }),
            },
            event_rx,
        )
    }

    fn spawn_background_tasks(&self, mut event_rx: mpsc::Receiver<PtyEvent>) {
        let manager_for_events = self.clone();
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                match event {
                    PtyEvent::Output { pty_id, data } => manager_for_events.handle_output(pty_id, data).await,
                    PtyEvent::Exited { pty_id, exit_code } => {
                        manager_for_events.handle_exit(pty_id, exit_code).await
                    }
                    PtyEvent::Error { pty_id, message } => {
                        manager_for_events.handle_error(pty_id, message).await
                    }
                }
            }
        });

        let manager_for_sweeper = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                manager_for_sweeper.expire_detached_sessions().await;
            }
        });
    }

    async fn create_pty(
        &self,
        shell: Option<String>,
        cols: u16,
        rows: u16,
        client_id: String,
        sender: mpsc::Sender<ServerMessage>,
    ) -> Result<()> {
        let shell_name = shell.unwrap_or_else(|| self.inner.default_shell.clone());
        let pty_id = format!("pty-{}", self.inner.next_pty_id.fetch_add(1, Ordering::Relaxed));
        let resume_token = encode_bytes(&rand::random::<[u8; 16]>());
        let process = PtyProcess::spawn(
            shell_name.clone(),
            cols,
            rows,
            pty_id.clone(),
            self.inner.event_tx.clone(),
        )?;
        let session = Arc::new(ManagedSession::new(
            pty_id.clone(),
            shell_name.clone(),
            cols,
            rows,
            resume_token.clone(),
            process,
            client_id,
            sender.clone(),
        ));

        self.inner
            .sessions
            .lock()
            .await
            .insert(pty_id.clone(), session);

        send_message(
            &sender,
            ServerMessage::PtyCreated {
                pty_id,
                shell: shell_name,
                cols,
                rows,
                resume_token,
                resumed: false,
            },
        )
        .await
    }

    async fn list_ptys(&self) -> Vec<PtySessionInfo> {
        let sessions: Vec<_> = self
            .inner
            .sessions
            .lock()
            .await
            .values()
            .cloned()
            .collect();
        let mut items = Vec::new();
        for session in sessions {
            if let Some(info) = session.list_info().await {
                items.push(info);
            }
        }
        items
    }

    async fn resume_pty(
        &self,
        pty_id: String,
        resume_token: String,
        cols: u16,
        rows: u16,
        client_id: String,
        sender: mpsc::Sender<ServerMessage>,
    ) -> Result<()> {
        let session = self
            .inner
            .sessions
            .lock()
            .await
            .get(&pty_id)
            .cloned()
            .context("requested PTY does not exist")?;

        if session.resume_token != resume_token {
            return Err(anyhow!("resume token mismatch for PTY {}", session.pty_id));
        }

        let (created, backlog) = session.resume(client_id.clone(), sender.clone(), cols, rows).await?;

        if let Err(err) = send_message(
            &sender,
            ServerMessage::PtyCreated {
                pty_id: created.pty_id.clone(),
                shell: created.shell,
                cols: created.cols,
                rows: created.rows,
                resume_token: created.resume_token,
                resumed: created.resumed,
            },
        )
        .await
        {
            session.detach_if_client(&client_id).await;
            return Err(err);
        }

        for chunk in backlog {
            if let Err(err) = send_message(
                &sender,
                ServerMessage::PtyOutput {
                    pty_id: created.pty_id.clone(),
                    data: chunk,
                },
            )
            .await
            {
                session.detach_if_client(&client_id).await;
                return Err(err);
            }
        }

        Ok(())
    }

    async fn write_input(&self, pty_id: &str, data: &[u8]) -> Result<()> {
        let session = self
            .inner
            .sessions
            .lock()
            .await
            .get(pty_id)
            .cloned()
            .with_context(|| format!("received input for unknown PTY: {pty_id}"))?;
        session.write_input(data).await
    }

    async fn resize_pty(&self, pty_id: &str, cols: u16, rows: u16) -> Result<()> {
        let session = self
            .inner
            .sessions
            .lock()
            .await
            .get(pty_id)
            .cloned()
            .with_context(|| format!("received resize for unknown PTY: {pty_id}"))?;
        session.resize(cols, rows).await
    }

    async fn close_pty(&self, pty_id: &str) -> Result<()> {
        let session = self
            .inner
            .sessions
            .lock()
            .await
            .remove(pty_id)
            .with_context(|| format!("received close for unknown PTY: {pty_id}"))?;
        session.shutdown().await;
        Ok(())
    }

    async fn detach_client_sessions(&self, client_id: &str) {
        let sessions: Vec<_> = self
            .inner
            .sessions
            .lock()
            .await
            .values()
            .cloned()
            .collect();
        for session in sessions {
            session.detach_if_client(client_id).await;
        }
    }

    async fn handle_output(&self, pty_id: String, data: String) {
        let session = self
            .inner
            .sessions
            .lock()
            .await
            .get(&pty_id)
            .cloned();
        let Some(session) = session else {
            return;
        };

        if let Some(target) = session.record_output(data.clone()).await {
            if send_message(&target.sender, ServerMessage::PtyOutput { pty_id, data })
                .await
                .is_err()
            {
                session.detach_if_client(&target.client_id).await;
            }
        }
    }

    async fn handle_exit(&self, pty_id: String, exit_code: Option<i32>) {
        let session = self
            .inner
            .sessions
            .lock()
            .await
            .get(&pty_id)
            .cloned();
        let Some(session) = session else {
            return;
        };

        if let Some(target) = session.mark_exited(exit_code).await {
            let _ = send_message(
                &target.sender,
                ServerMessage::PtyExited {
                    pty_id,
                    exit_code,
                },
            )
            .await;
        }
    }

    async fn handle_error(&self, pty_id: String, message: String) {
        let session = self
            .inner
            .sessions
            .lock()
            .await
            .get(&pty_id)
            .cloned();
        let Some(session) = session else {
            return;
        };

        if let Some(target) = session.dispatch_error(message.clone()).await {
            if send_message(&target.sender, ServerMessage::Error { message })
                .await
                .is_err()
            {
                session.detach_if_client(&target.client_id).await;
            }
        }
    }

    async fn expire_detached_sessions(&self) {
        let sessions: Vec<_> = self
            .inner
            .sessions
            .lock()
            .await
            .iter()
            .map(|(pty_id, session)| (pty_id.clone(), session.clone()))
            .collect();

        let now = Instant::now();
        let mut expired = Vec::new();
        for (pty_id, session) in sessions {
            if session.should_expire(now).await {
                expired.push((pty_id, session));
            }
        }

        if expired.is_empty() {
            return;
        }

        let mut guard = self.inner.sessions.lock().await;
        for (pty_id, _) in &expired {
            guard.remove(pty_id);
        }
        drop(guard);

        for (_, session) in expired {
            session.shutdown().await;
        }
    }
}

fn push_backlog(state: &mut ManagedState, data: String) {
    state.backlog_bytes += data.len();
    state.backlog.push_back(data);
    while state.backlog_bytes > BACKLOG_LIMIT_BYTES {
        if let Some(chunk) = state.backlog.pop_front() {
            state.backlog_bytes = state.backlog_bytes.saturating_sub(chunk.len());
        } else {
            break;
        }
    }
}

async fn send_message(sender: &mpsc::Sender<ServerMessage>, message: ServerMessage) -> Result<()> {
    sender
        .send(message)
        .await
        .context("failed to queue agent event")
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| String::from("info")))
        .init();

    let args = Args::parse();

    if args.stop_demand {
        stop_demand_agent(&args)?;
        return Ok(());
    }

    let shell = args.shell.clone().unwrap_or_else(default_shell);
    let secret = resolve_secret(&args)?;

    if args.demand {
        let (ticket, pid) = spawn_demand_child(&args, &shell, &secret.encoded)?;
        if let Some(pid_file) = configured_pid_file(&args)? {
            write_text_file(&pid_file, &pid.to_string())?;
        }
        print_agent_intro(&args.name, &shell, &ticket);
        println!("mode:   demand");
        println!("pid:    {}", pid);
        return Ok(());
    }

    let endpoint = bind_endpoint(secret.key).await?;
    endpoint.online().await;
    let endpoint_addr = ticket_endpoint_addr(&endpoint, has_stable_secret_source(&args));
    let ticket = ConnectTicket::new(args.name.clone(), endpoint_addr);

    if let Some(path) = args.ticket_output.as_deref() {
        write_text_file(path, &ticket.to_string())?;
    }

    if !args.demand_child {
        print_agent_intro(&args.name, &shell, &ticket);
    }

    let (manager, event_rx) = SessionManager::new(shell);
    manager.spawn_background_tasks(event_rx);
    serve_endpoint(endpoint, manager, args.name).await
}

async fn serve_endpoint(endpoint: Endpoint, manager: SessionManager, agent_name: String) -> Result<()> {
    loop {
        let Some(connecting) = endpoint.accept().await else {
            break;
        };
        let manager = manager.clone();
        let agent_name = agent_name.clone();
        tokio::spawn(async move {
            match connecting.await {
                Ok(connection) => {
                    if let Err(err) = handle_connection(connection, manager, agent_name).await {
                        error!("{err:?}");
                    }
                }
                Err(err) => error!("failed to accept connection: {err:?}"),
            }
        });
    }

    Ok(())
}

async fn bind_endpoint(secret_key: SecretKey) -> Result<Endpoint> {
    Endpoint::builder(presets::N0)
        .secret_key(secret_key)
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await
        .context("failed to bind iroh endpoint")
}

fn ticket_endpoint_addr(endpoint: &Endpoint, stable: bool) -> EndpointAddr {
    let current = endpoint.watch_addr().get();
    if !stable {
        return current;
    }

    let relay_url = current.relay_urls().next().cloned();
    if let Some(relay_url) = relay_url {
        EndpointAddr::new(endpoint.id()).with_relay_url(relay_url)
    } else {
        current
    }
}

fn print_agent_intro(agent_name: &str, shell: &str, ticket: &ConnectTicket) {
    println!("dumbpipex local agent");
    println!("agent:  {}", agent_name);
    println!("shell:  {}", shell);
    println!("ticket: {}", ticket);
}

fn resolve_secret(args: &Args) -> Result<ResolvedSecret> {
    if let Some(secret) = args.secret.as_deref() {
        let key = parse_secret(secret)?;
        return Ok(ResolvedSecret {
            encoded: encode_secret(&key),
            key,
        });
    }

    if let Some(path) = configured_secret_file(args)?.as_deref() {
        if path.exists() {
            let raw = fs::read_to_string(path)
                .with_context(|| format!("failed to read secret file {}", path.display()))?;
            if raw.trim().is_empty() {
                let key = SecretKey::generate();
                let encoded = encode_secret(&key);
                write_text_file(path, &encoded)?;
                return Ok(ResolvedSecret { key, encoded });
            }
            let key = parse_secret(raw.trim())?;
            return Ok(ResolvedSecret {
                encoded: encode_secret(&key),
                key,
            });
        }

        let key = SecretKey::generate();
        let encoded = encode_secret(&key);
        write_text_file(path, &encoded)?;
        return Ok(ResolvedSecret { key, encoded });
    }

    let key = SecretKey::generate();
    Ok(ResolvedSecret {
        encoded: encode_secret(&key),
        key,
    })
}

fn configured_secret_file(args: &Args) -> Result<Option<PathBuf>> {
    if let Some(path) = args.secret_file.clone() {
        return Ok(Some(path));
    }
    if args.persistent_ticket {
        return Ok(Some(default_persistent_secret_path()?));
    }
    Ok(None)
}

fn has_stable_secret_source(args: &Args) -> bool {
    args.secret.is_some() || args.secret_file.is_some() || args.persistent_ticket
}

fn configured_pid_file(args: &Args) -> Result<Option<PathBuf>> {
    configured_secret_file(args)?
        .map(|path| Ok(path.with_extension("pid")))
        .transpose()
}

fn parse_secret(value: &str) -> Result<SecretKey> {
    let decoded = decode_bytes(value.trim()).context("failed to decode agent secret")?;
    let key_bytes: [u8; 32] = decoded
        .try_into()
        .map_err(|_| anyhow!("agent secret must decode to 32 bytes"))?;
    Ok(SecretKey::from_bytes(&key_bytes))
}

fn encode_secret(secret: &SecretKey) -> String {
    encode_bytes(&secret.to_bytes())
}

fn write_text_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create parent directory for {}", path.display())
            })?;
        }
    }

    let mut options = OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        options.mode(0o600);
    }

    let mut file = options
        .open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    writeln!(file, "{content}").with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn default_persistent_secret_path() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        let base = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("USERPROFILE")
                    .map(PathBuf::from)
                    .map(|home| home.join("AppData").join("Roaming"))
            })
            .context("failed to resolve APPDATA for persistent ticket storage")?;
        return Ok(base.join("dumbpipex").join("agent.secret"));
    }

    #[cfg(not(windows))]
    {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .context("failed to resolve config directory for persistent ticket storage")?;
        Ok(base.join("dumbpipex").join("agent.secret"))
    }
}

fn stop_demand_agent(args: &Args) -> Result<()> {
    let pid_file = configured_pid_file(args)?.context(
        "stop-demand requires --persistent-ticket or --secret-file so the managed pid file can be located",
    )?;
    let raw = fs::read_to_string(&pid_file)
        .with_context(|| format!("failed to read demand pid file {}", pid_file.display()))?;
    let pid: i32 = raw
        .trim()
        .parse()
        .with_context(|| format!("invalid pid stored in {}", pid_file.display()))?;

    match terminate_process(pid) {
        Ok(()) => {
            let _ = fs::remove_file(&pid_file);
            println!("stopped demand agent pid={pid}");
            Ok(())
        }
        Err(err) => {
            if process_missing_error(&err) {
                let _ = fs::remove_file(&pid_file);
            }
            Err(err)
        }
    }
}

#[cfg(unix)]
fn terminate_process(pid: i32) -> Result<()> {
    let result = unsafe { libc::kill(pid, libc::SIGTERM) };
    if result == 0 {
        return Ok(());
    }
    Err(std::io::Error::last_os_error())
        .with_context(|| format!("failed to stop demand agent pid={pid}"))
}

#[cfg(windows)]
fn terminate_process(pid: i32) -> Result<()> {
    let status = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .status()
        .context("failed to invoke taskkill")?;
    if status.success() {
        return Ok(());
    }
    Err(anyhow!("taskkill exited with status {status}"))
        .with_context(|| format!("failed to stop demand agent pid={pid}"))
}

#[cfg(unix)]
fn process_missing_error(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io_err| io_err.raw_os_error() == Some(libc::ESRCH))
    })
}

#[cfg(windows)]
fn process_missing_error(_err: &anyhow::Error) -> bool {
    false
}

fn spawn_demand_child(args: &Args, shell: &str, secret: &str) -> Result<(ConnectTicket, u32)> {
    let ticket_path = demand_ticket_path();
    let mut command = Command::new(
        std::env::current_exe().context("failed to locate dumbpipex-cli executable")?,
    );
    command
        .arg("--name")
        .arg(&args.name)
        .arg("--shell")
        .arg(shell)
        .arg("--secret")
        .arg(secret)
        .arg("--demand-child")
        .arg("--ticket-output")
        .arg(&ticket_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        command.creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS);
    }

    let mut child = command.spawn().context("failed to launch background agent")?;
    let ticket = wait_for_ticket(&ticket_path, &mut child)?;
    Ok((ticket, child.id()))
}

fn demand_ticket_path() -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("dumbpipex-ticket-{}-{suffix}.txt", std::process::id()))
}

fn wait_for_ticket(path: &Path, child: &mut std::process::Child) -> Result<ConnectTicket> {
    let deadline = Instant::now() + Duration::from_secs(10);

    loop {
        if let Ok(raw) = fs::read_to_string(path) {
            let _ = fs::remove_file(path);
            return raw.trim().parse().context("failed to parse background ticket");
        }

        if let Some(status) = child
            .try_wait()
            .context("failed to inspect background agent state")?
        {
            let _ = fs::remove_file(path);
            return Err(anyhow!("background agent exited before publishing ticket: {status}"));
        }

        if Instant::now() >= deadline {
            let _ = fs::remove_file(path);
            return Err(anyhow!("timed out waiting for background agent ticket"));
        }

        thread::sleep(Duration::from_millis(100));
    }
}

async fn handle_connection(
    connection: Connection,
    manager: SessionManager,
    agent_name: String,
) -> Result<()> {
    let (mut send, mut recv) = connection
        .accept_bi()
        .await
        .context("failed to accept control stream")?;
    let (event_tx, mut event_rx) = mpsc::channel::<ServerMessage>(128);
    let writer = tokio::spawn(async move {
        while let Some(message) = event_rx.recv().await {
            if let Err(err) = write_frame(&mut send, &message).await {
                warn!("failed to send agent event: {err:?}");
                break;
            }
        }
    });

    let client_id = format!("client-{}", encode_bytes(&rand::random::<[u8; 8]>()));

    let hello = read_frame::<_, ClientMessage>(&mut recv)
        .await
        .context("failed to read initial client hello")?;
    match hello {
        ClientMessage::Hello { client_name } => {
            info!("remote app connected: {client_name}");
            event_tx
                .send(ServerMessage::Hello { agent_name })
                .await
                .ok();
        }
        other => return Err(anyhow!("expected hello message, received {other:?}")),
    }

    loop {
        let message = match read_frame::<_, ClientMessage>(&mut recv).await {
            Ok(message) => message,
            Err(err) => {
                info!("remote app disconnected: {err:#}");
                break;
            }
        };

        let result = match message {
            ClientMessage::Hello { .. } => Ok(()),
            ClientMessage::ListPtys => {
                let ptys = manager.list_ptys().await;
                event_tx
                    .send(ServerMessage::PtyList { ptys })
                    .await
                    .context("failed to queue PTY list")
            }
            ClientMessage::CreatePty { shell, cols, rows } => {
                manager
                    .create_pty(shell, cols, rows, client_id.clone(), event_tx.clone())
                    .await
            }
            ClientMessage::ResumePty {
                pty_id,
                resume_token,
                cols,
                rows,
            } => {
                manager
                    .resume_pty(pty_id, resume_token, cols, rows, client_id.clone(), event_tx.clone())
                    .await
            }
            ClientMessage::PtyInput { pty_id, data } => {
                let payload = decode_bytes(&data)?;
                manager.write_input(&pty_id, &payload).await
            }
            ClientMessage::ResizePty { pty_id, cols, rows } => {
                manager.resize_pty(&pty_id, cols, rows).await
            }
            ClientMessage::ClosePty { pty_id } => manager.close_pty(&pty_id).await,
            ClientMessage::Ping => event_tx
                .send(ServerMessage::Pong)
                .await
                .context("failed to queue pong"),
        };

        if let Err(err) = result {
            event_tx
                .send(ServerMessage::Error {
                    message: err.to_string(),
                })
                .await
                .ok();
        }
    }

    manager.detach_client_sessions(&client_id).await;
    drop(event_tx);
    let _ = writer.await;
    connection.close(0u8.into(), b"session detached");
    Ok(())
}

fn default_agent_name() -> String {
    let user = whoami::username();
    let host = whoami::fallible::hostname().unwrap_or_else(|_| String::from("host"));
    format!("{user}@{}", host.split('.').next().unwrap_or(&host))
}

fn default_shell() -> String {
    #[cfg(unix)]
    {
        std::env::var("SHELL").unwrap_or_else(|_| String::from("/bin/sh"))
    }
    #[cfg(windows)]
    {
        std::env::var("COMSPEC").unwrap_or_else(|_| String::from("cmd.exe"))
    }
}
