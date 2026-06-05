# Project Audit - 2026-06-05

## Scope

Source-backed review of the current repo state after loading the workspace
conventions and `.omx` session state. This pass focused on protocol/API
consistency, shipped documentation accuracy, security-sensitive boundaries, and
verification coverage.

## Fix Status

Updated after the follow-up fix pass:

- Fixed: keepalive now uses `Ping { nonce }` / `Pong { nonce }` through
  `dumbpipex-core`, `dumbpipex-cli`, Tauri, and the frontend event handler.
- Fixed: `save_ticket`, `load_ticket`, and `clear_ticket` now persist the ticket
  in a Tauri-managed `ticket.json` file with mode 0600 on Unix.
- Fixed: viewer mode is enforced locally by Tauri session state and by frontend
  UI guards for input, create, close, resize, and upload actions.
- Fixed with product constraint: upload filenames are sanitized and forced under
  `uploads/`; payload size remains intentionally unrestricted.
- Fixed: `bytes_dropped` is now tracked on backlog trimming and returned through
  PTY list/create messages; frontend input chunking is documented as chunking,
  not a max-size protocol limit.
- Fixed: Tauri CSP is no longer disabled.
- Improved: unit tests were added for protocol compatibility, upload filename
  sanitization, and backlog drop accounting.

## Verification Run

Commands run from the workspace root:

```bash
npm run check
npm run build
cargo check --workspace
cargo test --workspace
```

Result: all commands completed successfully. The original audit found no Rust
unit tests; the follow-up fix pass added focused protocol and CLI helper tests.
Coverage is still not broad enough to prove end-to-end session behavior.

## High-Risk Findings

### P0 - Frontend keepalive does not match the Rust protocol

Evidence:

- `src/routes/+page.svelte` sends `invoke("ping_remote", { nonce })`, tracks
  pending pings by nonce, and expects a `remote-event` payload of
  `{ type: "pong", nonce }`.
- `src-tauri/src/lib.rs` defines `ping_remote(state)` with no nonce argument.
- `crates/dumbpipex-core/src/lib.rs` defines `ClientMessage::Ping` and
  `ServerMessage::Pong` without nonce fields.
- `src-tauri/src/lib.rs` currently drops `ServerMessage::Pong` instead of
  emitting it to the frontend.

Impact: connected clients can mark healthy sessions as dead after missed
frontend-only pong tracking, then force unnecessary reconnects.

Recommended fix: either remove nonce tracking and treat a successful
`ping_remote` invoke as the keepalive acknowledgement, or add `nonce` to
`Ping/Pong` in `dumbpipex-core`, echo it in the CLI, and emit `RemoteEvent::Pong`
from Tauri.

### P0 - Ticket persistence is documented in the frontend but missing in Tauri

Evidence:

- `src/routes/+page.svelte` calls `save_ticket`, `load_ticket`, and
  `clear_ticket`.
- `src-tauri/src/lib.rs` registers only `connect_ticket`, `disconnect_ticket`,
  PTY commands, `ping_remote`, and `upload_file`.
- No `save_ticket`, `load_ticket`, or `clear_ticket` implementation exists under
  `src-tauri/src`.

Impact: v2 recovery state intentionally removes the ticket from `localStorage`,
but the replacement Tauri-managed ticket file does not exist. Auto-reconnect
after app restart cannot work reliably, and the UI can surface non-fatal
"save ticket failed" status after a successful connection.

Recommended fix: implement the three Tauri commands using `app_data_dir` with a
0600 Unix mode where available, or revert the recovery schema to an explicitly
accepted localStorage-based ticket model.

### P0 - Viewer mode is UI-only

Evidence:

- `src/lib/ConnectionHome.svelte` exposes "read-only connection (viewer)".
- `src/routes/+page.svelte` passes `viewer: viewerMode` to `connect_ticket`.
- `src-tauri/src/lib.rs::connect_ticket` does not accept a viewer flag.
- `dumbpipex-core` has no viewer/read-only capability in `Hello` or session
  messages, and the frontend does not disable input/resize/close paths when
  viewer mode is checked.

Impact: the UI promises read-only behavior that is not enforced by either the
frontend command paths or the agent protocol.

Recommended fix: either remove/disable the viewer toggle until the protocol
supports it, or add a viewer capability to the handshake and enforce it on
`PtyInput`, `ResizePty`, `ClosePty`, `CreatePty`, and `Upload`.

## Medium-Risk Findings

### P1 - File upload needs path and size hardening

Evidence:

- `crates/dumbpipex-cli/src/main.rs` writes uploaded bytes to
  `format!("uploads/{name}")`.
- The upload handler decodes the full base64 payload and writes it in one shot.
- `ClientMessage::Upload` carries `size`, but the handler ignores it.
- `docs/future-work.md` describes file transfer as sufficient for files below
  1 MiB, but no protocol or frontend limit enforces that.

Impact: a connected client can write outside `uploads/` with path traversal
names and can force large in-memory allocations through unbounded JSON/base64
frames.

Recommended fix: sanitize to a basename or generated server-side filename,
canonicalize and assert the destination remains under an upload root, enforce a
documented max decoded size, and reject frames before decoding when possible.

### P1 - Protocol limit and dropped-output telemetry drifted into frontend-only code

Evidence:

- `src/lib/terminal-ui.ts` says `MAX_INPUT_BYTES` and
  `MAX_INPUT_CHUNK_BYTES` must stay in sync with constants in
  `crates/dumbpipex-core/src/lib.rs`; those constants are not present there.
- `src/routes/+page.svelte` comments say the agent rejects oversized
  `PtyInput`, but the CLI currently decodes and writes whatever arrives.
- Frontend types include `bytes_dropped`, but Rust `PtySessionInfo` and
  `PtyCreated` do not carry it; `push_backlog` trims backlog without reporting
  cumulative dropped bytes.

Impact: the UI and comments imply guardrails that are not enforced. Future
changes may rely on these guarantees and miss real runtime failure modes.

Recommended fix: move protocol limits into `dumbpipex-core`, enforce them in
the CLI/Tauri boundary, and either wire `bytes_dropped` through the protocol or
remove the UI surface until it exists.

### P1 - CSP is disabled

Evidence:

- `src-tauri/tauri.conf.json` sets `"csp": null`.

Impact: this is risky for a WebView app that handles a bearer ticket and renders
terminal output. The current xterm path writes decoded terminal bytes rather
than HTML, but the app also opens links and uses browser clipboard APIs.

Recommended fix: define a minimal Tauri CSP before treating ticket persistence
as production-safe.

## Documentation Corrections

- README now points dependency installation at `pnpm install`, matching
  `pnpm-workspace.yaml` and the lockfile.
- `docs/future-work.md` now distinguishes implemented MVP file transfer from
  missing production hardening.
- The old "only dumbpipex-core has unit tests" note is replaced with the current
  evidence: the workspace currently reports zero Rust unit tests.
- `docs/macos-native-feel-audit.md` is marked with a 2026-06-05 refresh because
  generated iOS/Android platform directories now exist and a web `MenuBar`
  component exists, even though native shell integration is still missing.

## Current Priority Queue

1. Fix protocol/IPC drift: keepalive nonce/Pong and ticket persistence commands.
2. Remove or enforce viewer mode.
3. Harden upload path handling and file/frame size limits.
4. Move frontend-only protocol limits/telemetry into `dumbpipex-core`.
5. Add regression tests for protocol round-trips, recovery state, upload
   rejection, and reconnect keepalive behavior.
