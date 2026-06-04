<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  import ConnectionHome from "$lib/ConnectionHome.svelte";
  import type { RemotePtyApi } from "$lib/remote-pty-types";
  import SessionWorkspace from "$lib/SessionWorkspace.svelte";
  import ToastStack, { toast } from "$lib/ToastStack.svelte";
  import type {
    ConnectTicketResponse,
    PtySession,
    PtyRecoveryInfo,
    RemoteEvent,
    SessionMode,
    SessionPhase,
  } from "$lib/terminal-ui";
  import { MAX_INPUT_CHUNK_BYTES } from "$lib/terminal-ui";

  type PersistedPtyState = {
    pty_id: string;
    resume_token: string;
    mode: SessionMode;
  };

  type PersistedRecoveryState = {
    version: 1 | 2;
    ticket: string;
    shell: string;
    autoReconnect: boolean;
    ptys: PersistedPtyState[];
  };

  // The on-disk recovery state for `localStorage` is intentionally
  // **non-sensitive**: it holds the autoReconnect preference and the
  // per-PTY mode map. The ticket itself lives in a Tauri-managed file
  // under `app_data_dir/ticket.json` (mode 0600 on Unix), written via
  // the `save_ticket` / `load_ticket` / `clear_ticket` commands. This
  // means a WebView XSS can flip a switch but cannot exfiltrate the
  // agent credential from localStorage. `ticket` and `shell` are still
  // part of the in-memory shape because they come back from the legacy
  // v1 store; on write we emit them as empty strings.
  type PersistedNonSecretState = {
    version: 1 | 2;
    autoReconnect: boolean;
    ptys: PersistedPtyState[];
  };

  const STORAGE_KEY = "dumbpipex:recovery-state";
  const STORAGE_VERSION = 2;
  const MAX_RECONNECT_ATTEMPTS = 10;
  const KEEPALIVE_INTERVAL_MS = 20_000;
  // A Pong is considered missed if it has not arrived within
  // KEEPALIVE_INTERVAL_MS * 3 of the corresponding Ping. Three
  // consecutive misses trigger an explicit reconnect because the
  // peer is almost certainly unreachable, and the underlying QUIC
  // path will time out anyway.
  const PONG_DEADLINE_MS = KEEPALIVE_INTERVAL_MS * 3;
  const MAX_MISSED_PONGS = 3;

  let ticket = $state("");
  let shell = $state("");
  let viewerMode = $state(false);
  let status = $state("等待连接");
  let connected = $state(false);
  let sessionPhase = $state<SessionPhase>("idle");
  let agentName = $state("");
  let activePtyId = $state<string | null>(null);
  let manualDisconnectPending = $state(false);
  let ptys = $state<PtySession[]>([]);
  let autoReconnectEnabled = $state(false);

  const encoder = new TextEncoder();
  const ptyApis = new Map<string, RemotePtyApi>();
  const pendingOutput = new Map<string, string[]>();
  const ptySizes = new Map<string, { cols: number; rows: number }>();
  const ptyModes = new Map<string, SessionMode>();
  const ptyInputBuffers = new Map<string, string>();
  const ptyResumeTokens = new Map<string, string>();
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  let reconnectAttempt = 0;
  let keepaliveTimer: ReturnType<typeof setInterval> | null = null;
  let pongWatchdog: ReturnType<typeof setTimeout> | null = null;
  let nextPingNonce = 0;
  // Pings sent but not yet acknowledged. Map keyed by nonce so an
  // out-of-order Pong (e.g. across a reconnect+re-attach) still
  // resolves to the right entry. Capped at the last 8 to keep the
  // map from growing on a long-lived dead connection.
  const pendingPings = new Map<number, number>();
  let missedPongs = 0;
  let lastRttMs: number | null = null;

  // Debounce localStorage writes. writeRecoveryState is called from
  // 15+ places (PtyCreated, PtyExited, mode changes, tab switches) and
  // each call is a sync localStorage.setItem. During a busy session
  // this can hit 50+ writes/sec which stutters the main thread. We
  // coalesce them into a single write per `RECOVERY_DEBOUNCE_MS` window.
  // The debounce window is short enough that a crash loses at most a
  // few hundred ms of state.
  const RECOVERY_DEBOUNCE_MS = 200;
  let recoveryDirty = false;
  let recoveryTimer: ReturnType<typeof setTimeout> | null = null;

  function isBusy() {
    return (
      sessionPhase === "connecting" ||
      sessionPhase === "creating_pty" ||
      sessionPhase === "disconnecting"
    );
  }

  function activeApi() {
    return activePtyId ? ptyApis.get(activePtyId) ?? null : null;
  }

  function activeMode() {
    return activePtyId ? (ptyModes.get(activePtyId) ?? "shell") : "shell";
  }

  function readRecoveryState(): PersistedRecoveryState | null {
    if (typeof localStorage === "undefined") return null;
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return null;
      const parsed = JSON.parse(raw) as { version?: number } & Record<string, unknown>;
      // Forward-migrate. v1 (initial schema) had no per-PTY
      // `mode` field; default to "shell" on read. Unknown future
      // versions are rejected to avoid a corrupted store taking down
      // the boot path.
      if (parsed?.version === 1) {
        const v1 = parsed as unknown as { ptys?: Array<Record<string, unknown>> };
        if (Array.isArray(v1.ptys)) {
          for (const pty of v1.ptys) {
            if (typeof pty.mode !== "string") pty.mode = "shell";
          }
        }
        return parsed as unknown as PersistedRecoveryState;
      }
      if (parsed?.version !== STORAGE_VERSION) return null;
      return parsed as PersistedRecoveryState;
    } catch {
      return null;
    }
  }

  function writeRecoveryState() {
    // Mark the recovery state dirty and schedule a coalesced write.
    // Multiple calls within RECOVERY_DEBOUNCE_MS collapse into one.
    // Callers that need durability (e.g. user-initiated disconnect)
    // should follow up with `flushRecoveryState()`.
    recoveryDirty = true;
    if (recoveryTimer !== null) return;
    recoveryTimer = setTimeout(() => {
      recoveryTimer = null;
      flushRecoveryState();
    }, RECOVERY_DEBOUNCE_MS);
  }

  /// Synchronous write of the current recovery state. Skipped if the
  /// state is not dirty, so it's safe to call from any path. Use
  /// this on the disconnect path (we want the cleared state on disk
  /// before the IPC round-trip) and on visibilitychange.
  function flushRecoveryState() {
    if (recoveryTimer !== null) {
      clearTimeout(recoveryTimer);
      recoveryTimer = null;
    }
    if (!recoveryDirty) return;
    recoveryDirty = false;

    if (typeof localStorage === "undefined") return;

    // Strip secrets before writing to localStorage. The ticket lives
    // in the Tauri-managed ticket file (0600); `shell` is harmless
    // but no longer needed in the recovery state — drop it too.
    const persisted: PersistedNonSecretState = {
      version: STORAGE_VERSION,
      autoReconnect: autoReconnectEnabled,
      ptys: ptys
        .filter((pty) => !pty.exited)
        .map((pty) => ({
          pty_id: pty.pty_id,
          resume_token: ptyResumeTokens.get(pty.pty_id) ?? "",
          mode: ptyModes.get(pty.pty_id) ?? "shell",
        }))
        .filter((pty) => pty.resume_token.length > 0),
    };

    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(persisted));
    } catch {
      // Quota / private-mode errors. Recovery is best-effort.
    }
  }

  function cancelReconnect() {
    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
  }

  function startKeepalive() {
    stopKeepalive();
    // Send a Ping every 20s so relay connections and idle NAT mappings
    // stay warm. The agent already replies with Pong; we just need the
    // bytes to traverse the path in both directions to keep middleboxes
    // from dropping the flow. We also arm a watchdog that treats three
    // consecutive missed Pongs as a dead connection.
    keepaliveTimer = setInterval(() => {
      if (!connected) return;
      sendPing();
    }, KEEPALIVE_INTERVAL_MS);
  }

  function stopKeepalive() {
    if (keepaliveTimer) {
      clearInterval(keepaliveTimer);
      keepaliveTimer = null;
    }
    if (pongWatchdog) {
      clearTimeout(pongWatchdog);
      pongWatchdog = null;
    }
    pendingPings.clear();
    missedPongs = 0;
    lastRttMs = null;
  }

  function sendPing() {
    const nonce = ++nextPingNonce;
    pendingPings.set(nonce, Date.now());
    // Cap the in-flight set so a long-lived dead connection cannot
    // grow it without bound; if we have more than 8 unanswered pings
    // something is very wrong and we should already have reconnected.
    if (pendingPings.size > 8) {
      const oldest = pendingPings.keys().next().value;
      if (oldest !== undefined) pendingPings.delete(oldest);
    }
    void invoke("ping_remote", { nonce }).catch((error) => {
      // A failed invoke means the Tauri command itself errored, which
      // usually tracks a closed connection. The reader loop will
      // surface `Disconnected` shortly; just log to status for now.
      status = `keepalive 失败: ${String(error)}`;
    });
    armPongWatchdog();
  }

  function armPongWatchdog() {
    if (pongWatchdog) clearTimeout(pongWatchdog);
    pongWatchdog = setTimeout(() => {
      pongWatchdog = null;
      markPongMissed();
    }, PONG_DEADLINE_MS);
  }

  function recordPong(nonce: number) {
    const sent = pendingPings.get(nonce);
    if (sent === undefined) {
      // Stale Pong from a previous session. Ignore.
      return;
    }
    pendingPings.delete(nonce);
    missedPongs = 0;
    lastRttMs = Date.now() - sent;
    if (pongWatchdog) {
      clearTimeout(pongWatchdog);
      pongWatchdog = null;
    }
    if (pendingPings.size > 0) {
      // Other in-flight pings: arm a new watchdog for the oldest
      // outstanding one.
      armPongWatchdog();
    }
  }

  function markPongMissed() {
    if (!connected) return;
    missedPongs += 1;
    if (missedPongs >= MAX_MISSED_PONGS) {
      const message = `连续 ${MAX_MISSED_PONGS} 次未收到 Pong，连接可能已断开，触发重连`;
      status = message;
      toast(message, "error", 6000);
      // The reader loop's Disconnected event will also fire when
      // iroh tears the connection down, but that can take 30+
      // seconds on a wedged NAT. Force a reconnect now.
      manualDisconnectPending = false;
      scheduleReconnect();
      stopKeepalive();
    }
  }

  function scheduleReconnect() {
    if (!autoReconnectEnabled || manualDisconnectPending || reconnectTimer || connected) return;
    if (reconnectAttempt >= MAX_RECONNECT_ATTEMPTS) {
      // Stop hammering the agent: leave auto-reconnect armed but do
      // not schedule another timer. The user can press "connect" on
      // the home screen to retry, which resets the counter.
      const message = `已停止自动重连（达到 ${MAX_RECONNECT_ATTEMPTS} 次上限），请手动重连`;
      status = message;
      toast(message, "warning", 8000);
      return;
    }
    const delay = Math.min(1000 * 2 ** reconnectAttempt, 8000);
    reconnectTimer = setTimeout(() => {
      reconnectTimer = null;
      reconnectAttempt += 1;
      void connect(true);
    }, delay);
  }

  /// Encode bytes as URL-safe base64 (no padding) off the main
  /// thread. The legacy `btoa(String.fromCharCode(...))` trick
  /// blocks the UI for the duration of the encoding — ~10ms for a
  /// 16 KiB chunk, which is enough to drop a frame on a 60 fps
  /// mobile WebView. `FileReader.readAsDataURL` dispatches the read
  /// to the browser's IO thread, so the result resolves on a later
  /// microtask and the main thread can keep painting.
  function bytesToBase64Url(bytes: Uint8Array): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => {
        const result = reader.result as string;
        // `readAsDataURL` produces
        //   "data:<mime>;base64,<payload>"
        // Strip the prefix and turn the result into URL-safe base64
        // (matching the agent's URL_SAFE_NO_PAD decoder).
        const comma = result.indexOf(",");
        const b64 = comma >= 0 ? result.slice(comma + 1) : result;
        resolve(
          b64.replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/g, ""),
        );
      };
      reader.onerror = () => reject(reader.error ?? new Error("FileReader failed"));
      reader.readAsDataURL(new Blob([bytes]));
    });
  }

  /// Human-friendly byte count for the "X bytes dropped" indicator.
  /// `KiB` style is fine for a tool that already uses IEC units
  /// implicitly (PTY sizes in cols/rows, xterm scrollback in lines).
  function formatBytes(n: number): string {
    if (!Number.isFinite(n) || n < 0) return "0 B";
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KiB`;
    if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(2)} MiB`;
    return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GiB`;
  }

  function getPty(ptyId: string | null) {
    if (!ptyId) return null;
    return ptys.find((pty) => pty.pty_id === ptyId) ?? null;
  }

  function getWritableActivePty() {
    const pty = getPty(activePtyId);
    if (!pty || pty.exited) return null;
    return pty;
  }

  function resetPtyState() {
    pendingOutput.clear();
    ptySizes.clear();
    ptyModes.clear();
    ptyInputBuffers.clear();
    ptyResumeTokens.clear();
    ptys = [];
    activePtyId = null;
  }

  function queuePendingOutput(ptyId: string, chunk: string) {
    const existing = pendingOutput.get(ptyId) ?? [];
    existing.push(chunk);
    pendingOutput.set(ptyId, existing);
  }

  function registerPtyApi(ptyId: string, api: RemotePtyApi | null) {
    if (!api) {
      ptyApis.delete(ptyId);
      return;
    }

    ptyApis.set(ptyId, api);
    const backlog = pendingOutput.get(ptyId);
    if (backlog) {
      for (const chunk of backlog) api.writeBase64Url(chunk);
      pendingOutput.delete(ptyId);
    }
  }

  function setPtyMode(ptyId: string, mode: SessionMode) {
    ptyModes.set(ptyId, mode);
    writeRecoveryState();
  }

  function clearInputBuffer(ptyId: string) {
    ptyInputBuffers.set(ptyId, "");
  }

  function detectCommandMode(command: string): SessionMode | null {
    const normalized = command.trim().toLowerCase();
    if (!normalized) return null;
    if (/^(sudo\s+)?(env\s+\S+\s+)*(vim|nvim)\b/.test(normalized)) return "vim";
    if (/^(sudo\s+)?(env\s+\S+\s+)*claude\b/.test(normalized)) return "claude";
    if (/^(sudo\s+)?(env\s+\S+\s+)*(less|man)\b/.test(normalized)) return "pager";
    if (/^(sudo\s+)?(env\s+\S+\s+)*(python|python3|ipython|node)\b/.test(normalized)) return "repl";
    if (/^(sudo\s+)?(env\s+\S+\s+)*(top|htop|btop)\b/.test(normalized)) return "monitor";
    return null;
  }

  function maybeResetModeFromInput(ptyId: string, mode: SessionMode, data: string) {
    const normalized = data.trim().toLowerCase();

    switch (mode) {
      case "vim":
        if (/\:(q|q!|qa|qa!|wq|x)\r?$/.test(normalized) || normalized === "zz") {
          setPtyMode(ptyId, "shell");
          clearInputBuffer(ptyId);
        }
        break;
      case "claude":
        if (data.includes("\u0003") || ["/exit", "/quit"].includes(normalized)) {
          setPtyMode(ptyId, "shell");
          clearInputBuffer(ptyId);
        }
        break;
      case "pager":
      case "monitor":
        if (normalized === "q" || data.includes("\u0003")) {
          setPtyMode(ptyId, "shell");
          clearInputBuffer(ptyId);
        }
        break;
      case "repl":
        if (data.includes("\u0004") || ["exit()", "quit()"].includes(normalized)) {
          setPtyMode(ptyId, "shell");
          clearInputBuffer(ptyId);
        }
        break;
      default:
        break;
    }
  }

  function trackInputMode(ptyId: string, data: string) {
    const mode = ptyModes.get(ptyId) ?? "shell";

    if (mode !== "shell") {
      maybeResetModeFromInput(ptyId, mode, data);
      return;
    }

    let buffer = ptyInputBuffers.get(ptyId) ?? "";
    for (const char of data) {
      if (char === "\u007f" || char === "\b") {
        buffer = buffer.slice(0, -1);
        continue;
      }

      if (char === "\r" || char === "\n") {
        const detected = detectCommandMode(buffer);
        if (detected) {
          setPtyMode(ptyId, detected);
        }
        buffer = "";
        continue;
      }

      if (char === "\u0003") {
        buffer = "";
        continue;
      }

      if (char >= " " && char !== "\u001b") {
        buffer += char;
      }
    }

    ptyInputBuffers.set(ptyId, buffer);
  }

  async function selectPty(ptyId: string) {
    activePtyId = ptyId;
    await tick();
    ptyApis.get(ptyId)?.fit();
    ptyApis.get(ptyId)?.focus();
  }

  async function sendRemoteInput(data: string) {
    const pty = getWritableActivePty();
    if (!pty) return;
    trackInputMode(pty.pty_id, data);
    // The agent rejects any single `PtyInput` payload larger than
    // `MAX_INPUT_BYTES`. Chunk at `MAX_INPUT_CHUNK_BYTES` boundaries
    // on the UTF-16 string so we never split a surrogate pair in
    // the middle. For a single keystroke this loop runs once and is
    // a no-op.
    const bytes = encoder.encode(data);
    for (let offset = 0; offset < bytes.length; offset += MAX_INPUT_CHUNK_BYTES) {
      const end = Math.min(offset + MAX_INPUT_CHUNK_BYTES, bytes.length);
      const slice = bytes.subarray(offset, end);
      await invoke("send_pty_input", {
        ptyId: pty.pty_id,
        data: await bytesToBase64Url(slice),
      });
    }
  }

  async function triggerShortcut(data: string) {
    try {
      await sendRemoteInput(data);
      activePtyId && ptyApis.get(activePtyId)?.focus();
    } catch (error) {
      status = String(error);
    }
  }

  function searchActiveTerminal(query: string, direction: "next" | "previous"): boolean {
    const api = activeApi();
    if (!api || !query.trim()) return false;
    const found =
      direction === "next" ? api.findNext(query.trim()) : api.findPrevious(query.trim());
    status = found ? `已定位到：${query}` : `未找到：${query}`;
    return found;
  }

  async function copyActiveTerminal() {
    const api = activeApi();
    if (!api) return;
    await api.copySelection();
  }

  function focusActiveTerminal() {
    activeApi()?.focus();
  }

  function handlePaneNotice(message: string) {
    status = message;
    toast(message, "info", 2200);
  }

  async function connect(isAutoReconnect = false) {
    if (!ticket.trim() || isBusy()) return;
    cancelReconnect();
    sessionPhase = "connecting";
    if (!isAutoReconnect) {
      manualDisconnectPending = false;
    }
    status = isAutoReconnect ? "网络已断开，正在自动重连..." : "正在连接远程服务...";
    try {
      const result = await invoke<ConnectTicketResponse>("connect_ticket", {
        ticket: ticket.trim(),
        viewer: viewerMode,
      });
      connected = true;
      agentName = result.agent_name;
      resetPtyState();
      status = `已连接 ${result.agent_name}`;
      sessionPhase = "ready";
      autoReconnectEnabled = true;
      reconnectAttempt = 0;
      // Persist the ticket to the Tauri-managed file (0600 on Unix)
      // so subsequent launches can auto-reconnect. Failures here are
      // non-fatal — the user is already connected, the worst case is
      // no auto-reconnect on next launch.
      if (!isAutoReconnect) {
        void invoke("save_ticket", { ticket: ticket.trim() }).catch((error) => {
          status = `保存 ticket 失败: ${String(error)}`;
        });
      }
      writeRecoveryState();
      startKeepalive();
      const resumed = await resumeExistingSessions(result.sessions);
      if (!resumed) {
        void createRemotePty();
      }
      toast(`已连接 ${result.agent_name}`, "success", 2400);
    } catch (error) {
      const message = String(error);
      status = message;
      toast(message, "error", 6000);
      sessionPhase = "idle";
      // On failure we intentionally do NOT call `resetPtyState()`:
      // a transient network error during reconnect should leave the
      // in-memory PTY list (and the localStorage snapshot) untouched
      // so a subsequent successful reconnect re-attaches the same
      // sessions. `writeRecoveryState` is a no-op when nothing
      // changed (H6 debounce), so it is safe to call here. The two
      // branches differ only in `autoReconnectEnabled`, which is
      // already true on auto-reconnect attempts and false on the
      // first manual attempt — we keep that value either way.
      if (isAutoReconnect || autoReconnectEnabled) {
        autoReconnectEnabled = true;
        scheduleReconnect();
      }
      writeRecoveryState();
    }
  }

  async function disconnect() {
    if (isBusy()) return;
    cancelReconnect();
    stopKeepalive();
    reconnectAttempt = 0;
    sessionPhase = "disconnecting";
    manualDisconnectPending = true;
    autoReconnectEnabled = false;
    writeRecoveryState();
    status = "正在断开连接...";
    try {
      await invoke("disconnect_ticket");
      connected = false;
      agentName = "";
      status = "已断开";
      sessionPhase = "idle";
      resetPtyState();
      // Forget the persisted credential on a clean user disconnect.
      // We keep it across transient network failures (handled by
      // scheduleReconnect without invoking this function) so the user
      // does not have to re-paste the ticket after a WiFi blip.
      void invoke("clear_ticket").catch(() => undefined);
      // Force-flush the recovery state: the next launch should not
      // see `autoReconnect=true` lingering from before the disconnect.
      flushRecoveryState();
    } catch (error) {
      const message = String(error);
      status = message;
      toast(message, "error", 5000);
      sessionPhase = connected ? "ready" : "idle";
      manualDisconnectPending = false;
      autoReconnectEnabled = true;
      writeRecoveryState();
    }
  }

  async function createRemotePty() {
    if (!connected || isBusy()) return;
    sessionPhase = "creating_pty";
    status = `正在创建第 ${ptys.length + 1} 个远程终端...`;
    try {
      const preferredSize =
        (activePtyId ? ptySizes.get(activePtyId) : undefined) ??
        [...ptySizes.values()].at(-1) ?? { cols: 80, rows: 24 };

      await invoke("create_pty", {
        shell: shell.trim() || null,
        cols: Math.max(preferredSize.cols, 1),
        rows: Math.max(preferredSize.rows, 1),
      });
    } catch (error) {
      status = String(error);
      sessionPhase = connected ? "ready" : "idle";
    }
  }

  async function uploadFile(file: File) {
    if (!connected || isBusy()) return;
    const reader = new FileReader();
    reader.readAsDataURL(file);
    await new Promise<void>((resolve) => {
      reader.onloadend = () => resolve();
    });
    const base64 = (reader.result as string).split(",")[1];
    try {
      await invoke("upload_file", {
        name: file.name,
        mime: file.type || "application/octet-stream",
        data: base64,
      });
    } catch (error) {
      status = `上传失败: ${error}`;
      toast(status, "error", 5000);
    }
  }

  async function handlePtyResize(ptyId: string, size: { cols: number; rows: number }) {
    ptySizes.set(ptyId, size);

    const pty = getPty(ptyId);
    if (!connected || !pty || pty.exited || isBusy()) return;

    try {
      await invoke("resize_pty", {
        ptyId,
        cols: Math.max(size.cols, 1),
        rows: Math.max(size.rows, 1),
      });
    } catch (error) {
      status = String(error);
    }
  }

  async function closeRemotePty(ptyId = activePtyId) {
    if (!ptyId || isBusy()) return;
    const pty = getPty(ptyId);
    if (pty?.exited) {
      // Already exited — remove immediately. No PtyExited event
      // will arrive because state.attached was already taken by
      // mark_exited during natural exit.
      ptys = ptys.filter((item) => item.pty_id !== ptyId);
      ptyApis.get(ptyId)?.finish();
      if (activePtyId === ptyId) {
        const fallback = ptys.find((item) => !item.exited)?.pty_id ?? ptys[0]?.pty_id ?? null;
        if (fallback) await selectPty(fallback);
      }
      status = `${ptyId} 已关闭`;
      return;
    }
    status = `正在关闭 ${ptyId}...`;
    try {
      await invoke("close_pty", { ptyId });
    } catch (error) {
      status = String(error);
    }
  }

  function upsertPty(pty: PtySession) {
    const index = ptys.findIndex((item) => item.pty_id === pty.pty_id);
    if (index >= 0) {
      ptys[index] = pty;
      ptys = [...ptys];
    } else {
      ptys = [...ptys, pty];
    }
  }

  async function resumeExistingSessions(sessions: PtyRecoveryInfo[]) {
    if (sessions.length === 0) return false;

    // Filter out PTYs the agent reports as already-attached. The
    // agent omits the resume token for these (M1), so the resume
    // call below would be guaranteed to fail — skip them entirely
    // and surface a status message so the user understands why some
    // sessions are missing.
    const resumable = sessions.filter((session) => {
      if (session.attached) {
        status = `${session.pty_id} 已被其他客户端接管，跳过恢复`;
        return false;
      }
      if (!session.resume_token) {
        status = `${session.pty_id} 缺少 resume token，跳过恢复`;
        return false;
      }
      return true;
    });
    if (resumable.length === 0) return false;

    const stored = readRecoveryState();
    const storedModes = new Map(
      (stored?.ptys ?? []).map((pty) => [pty.pty_id, pty.mode] satisfies [string, SessionMode]),
    );

    let resumed = 0;
    for (const session of resumable) {
      ptyResumeTokens.set(session.pty_id, session.resume_token);
      const restoredMode = storedModes.get(session.pty_id);
      if (restoredMode) {
        ptyModes.set(session.pty_id, restoredMode);
      }
      try {
        await invoke("resume_pty", {
          ptyId: session.pty_id,
          resumeToken: session.resume_token,
          cols: Math.max(session.cols, 1),
          rows: Math.max(session.rows, 1),
        });
        resumed += 1;
      } catch (error) {
        // The PTY can no longer be resumed (token mismatch after
        // agent restart, gone from sweeper, or rejected because
        // another client took the slot between ListPtys and our
        // resume). Drop every trace of it from the local recovery
        // state and in-memory caches so we do not try to resume it
        // again on the next reconnect.
        ptyResumeTokens.delete(session.pty_id);
        ptyModes.delete(session.pty_id);
        ptyInputBuffers.delete(session.pty_id);
        status = `${session.pty_id} 恢复失败: ${String(error)}`;
      }
    }

    writeRecoveryState();
    return resumed > 0;
  }

  async function handleRemoteEvent(payload: RemoteEvent) {
    switch (payload.type) {
      case "disconnected":
        connected = false;
        agentName = "";
        status = manualDisconnectPending
          ? "已断开"
          : payload.reason
            ? `连接断开: ${payload.reason}`
            : "连接已断开";
        sessionPhase = manualDisconnectPending ? "idle" : "connecting";
        stopKeepalive();
        resetPtyState();
        writeRecoveryState();
        if (manualDisconnectPending) {
          manualDisconnectPending = false;
          reconnectAttempt = 0;
        } else {
          scheduleReconnect();
        }
        break;
      case "pty_created":
        ptyResumeTokens.set(payload.pty_id, payload.resume_token);
        upsertPty({
          pty_id: payload.pty_id,
          shell: payload.shell,
          cols: payload.cols,
          rows: payload.rows,
          exited: false,
          bytes_dropped: payload.bytes_dropped ?? 0,
        });
        ptySizes.set(payload.pty_id, { cols: payload.cols, rows: payload.rows });
        if (!ptyModes.has(payload.pty_id)) {
          setPtyMode(payload.pty_id, "shell");
        }
        clearInputBuffer(payload.pty_id);
        activePtyId = payload.pty_id;
        status = payload.resumed
          ? `已恢复 ${payload.pty_id} (${payload.shell})`
          : `已创建 ${payload.pty_id} (${payload.shell})`;
        toast(status, "success", 2400);
        // Surface backlog truncation immediately so the user knows
        // the resumed session is not a complete replay.
        const dropped = payload.bytes_dropped ?? 0;
        if (dropped > 0) {
          const message = `代理已丢失 ${formatBytes(dropped)} 输出（回放可能不完整）`;
          toast(message, "warning", 6000);
          status = message;
        }
        sessionPhase = "ready";
        await tick();
        ptyApis.get(payload.pty_id)?.writeText(
          `[pty] ${payload.shell} (${payload.cols}x${payload.rows})${payload.resumed ? " restored" : ""}${dropped > 0 ? ` [已丢失 ${formatBytes(dropped)}]` : ""}\r\n`,
        );
        ptyApis.get(payload.pty_id)?.fit();
        ptyApis.get(payload.pty_id)?.focus();
        writeRecoveryState();
        break;
      case "pty_output": {
        const api = ptyApis.get(payload.pty_id);
        if (api) api.writeBase64Url(payload.data);
        else queuePendingOutput(payload.pty_id, payload.data);
        break;
      }
      case "pty_exited": {
        ptyModes.delete(payload.pty_id);
        ptyInputBuffers.delete(payload.pty_id);
        ptyResumeTokens.delete(payload.pty_id);
        const pty = getPty(payload.pty_id);
        if (payload.exit_code === null || payload.exit_code === undefined) {
          // close_pty — remove tab entirely
          if (pty) {
            ptys = ptys.filter((item) => item.pty_id !== payload.pty_id);
          }
        } else if (pty) {
          upsertPty({
            ...pty,
            exited: true,
            exit_code: payload.exit_code,
          });
        }

        const api = ptyApis.get(payload.pty_id);
        api?.finish();
        if (payload.exit_code !== null && payload.exit_code !== undefined) {
          api?.writeText(`\r\n[pty exited] code=${payload.exit_code}\r\n`);
        }

        if (activePtyId === payload.pty_id) {
          const fallback =
            ptys.find((item) => item.pty_id !== payload.pty_id && !item.exited)?.pty_id ??
            ptys.find((item) => item.pty_id !== payload.pty_id)?.pty_id ??
            payload.pty_id;
          await selectPty(fallback);
        }

        status = `${payload.pty_id} 已退出`;
        toast(status, "info", 2400);
        sessionPhase = connected ? "ready" : "idle";
        writeRecoveryState();
        break;
      }
      case "pty_detached": {
        // Another client took over this PTY. We do NOT get a PtyExited, so
        // mark the local tab as detached and surface a clear status so the
        // user knows their input is being ignored. Reusing the `exited`
        // flag is the simplest way to disable input on this tab without
        // introducing a new session field.
        const api = ptyApis.get(payload.pty_id);
        api?.writeText(
          `\r\n[pty detached] ${payload.reason}\r\n[pty detached] input disabled for ${payload.pty_id}\r\n`,
        );
        const existing = getPty(payload.pty_id);
        if (existing && !existing.exited) {
          upsertPty({ ...existing, exited: true });
        }
        ptyResumeTokens.delete(payload.pty_id);
        ptyModes.delete(payload.pty_id);
        ptyInputBuffers.delete(payload.pty_id);
        if (activePtyId === payload.pty_id) {
          const fallback =
            ptys.find((item) => item.pty_id !== payload.pty_id && !item.exited)?.pty_id ??
            ptys.find((item) => item.pty_id !== payload.pty_id)?.pty_id ??
            payload.pty_id;
          await selectPty(fallback);
        }
        status = `${payload.pty_id} 被其他客户端接管`;
        toast(status, "warning", 5000);
        sessionPhase = connected ? "ready" : "idle";
        writeRecoveryState();
        break;
      }
      case "error":
        status = payload.message;
        toast(payload.message, "error", 6000);
        if (connected) sessionPhase = "ready";
        break;
      case "pong":
        recordPong(payload.nonce);
        break;
      case "upload_accepted":
        status = `文件 ${payload.name} 已上传到 ${payload.path}`;
        toast(status, "success", 3000);
        break;
      case "upload_error":
        status = `上传 ${payload.name} 失败: ${payload.message}`;
        toast(status, "error", 5000);
        break;
    }
  }

  /* ── Android WebView 键盘适配 ──
     We don't fight the WebView with scrollTo(0,0) intervals: the
     root <html> and <body> are already overflow:hidden +
     height:var(--app-vh), so there's no scroll position to force.
     We just keep --app-vh in sync with the visual viewport so
     the layout reflows to fit the new (smaller) viewport as the
     keyboard animates in. */
  function syncViewportHeight() {
    if (typeof window === "undefined") return;
    const h = window.visualViewport?.height ?? window.innerHeight;
    document.documentElement.style.setProperty("--app-vh", `${h}px`);
  }

  onMount(() => {
    const persisted = readRecoveryState();
    if (persisted) {
      // v1 (and early v2) stored the ticket in localStorage. We still
      // honor it on read for back-compat, but on next save the
      // recovery state will no longer include it. `shell` carries
      // over the same way.
      ticket = persisted.ticket;
      shell = persisted.shell;
      autoReconnectEnabled = persisted.autoReconnect;
      for (const pty of persisted.ptys) {
        ptyModes.set(pty.pty_id, pty.mode);
      }
    }

    let unlisten: (() => void) | undefined;
    // Order matters: subscribe to remote-event BEFORE issuing the
    // auto-reconnect. Otherwise early "Hello" / "PtyList" / "PtyCreated"
    // frames arrive between connect() and listen() resolving and are
    // silently dropped, which manifests as "connected but PTYs missing
    // after restart". The listen() promise resolves synchronously in
    // tests but is a real Tauri IPC round-trip on device, hence the
    // observable race.
    //
    // We also load the ticket from the Tauri-managed file. If the
    // user has never connected (or has disconnected) the file is
    // absent and we silently fall through to manual paste.
    void (async () => {
      const [dispose, storedTicket] = await Promise.all([
        listen<RemoteEvent>("remote-event", (event) =>
          void handleRemoteEvent(event.payload)),
        invoke<string | null>("load_ticket").catch(() => null),
      ]);
      unlisten = dispose;
      if (storedTicket && !ticket.trim()) {
        ticket = storedTicket;
      }
      if (autoReconnectEnabled && ticket.trim()) {
        void connect(true);
      }
    })();

    syncViewportHeight();

    const vv = window.visualViewport;
    const onResize = () => syncViewportHeight();
    vv?.addEventListener("resize", onResize);
    window.addEventListener("resize", onResize);

    // Re-sync at intervals after focus transitions, since some
    // WebViews report stale visualViewport.height mid-keyboard
    // animation. Cheap (single DOM write per call) and bounded.
    const resyncAfterFocus = () => {
      syncViewportHeight();
      setTimeout(syncViewportHeight, 100);
      setTimeout(syncViewportHeight, 350);
    };
    document.addEventListener("focusin", resyncAfterFocus);
    document.addEventListener("focusout", resyncAfterFocus);

    const t1 = setTimeout(syncViewportHeight, 300);
    const t2 = setTimeout(syncViewportHeight, 800);

    return () => {
      cancelReconnect();
      stopKeepalive();
      // Flush any pending recovery-state write synchronously so a
      // page navigation / app close does not lose the most recent
      // state.
      flushRecoveryState();
      // In Vite dev mode, `onMount` cleanup fires on every HMR
      // reload of the page module. Issuing `disconnect_ticket`
      // there would kill the live iroh session mid-edit and force
      // the user to re-paste the ticket after every code change.
      // Skip the teardown in DEV; production builds (no HMR) still
      // go through the normal disconnect path on app close.
      if (!import.meta.env.DEV) {
        void invoke("disconnect_ticket").catch(() => undefined);
      }
      unlisten?.();
      vv?.removeEventListener("resize", onResize);
      window.removeEventListener("resize", onResize);
      document.removeEventListener("focusin", resyncAfterFocus);
      document.removeEventListener("focusout", resyncAfterFocus);
      clearTimeout(t1);
      clearTimeout(t2);
    };
  });

  // Flush pending writes when the tab/page is hidden, so a mobile OS
  // backgrounding the WebView does not lose the last few hundred ms
  // of recovery state.
  if (typeof document !== "undefined") {
    document.addEventListener("visibilitychange", () => {
      if (document.visibilityState === "hidden") flushRecoveryState();
    });
  }
</script>

<svelte:head>
  <title>dumbpipex</title>
</svelte:head>

<input
  type="file"
  id="file-upload"
  style="display:none"
  onchange={(e) => {
    const file = e.currentTarget.files?.[0];
    if (file) uploadFile(file);
    e.currentTarget.value = "";
  }}
/>
<main
  class="app-shell"
  ondragover={(e) => { e.preventDefault(); }}
  ondrop={(e) => {
    e.preventDefault();
    const file = e.dataTransfer?.files[0];
    if (file && connected) uploadFile(file);
  }}
>
  <button
    class="upload-btn"
    onclick={() => document.getElementById("file-upload")?.click()}
    title="上传文件"
  >
    📎
  </button>
  <ToastStack />
  {#if connected}
    <SessionWorkspace
      agentName={agentName}
      status={status}
      sessionPhase={sessionPhase}
      activeMode={activeMode()}
      ptys={ptys}
      activePtyId={activePtyId}
      busy={isBusy()}
      onDisconnect={() => void disconnect()}
      onCreatePty={() => void createRemotePty()}
      onCloseActivePty={() => void closeRemotePty()}
      onSelectPty={(ptyId) => void selectPty(ptyId)}
      onFocusActivePty={focusActiveTerminal}
      onCopyActiveTerminal={() => void copyActiveTerminal()}
      onSearch={searchActiveTerminal}
      onSendShortcut={(data) => void triggerShortcut(data)}
      onPaneData={(data) => void sendRemoteInput(data).catch((error) => (status = String(error)))}
      onPaneNotice={handlePaneNotice}
      onRegisterPtyApi={registerPtyApi}
      onResizePty={(ptyId, size) => void handlePtyResize(ptyId, size)}
    />
  {:else}
    <ConnectionHome
      ticket={ticket}
      shell={shell}
      status={status}
      sessionPhase={sessionPhase}
      busy={isBusy()}
      viewerMode={viewerMode}
      onTicketChange={(value) => {
        ticket = value;
        writeRecoveryState();
      }}
      onShellChange={(value) => {
        shell = value;
        writeRecoveryState();
      }}
      onViewerModeChange={(value) => {
        viewerMode = value;
      }}
      onConnect={() => void connect()}
    />
  {/if}
</main>

<style>
  :global(html) {
    height: var(--app-vh, 100vh);
    background: #020617;
    scrollbar-width: none;
    -ms-overflow-style: none;
    overflow: hidden;
  }

  :global(html::-webkit-scrollbar) {
    display: none;
  }

  :global(body) {
    height: var(--app-vh, 100vh);
    margin: 0;
    background: #020617;
    color: #e2e8f0;
    font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    overscroll-behavior: none;
    -webkit-tap-highlight-color: transparent;
    scrollbar-width: none;
    -ms-overflow-style: none;
    overflow: hidden;
  }

  :global(body::-webkit-scrollbar) {
    display: none;
  }

  :global(*::-webkit-scrollbar) {
    display: none;
  }

  :global(button),
  :global(input),
  :global(textarea),
  :global(select) {
    font: inherit;
  }

  .app-shell {
    height: var(--app-vh, 100vh);
    max-height: var(--app-vh, 100vh);
    overflow: hidden;
    background:
      radial-gradient(circle at top, rgba(59, 130, 246, 0.18), transparent 30%),
      #020617;
  }

  /* 禁止 iOS 点击输入时自动缩放 */
  :global(textarea),
  :global(input) {
    font-size: 16px !important;
    touch-action: manipulation;
  }

  @media (max-width: 899px) {
    :global(input),
    :global(textarea),
    :global(select),
    :global(button) {
      font-size: 16px;
    }
  }

  .upload-btn {
    position: fixed;
    bottom: 1rem;
    right: 1rem;
    z-index: 50;
    width: 2.5rem;
    height: 2.5rem;
    border-radius: 9999px;
    background: #334155;
    color: #e2e8f0;
    border: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1.25rem;
    opacity: 0.8;
    transition: opacity 0.15s;
  }
  .upload-btn:hover {
    opacity: 1;
  }
</style>
