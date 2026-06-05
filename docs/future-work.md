# Future Work - Deferred from Audit

Last updated: 2026-06-05. See `docs/project-audit-2026-06-05.md` for the latest
source-backed review.

## Progress

- Done P0 Protocol/IPC drift. Keepalive `nonce`/`Pong` is wired through
  Tauri/core/CLI, and ticket persistence commands exist.
- Done P0 Viewer mode. Tauri stores read-only session state and rejects write
  commands; the frontend disables or blocks write actions.
- Done P1 M4 file transfer path hardening. Upload filenames are sanitized and
  forced under `uploads/`. Payload size remains intentionally unrestricted.
- Done P1 Protocol limit telemetry. `bytes_dropped` is wired through the
  protocol, and frontend input chunking is documented as responsiveness
  chunking rather than a size limit.
- Done P1 CSP baseline. Tauri no longer runs with `csp: null`.
- Open H2 Per-PTY substream - ~600 LoC, suggested standalone PR.
- Open H3 Binary protocol - ~500 LoC, needs ALPN/protocol negotiation.
- Open M5 Port forwarding - ~500 LoC.
- Open M14 iOS/mobile build hardening - generated platform files exist, but
  signing, device QA, and mobile-native terminal behavior remain unresolved.
- Open L* i18n - hardcoded Chinese strings.
- Open L* a11y - screen-reader, high-contrast, keyboard nav.
- Open L* tests - protocol and CLI helper unit tests now exist, but PTY/session
  behavior and frontend E2E coverage are still thin.

Items from the original v0.1 audit (see the review doc) that were
intentionally left out of the priority-1/2/3 fix batches because
each one is a significant feature in its own right and deserves
its own PR.

| ID | Title | Why deferred | Suggested scope |
|---|---|---|---|
| **H3** | Binary protocol | Replaces length-prefixed JSON with a binary typed-stream format. ~10x faster serialize/deserialize for the PTY-output hot path. Not a correctness win; current JSON is well under CPU on a single PTY. | New `Bins` message variant in `dumbpipex-core`; keep `Json` for back-compat; output frames always use `Bins`. ~500 LoC across core + cli + tauri. |
| **H2** | Per-PTY substream | One QUIC bi-stream today carries control + all PTY I/O. A `cat /dev/urandom` on PTY A starves PTY B's input. Real fix: per-PTY bi-substream with control on stream #0. | New `OpenPty { pty_id }` message that returns a substream id; `PtyInput` / `PtyOutput` carry the id. Touches the reader/writer loops in both `lib.rs` (tauri) and `main.rs` (cli). ~600 LoC. |
| **P0** | Protocol/IPC drift | Fixed. Frontend keepalive now sends `Ping { nonce }`; CLI echoes `Pong { nonce }`; Tauri emits the pong event and implements ticket-file commands. | Keep regression tests around reconnect when E2E coverage is added. |
| **P0** | Viewer mode enforcement | Fixed locally. UI exposes viewer mode, Tauri stores read-only session state, and write commands are rejected. | If viewer mode must become a trust boundary between independent clients, add protocol-level viewer capability negotiation on top of the local enforcement. |
| **M4** | File transfer | MVP plus path hardening is implemented — `Upload { name, mime, size, data }` over the control stream with frontend drag/drop. Filenames are sanitized and forced under `uploads/`. Payload size is intentionally unrestricted for now. | Keep iroh-blobs and upload size caps deferred unless larger-file reliability or abuse control becomes a real requirement. |
| **M5** | Port forwarding | `ssh -L 8080:remote:80` analog. Useful for "my dev DB is on the box I'm remoting into". | New `PortForward { local_port, remote_addr }` message; agent listens on `local_port` and pipes each accepted TCP socket through a new iroh substream to `remote_addr`. ~500 LoC. |
| **M14** | iOS/mobile build hardening | Generated `src-tauri/gen/apple` and `src-tauri/gen/android` projects now exist, and `docs/mobile-gen-regeneration.md` records manual platform tweaks. The remaining work is not "create platform files"; it is signing, device QA, and making the terminal interaction model native enough on mobile. | Verify iOS signing/profile setup, device-test the existing mobile input bridge, move fragile generated-project tweaks into a repeatable generation workflow, and document release steps. |
| **L* i18n** | All UI strings are hardcoded Chinese | Global users cannot use the app. | Extract strings to a `svelte-i18n` store; English + Chinese bundles; switch via header dropdown. |
| **L* a11y** | No screen-reader announcements, no high-contrast theme | Severely limits who can use the app. | Add `aria-live` regions for PTY output; high-contrast theme; full keyboard navigation; pass axe-core scan. |
| **L* tests** | Behavioral coverage is still thin | Core protocol compatibility and CLI helper tests now exist, but the session manager, Tauri command bridge, and frontend flows still lack meaningful E2E coverage. | Add `cargo test` cases for `SessionManager` (mock PTY), `ManagedSession::resume`, upload rejection; add Playwright E2E for connect/disconnect/reconnect flow. |

## Why these were deferred

Each one is a multi-day PR. The priority-1/2/3 batches already added
~1200 LoC of security / reliability / UX fixes. The cumulative
diff is now ~1700 LoC across the workspace; a follow-up batch on
any of the items above will land cleanly on top.

The current protocol is still `dumbpipex-terminal-v1` length-prefixed JSON with
no explicit negotiated `PROTOCOL_VERSION`. Any H3 binary-protocol work should
first add version negotiation / compatibility tests, then move hot-path PTY
output off JSON without breaking the deployed v1 client/agent pair.
