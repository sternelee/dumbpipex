# dumbpipex

A p2p remote terminal built with **Tauri + iroh + PTY**.

## What is in this repo

- **`crates/dumbpipex-cli`**: the local agent that runs on the machine you want to control. It owns the PTY, prints an iroh-based ticket, accepts the remote app connection, forwards terminal input/output, and keeps detached PTYs alive for recovery.
- **`src-tauri` + `src/`**: the Tauri app. It connects to the agent with the ticket, creates a remote PTY, and renders the terminal UI.
- **`crates/dumbpipex-core`**: the shared Rust protocol/ticket crate used by both sides.

The design follows the same split as sshx for terminal ownership: the machine with the shell keeps the PTY locally, while the app is only a remote controller/viewer. The transport layer is direct iroh p2p instead of a central server. Both the CLI agent and Tauri app use `https://relay.leeapp.dev` as the default iroh relay service while still allowing iroh to use direct paths when available.

## Run the local agent

```bash
cargo run -p dumbpipex-cli
```

Optional shell override:

```bash
cargo run -p dumbpipex-cli -- --shell /bin/zsh
```

Persist the agent identity so the generated ticket stays stable across restarts:

```bash
cargo run -p dumbpipex-cli -- --persistent-ticket
```

In this mode the CLI automatically saves the generated secret and reuses it on later launches.

Use a custom secret file path when you need to control where it is stored:

```bash
cargo run -p dumbpipex-cli -- --secret-file ~/.config/dumbpipex/agent.secret
```

You can also pass an explicit secret string directly:

```bash
cargo run -p dumbpipex-cli -- --secret <url-safe-secret>
```

Demand mode prints the ticket and leaves the agent running in the background:

```bash
cargo run -p dumbpipex-cli -- --persistent-ticket --demand
```

Stop the managed background agent:

```bash
cargo run -p dumbpipex-cli -- --persistent-ticket --stop-demand
```

The agent prints a **ticket**. Paste that ticket into the Tauri app to connect. The ticket includes the current iroh address information, including the default relay `https://relay.leeapp.dev`.

## Run the Tauri app

Install frontend dependencies once:

```bash
pnpm install
```

Then start the app:

```bash
npm run tauri dev
```

After connecting with a ticket, the app first tries to restore detached PTYs from the remote agent. If none exist, it creates a fresh PTY and starts syncing input/output. On unstable mobile networks, unexpected disconnects automatically retry and reconnect to the same PTYs.

## Current interaction flow

1. Start `dumbpipex-cli` on the machine that should host the shell.
2. Copy the printed ticket into the Tauri app.
3. The app connects to the agent over iroh, opens a control stream, and requests any resumable PTYs.
4. If detached PTYs exist, the app resumes them and replays buffered output; otherwise it requests the first fresh PTY.
5. Additional PTYs can be created from the app without disconnecting the existing ones.
6. Terminal bytes are synced directly between the app and the local PTY owner.

The app UI now exposes explicit session phases (`idle`, `connecting`, `creating_pty`, `ready`, `disconnecting`) so connection problems are easier to reason about.

## Multi-PTY behavior

- Each PTY keeps its own xterm instance, streaming UTF-8 decoder, and scrollback.
- The app shows PTYs as tabs and lets you switch between them without losing terminal state.
- Output that arrives before a PTY tab is mounted is buffered and flushed once that terminal becomes ready.
- Active terminal resizes are forwarded back to the owning PTY so remote line wrapping stays in sync.
- Detached PTYs stay alive on the CLI side for a recovery window, so reconnecting or restarting the app can restore the same shell sessions instead of always spawning new ones.
- Only an explicit **Close PTY** action tears down the remote PTY. Disconnecting the app or losing the network only detaches the mobile client.

## Mobile-first controls

- The terminal remains the main interaction surface.
- The control panel includes shortcut buttons for `Esc`, `Tab`, `Enter`, `Ctrl+C`, and the four arrow keys to help on mobile keyboards.
- A dedicated **Focus terminal** button restores terminal focus after using the control panel.

## Checks

```bash
npm run check
npm run build
cargo check --workspace
cargo test --workspace
```

## Current audit status

The latest source-backed audit is in
[`docs/project-audit-2026-06-05.md`](docs/project-audit-2026-06-05.md).
The follow-up fix pass resolved the protocol/IPC drift, ticket persistence,
viewer-mode enforcement, upload path hardening, `bytes_dropped` telemetry, and
the disabled CSP baseline. Remaining high-value work is broader behavioral
testing plus the larger deferred features in `docs/future-work.md`.

## Current scope

This version is intentionally an MVP:

- one app connection talks directly to one local agent
- multiple PTYs can be created on demand from the app and switched by tabs
- terminal traffic is sent over a single iroh control stream with explicit create/input/resize/close messages
- terminal bytes use URL-safe base64 across the Rust/browser boundary

That keeps the code small while establishing the full end-to-end path needed for future multi-session, sharing, persistence, or richer mobile UX work.
