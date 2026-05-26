# Repository Guidelines

## Project Overview

Dumbpipex is a P2P remote terminal. A local CLI agent (`dumbpipex-cli`) hosts PTYs and exposes them via iroh P2P. A Tauri v2 desktop/mobile app connects with a ticket and renders the terminal UI. The design follows the sshx split: the machine with the shell owns the PTY; the app is a remote controller/viewer.

## Architecture & Data Flow

```
┌─────────────────────┐         iroh P2P          ┌──────────────────────┐
│  dumbpipex-cli      │ ◄────────────────────────► │  Tauri App           │
│  (local PTY agent)  │   length-prefixed JSON     │  (SvelteKit + Rust)  │
│                     │   URL-safe base64 bytes    │                      │
│  SessionManager     │                            │  RemoteManager       │
│  ManagedSession     │   ClientMessage ────────►  │  Tauri commands      │
│  PtyProcess         │   ◄──────── ServerMessage  │  remote-event emit   │
└─────────────────────┘                            └──────────────────────┘
```

### Three Crates

| Crate | Purpose |
|---|---|
| `crates/dumbpipex-core` | Shared protocol: ticket format, message enums, length-prefixed JSON framing, URL-safe base64 helpers, ALPN constant |
| `crates/dumbpipex-cli` | Local PTY agent: CLI args, `SessionManager`, `ManagedSession`, `PtyProcess`, secret resolution, demand mode |
| `src-tauri` (`dumbpipex`) | Tauri v2 app: SvelteKit SPA frontend + Rust backend with 8 Tauri commands |

### Frontend/Backend Boundary

- **Frontend → Rust**: `invoke("command_name", { args })` from `@tauri-apps/api/core`
- **Rust → Frontend**: `app.emit("remote-event", RemoteEvent { ... })` → `listen<RemoteEvent>("remote-event", ...)` on frontend
- **Terminal bytes**: URL-safe base64-encoded (`encode_bytes`/`decode_bytes` in core crate)

### Session Lifecycle

`idle` → `connecting` → `creating_pty` → `ready` → `disconnecting`

- On connect: `Hello` exchange → `ListPtys` → resume existing PTYs or `CreatePty`
- Multi-PTY: each PTY has its own xterm instance, tabs switch between them
- Output before mount is buffered and flushed on first render
- Disconnect only detaches — PTYs stay alive on agent 30 min (`DETACHED_SESSION_TTL`)
- Only `ClosePty` tears down the remote PTY
- Auto-reconnect: exponential backoff (max 8s), state persisted to `localStorage` under `dumbpipex:recovery-state`

## Key Directories

| Directory | Purpose |
|---|---|
| `src/routes/` | SvelteKit page components (`+page.svelte`, `+layout.ts`) |
| `src/lib/` | Shared Svelte components, types, UI definitions |
| `src-tauri/src/` | Tauri Rust backend: commands, state, event emission |
| `src-tauri/capabilities/` | Tauri permission config for the main window |
| `crates/dumbpipex-core/src/` | Shared protocol: messages, ticket, frame codec, base64 |
| `crates/dumbpipex-cli/src/` | Local agent: PTY spawning, session management, CLI |
| `docs/` | Project documentation |
| `static/` | Static assets (`favicon.png`, SVGs) |

## Development Commands

| Task | Command |
|---|---|
| Frontend dev server | `npm run dev` |
| SvelteKit/TypeScript check | `npm run check` |
| Frontend production build | `npm run build` |
| Tauri desktop dev | `npm run tauri dev` |
| Tauri desktop build | `npm run tauri build` |
| Rust workspace check | `cargo check --workspace` |
| Rust workspace test | `cargo test --workspace` |
| Single Rust test | `cargo test -p <crate> <test_name>` |
| Run local agent | `cargo run -p dumbpipex-cli` |
| Agent with persistent ticket | `cargo run -p dumbpipex-cli -- --persistent-ticket` |
| Agent with custom shell | `cargo run -p dumbpipex-cli -- --shell /bin/zsh` |

**Package manager**: `pnpm` (workspace root has `pnpm-workspace.yaml`). Scripts in `package.json` use `npm run`, and `tauri.conf.json` calls `npm run dev`/`npm run build` for `beforeDevCommand`/`beforeBuildCommand`.

## Code Conventions & Common Patterns

### Svelte 5 Frontend

- **Rune-style state only**: `$state(...)`, `$derived(...)`, `$props()` — no Svelte stores, no context API
- **Props**: Components use `let { prop1, prop2 }: { ... } = $props()` with explicit type annotations
- **Callback props**: Actions flow up via callback props (e.g. `onDisconnect`, `onCreatePty`, `onSendShortcut`)
- **No SSR**: `export const ssr = false` in `+layout.ts`; the app is a static SPA loaded by Tauri
- **`$lib` alias**: SvelteKit auto-aliases `src/lib` to `$lib`

### Tauri Bridge

- Import commands: `import { invoke } from "@tauri-apps/api/core"`
- Call pattern: `await invoke("connect_ticket", { ticket: "..." })`
- Listen for events: `import { listen } from "@tauri-apps/api/event"` then `listen<RemoteEvent>("remote-event", handler)`
- Event type is discriminated union with `tag = "type"` (snake_case): `{ type: "pty_output", pty_id, data }`

### Rust Backend

- **State management**: `Arc<Mutex<Option<RemoteSession>>>` — a single shared `RemoteManager` managed via Tauri state
- **Tauri commands**: `#[tauri::command]` functions take `State<'_, RemoteManager>` and return `Result<T, String>`
- **Error handling**: `anyhow::Result` internally, `.map_err(err_to_string)` to convert to `Result<T, String>` at command boundary
- **Async**: `tokio::spawn` for background reader/writer loops, `mpsc::channel(128)` for command queuing
- **Application setup in `lib.rs`**: `main.rs` is only a binary entrypoint; all wiring (plugins, commands, state) goes in `lib.rs`
- **Platform guards**: `#[cfg_attr(mobile, tauri::mobile_entry_point)]`, `#[cfg(target_os = "android")]` for rustls init
- **Capabilities**: `src-tauri/capabilities/default.json` must be updated when adding plugins or features

### Protocol (dumbpipex-core)

- **Framing**: length-prefixed JSON — `write_frame` writes u32 size + JSON payload; `read_frame` reads u32 size + deserializes
- **Serde tagged enums**: `#[serde(tag = "type", rename_all = "snake_case")]` on `ClientMessage` and `ServerMessage`
- **Terminal bytes**: `encode_bytes(&[u8]) -> String` (URL-safe base64, no padding); `decode_bytes(&str) -> Result<Vec<u8>>`
- **Ticket**: URL-safe base64-encoded JSON `{ version, label, endpoint_addr }`, parsed via `FromStr`

### Styling

- **Scoped `<style>` blocks** in each Svelte component — no Tailwind, no global CSS framework
- **CSS custom properties**: e.g. `--app-vh` (visual viewport height), theme colors via `theme` prop objects
- **Dark terminal-first palette**: dark backgrounds throughout, xterm.js with custom themes
- **Mobile**: `safe-area-inset-*` environment variables, responsive breakpoints via `compactLayout`/`phoneCompactLayout`

### Mobile Patterns

- **Keyboard detection**: compare `window.innerHeight` vs `visualViewport.height` with 12% threshold
- **Viewport height**: `syncViewportHeight()` sets `--app-vh` CSS variable and `html`/`body` height to `visualViewport.height`
- **Session mode detection**: heuristic parses command text to guess mode (`shell`, `vim`, `claude`, `pager`, `repl`, `monitor`)
- **Mode-specific shortcut bars**: `mobileModeShortcuts: Record<SessionMode, MobileShortcutButton[]>`
- **Modifier keys**: sticky modifiers (Ctrl, Alt, Esc) persist until next keystroke
- **Android manifest**: `android:windowSoftInputMode="adjustResize|stateHidden"`

## Important Files

| File | Purpose |
|---|---|
| `src/routes/+page.svelte` | Main app state machine: connection, PTY lifecycle, auto-reconnect, event handling, recovery state persistence |
| `src/routes/+layout.ts` | Disables SSR (`export const ssr = false`) |
| `src/lib/terminal-ui.ts` | All shared types (`RemoteEvent`, `PtySession`, `SessionPhase`, `SessionMode`, `ShortcutButton`), theme definitions, shortcut sections, mobile mode shortcuts |
| `src/lib/remote-pty-types.ts` | `RemotePtyApi` interface (xterm wrapper contract) and `RemotePtyTheme` type |
| `src/lib/RemotePtyPane.svelte` | xterm.js terminal component: creates `Terminal`, attaches addons (fit, search, web-links), handles base64 input, clipboard, focus |
| `src/lib/SessionWorkspace.svelte` | Connected UI shell: layout, tabs, terminal panes, theme/font size, mobile panels, keyboard detection |
| `src/lib/SessionBar.svelte` | PTY tab bar with add/close/swipe, dropdown menu for close/rename |
| `src/lib/SessionHeader.svelte` | Header with status, agent name, session phase indicators |
| `src/lib/ConnectionHome.svelte` | Disconnected UI: ticket input, shell override, auto-reconnect toggle, connect button |
| `src/lib/MobileShortcutBar.svelte` | Mode-adaptive shortcut buttons with sticky modifiers, swipe gestures |
| `src/lib/MobileSheet.svelte` | Reusable mobile bottom-sheet panel |
| `src/lib/DesktopShortcuts.svelte` | Desktop shortcut button grid |
| `src/lib/SearchPanel.svelte` | xterm search UI (find next/previous) |
| `src/lib/DisplayPanel.svelte` | Theme picker and font size controls |
| `src-tauri/src/lib.rs` | Tauri commands (8), `RemoteManager` state, iroh connection, `RemoteEvent` emission, `run()` entry |
| `src-tauri/src/main.rs` | Binary entrypoint: calls `tauri_build::build()` then `dumbpipex_lib::run()` |
| `crates/dumbpipex-core/src/lib.rs` | `ClientMessage`/`ServerMessage` enums, `ConnectTicket`, `write_frame`/`read_frame`, `encode_bytes`/`decode_bytes`, `ALPN` constant |
| `crates/dumbpipex-cli/src/main.rs` | `Args` (clap), `SessionManager`, `ManagedSession`, `PtyProcess`, secret resolution, demand mode, backlog, sweeper |

## Runtime/Tooling Preferences

- **Runtime**: Tauri v2 (Rust backend + WebView frontend), requires system WebView (WebKit on macOS, WebView2 on Windows)
- **Package manager**: `pnpm` for Node dependencies; `cargo` for Rust
- **Build**: Vite dev server on fixed port `1420`, HMR on `1421` when `TAURI_DEV_HOST` is set
- **TypeScript**: strict mode, bundler module resolution, `svelte-kit sync` before type checking
- **Svelte**: Svelte 5 with rune syntax, `svelte-check` for type validation
- **Rust edition**: 2021, resolver 2

## Testing & QA

- **Rust tests**: `cargo test --workspace` or per-crate `cargo test -p dumbpipex-cli`
- **Frontend type checking**: `npm run check` runs `svelte-check --tsconfig ./tsconfig.json`
- **Build verification**: `npm run build` for frontend, `cargo build` for Rust
- **Mobile**: `vconsole` injected in `+layout.ts` for debugging on physical devices
- **No frontend test framework** is configured — testing is primarily manual (type check + build + device testing)
