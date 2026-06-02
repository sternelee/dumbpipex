# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project overview

Dumbpipex is a P2P remote terminal. A local CLI agent (`dumbpipex-cli`) hosts PTYs and exposes them via iroh P2P. A Tauri desktop/mobile app connects with a ticket and renders the terminal UI.

Three crates:
- `crates/dumbpipex-cli` — local agent, owns PTYs, accepts P2P connections
- `crates/dumbpipex-core` — shared protocol (ticket format, message types, frame codec)
- `src-tauri` — Tauri v2 Rust backend + SvelteKit frontend

## Common commands

| Task | Command |
|------|---------|
| Install JS deps (first time) | `pnpm install` |
| Frontend dev server | `npm run dev` |
| Tauri desktop dev | `npm run tauri dev` |
| Tauri desktop build | `npm run tauri build` |
| SvelteKit/TypeScript check | `npm run check` |
| Rust workspace check | `cargo check --workspace` |
| Rust workspace test | `cargo test --workspace` |
| Run local agent | `cargo run -p dumbpipex-cli` |
| Agent with persistent ticket | `cargo run -p dumbpipex-cli -- --persistent-ticket` |
| Agent with custom shell | `cargo run -p dumbpipex-cli -- --shell /bin/zsh` |
| Run single Rust test | `cargo test -p <crate> <test_name>` |

Package manager is `pnpm` (workspace root has `pnpm-workspace.yaml`). `package.json` scripts use `npm` but Tauri's `beforeDevCommand`/`beforeBuildCommand` in `tauri.conf.json` call `npm run dev` / `npm run build`. If you change package manager invocation in one place, update the other.

For Svelte component conventions, mobile patterns, and code-style rules see `AGENTS.md` (single source of truth for the contributor guide).

## Frontend architecture

- **Svelte 5** with rune-style state (`$state(...)`).
- **SvelteKit** configured as a static SPA: `adapter-static` with `fallback: "index.html"`, SSR disabled via `src/routes/+layout.ts` (`ssr = false`). Tauri loads `../build`.
- **Vite** fixed on port `1420` with HMR on `1421` when `TAURI_DEV_HOST` is set. Watcher ignores `src-tauri/**`.
- **xterm.js** (`@xterm/xterm`) with addons fit, search, web-links for terminal rendering.
- **vconsole** injected in `+layout.ts` for mobile debug.

## Backend / Rust architecture

- **Tauri command bridge**: Svelte components call Rust via `invoke()` from `@tauri-apps/api/core`. Rust exposes commands with `#[tauri::command]` registered in `tauri::generate_handler![...]` in `src-tauri/src/lib.rs`.
- **Application setup lives in `src-tauri/src/lib.rs`**, not `main.rs`. Add new commands, plugins, and wiring there.
- **Capabilities are explicit**: `src-tauri/capabilities/default.json` defines permissions for the main window. New plugin features need updates there too.
- **Android-specific**: `src-tauri/src/lib.rs` explicitly initializes `rustls::crypto::ring::default_provider()` on Android to avoid TLS panic.

## P2P protocol

- Transport: **iroh** direct P2P, ALPN `b"dumbpipex-terminal-v1"`.
- Framing: length-prefixed JSON (u32 size + payload). Defined in `crates/dumbpipex-core/src/lib.rs` via `read_frame`/`write_frame`.
- **Terminal bytes use URL-safe base64** across the Rust/browser boundary (`encode_bytes`/`decode_bytes`).
- **Ticket format**: URL-safe base64-encoded JSON containing `version`, `label`, `endpoint_addr`.

### Message types

`ClientMessage` (app → agent):
- `Hello { client_name }`
- `ListPtys`
- `CreatePty { shell, cols, rows }`
- `ResumePty { pty_id, resume_token, cols, rows }`
- `PtyInput { pty_id, data }` — data is base64-encoded bytes
- `ResizePty { pty_id, cols, rows }`
- `ClosePty { pty_id }`
- `Ping`

`ServerMessage` (agent → app):
- `Hello { agent_name }`
- `PtyList { ptys }`
- `PtyCreated { pty_id, shell, cols, rows, resume_token, resumed }`
- `PtyOutput { pty_id, data }` — data is base64-encoded bytes
- `PtyExited { pty_id, exit_code }`
- `Error { message }`
- `Pong`

Tauri backend receives `ServerMessage` over iroh, translates to `RemoteEvent`, and emits via `app.emit("remote-event", ...)`. Frontend listens with `listen<RemoteEvent>("remote-event", ...)`.

## Session and PTY lifecycle

- **Connection flow**: `connect_ticket` → `Hello` exchange → `ListPtys` → resume existing or `CreatePty`.
- **Session phases** (frontend): `idle` → `connecting` → `creating_pty` → `ready` → `disconnecting`.
- **Multi-PTY**: Each PTY has its own xterm instance. Tabs switch between PTYs. Output before mount is buffered and flushed on first render.
- **Detach/resume**: Disconnecting the app only detaches the client; PTYs stay alive on the agent for 30 min (`DETACHED_SESSION_TTL`). Reconnecting with `resume_token` restores the same PTY and replays backlog.
- **Only `ClosePty` tears down the remote PTY** — disconnect/network loss only detaches.
- **Auto-reconnect**: Exponential backoff (max 8s) on unexpected disconnect. State persisted to `localStorage` under `dumbpipex:recovery-state`.

## Agent (dumbpipex-cli) architecture

- `SessionManager` owns a `HashMap<String, Arc<ManagedSession>>` of PTYs.
- Each `ManagedSession` has: `pty_id`, `shell`, `resume_token`, `PtyProcess` (actual PTY), `ManagedState` (cols/rows, backlog, attached client).
- `PtyProcess` spawns `portable_pty` with `native_pty_system()`, runs reader/wait threads, communicates via `mpsc::Sender<PtyEvent>`.
- Backlog limit: 256KB per PTY (`BACKLOG_LIMIT_BYTES`).
- Detached session sweeper runs every 30s.
- Secret/key resolution: `--secret` → `--secret-file` → `--persistent-ticket` (auto-generates/stores in config dir) → ephemeral.
- Demand mode (`--demand`) spawns a background child process, prints ticket, keeps running.

## Mobile considerations

- Android WebView keyboard popup does not update `100vh`. `syncViewportHeight()` sets `--app-vh` CSS variable and `html/body` height to `visualViewport.height`.
- Mobile shortcut bar adapts per `SessionMode` (`shell`, `vim`, `claude`, `pager`, `repl`, `monitor`).
- Input detection heuristic in `+page.svelte` guesses mode from command text (e.g. `vim` → `vim` mode, `claude` → `claude` mode).

## Key files

| File | Purpose |
|------|---------|
| `src/routes/+page.svelte` | Main app state machine: connection, PTY lifecycle, auto-reconnect, event handling |
| `src/lib/terminal-ui.ts` | Types, themes, shortcut definitions, mobile platform helpers |
| `src/lib/remote-pty-types.ts` | `RemotePtyApi` interface (xterm wrapper) |
| `src/lib/RemotePtyPane.svelte` | xterm.js terminal component |
| `src/lib/SessionWorkspace.svelte` | Connected UI: tabs, terminal panes, controls |
| `src/lib/ConnectionHome.svelte` | Disconnected UI: ticket input, connect button |
| `src-tauri/src/lib.rs` | Tauri commands, iroh connection manager, event emission |
| `crates/dumbpipex-core/src/lib.rs` | Shared protocol: ticket, messages, frame codec, base64 helpers |
| `crates/dumbpipex-cli/src/main.rs` | Local agent: PTY spawning, session management, iroh server |
