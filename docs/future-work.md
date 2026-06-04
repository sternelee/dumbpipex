# Future Work — Deferred from Audit

## Progress

- ✅ **M4** File transfer — implemented (control-stream base64 upload)
- ⏳ H2 Per-PTY substream — ~600 LoC, suggested standalone PR
- ⏳ H3 Binary protocol — ~500 LoC, needs ALPN bump
- ⏳ M5 Port forwarding — ~500 LoC
- ⏳ M14 iOS build — ~3 engineer-days
- ⏳ L* i18n — hardcoded Chinese strings
- ⏳ L* a11y — screen-reader, high-contrast, keyboard nav
- ⏳ L* tests — only `dumbpipex-core` has unit tests

Items from the original v0.1 audit (see the review doc) that were
intentionally left out of the priority-1/2/3 fix batches because
each one is a significant feature in its own right and deserves
its own PR.

| ID | Title | Why deferred | Suggested scope |
|---|---|---|---|
| **H3** | Binary protocol | Replaces length-prefixed JSON with a binary typed-stream format. ~10x faster serialize/deserialize for the PTY-output hot path. Not a correctness win; current JSON is well under CPU on a single PTY. | New `Bins` message variant in `dumbpipex-core`; keep `Json` for back-compat; output frames always use `Bins`. ~500 LoC across core + cli + tauri. |
| **H2** | Per-PTY substream | One QUIC bi-stream today carries control + all PTY I/O. A `cat /dev/urandom` on PTY A starves PTY B's input. Real fix: per-PTY bi-substream with control on stream #0. | New `OpenPty { pty_id }` message that returns a substream id; `PtyInput` / `PtyOutput` carry the id. Touches the reader/writer loops in both `lib.rs` (tauri) and `main.rs` (cli). ~600 LoC. |
| **M4** | File transfer | ✅ **Done** — `Upload { name, mime, size, data }` message over control stream; agent decodes base64 to `uploads/{name}`. Frontend: drag-drop + fixed upload button. ~120 LoC (simpler than iroh-blobs because control-stream base64 is sufficient for files <1 MiB). |
| **M5** | Port forwarding | `ssh -L 8080:remote:80` analog. Useful for "my dev DB is on the box I'm remoting into". | New `PortForward { local_port, remote_addr }` message; agent listens on `local_port` and pipes each accepted TCP socket through a new iroh substream to `remote_addr`. ~500 LoC. |
| **M14** | iOS build | Tauri v2 has iOS support; AGENTS.md mentions Android but the iOS bundle is unconfigured. A real iOS app requires an iOS dev cert, App Store Connect entry, and an iOS-native input bridge. | Configure `tauri.conf.json` `bundle.iOS`, add `ios/` Xcode project, port the existing `mobile-input-bridge` and keyboard-detection logic to iOS WebView quirks. ~3 engineer-days including device testing. |
| **L* i18n** | All UI strings are hardcoded Chinese | Global users cannot use the app. | Extract strings to a `svelte-i18n` store; English + Chinese bundles; switch via header dropdown. |
| **L* a11y** | No screen-reader announcements, no high-contrast theme | Severely limits who can use the app. | Add `aria-live` regions for PTY output; high-contrast theme; full keyboard navigation; pass axe-core scan. |
| **L* tests** | Only `dumbpipex-core` has unit tests | Regressions in cli / tauri / frontend go undetected. | Add `cargo test` cases for `SessionManager` (mock PTY), `ManagedSession::resume` (first-wins, no-kick), `push_backlog` (drop accounting); add Playwright E2E for the connect/disconnect flow. |

## Why these were deferred

Each one is a multi-day PR. The priority-1/2/3 batches already added
~1200 LoC of security / reliability / UX fixes. The cumulative
diff is now ~1700 LoC across the workspace; a follow-up batch on
any of the items above will land cleanly on top.

The protocol additions in priority 2 (H4 `PROTOCOL_VERSION`, the
`#[serde(default)]` fields on `PtySessionInfo` / `PtyCreated`) leave
explicit room for H3 to be done later as a wire-format upgrade
without breaking the deployed v2 client/agent pair.
