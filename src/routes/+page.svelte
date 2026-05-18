<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  import ConnectionHome from "$lib/ConnectionHome.svelte";
  import type { RemotePtyApi } from "$lib/RemotePtyPane.svelte";
  import SessionWorkspace from "$lib/SessionWorkspace.svelte";
  import type {
    ConnectTicketResponse,
    PtySession,
    PtyRecoveryInfo,
    RemoteEvent,
    SessionMode,
    SessionPhase,
  } from "$lib/terminal-ui";

  type PersistedPtyState = {
    pty_id: string;
    resume_token: string;
    mode: SessionMode;
  };

  type PersistedRecoveryState = {
    version: 1;
    ticket: string;
    shell: string;
    autoReconnect: boolean;
    ptys: PersistedPtyState[];
  };

  const STORAGE_KEY = "dumbpipex:recovery-state";

  let ticket = $state("");
  let shell = $state("");
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
      const parsed = JSON.parse(raw) as PersistedRecoveryState;
      if (parsed?.version !== 1) return null;
      return parsed;
    } catch {
      return null;
    }
  }

  function writeRecoveryState() {
    if (typeof localStorage === "undefined") return;

    const persisted: PersistedRecoveryState = {
      version: 1,
      ticket,
      shell,
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

    localStorage.setItem(STORAGE_KEY, JSON.stringify(persisted));
  }

  function clearRecoveryState() {
    if (typeof localStorage === "undefined") return;
    localStorage.removeItem(STORAGE_KEY);
  }

  function cancelReconnect() {
    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
  }

  function scheduleReconnect() {
    if (!autoReconnectEnabled || manualDisconnectPending || reconnectTimer || connected) return;
    const delay = Math.min(1000 * 2 ** reconnectAttempt, 8000);
    reconnectTimer = setTimeout(() => {
      reconnectTimer = null;
      reconnectAttempt += 1;
      void connect(true);
    }, delay);
  }

  function bytesToBase64Url(bytes: Uint8Array): string {
    let binary = "";
    for (const byte of bytes) binary += String.fromCharCode(byte);
    return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/g, "");
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
    await invoke("send_pty_input", {
      ptyId: pty.pty_id,
      data: bytesToBase64Url(encoder.encode(data)),
    });
  }

  async function triggerShortcut(data: string) {
    try {
      await sendRemoteInput(data);
      activePtyId && ptyApis.get(activePtyId)?.focus();
    } catch (error) {
      status = String(error);
    }
  }

  function searchActiveTerminal(query: string, direction: "next" | "previous") {
    const api = activeApi();
    if (!api || !query.trim()) return;
    const found =
      direction === "next" ? api.findNext(query.trim()) : api.findPrevious(query.trim());
    status = found ? `已定位到：${query}` : `未找到：${query}`;
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
      const result = await invoke<ConnectTicketResponse>("connect_ticket", { ticket: ticket.trim() });
      connected = true;
      agentName = result.agent_name;
      resetPtyState();
      status = `已连接 ${result.agent_name}`;
      sessionPhase = "ready";
      autoReconnectEnabled = true;
      reconnectAttempt = 0;
      writeRecoveryState();
      const resumed = await resumeExistingSessions(result.sessions);
      if (!resumed) {
        void createRemotePty();
      }
    } catch (error) {
      status = String(error);
      sessionPhase = "idle";
      if (isAutoReconnect || autoReconnectEnabled) {
        autoReconnectEnabled = true;
        writeRecoveryState();
        scheduleReconnect();
      } else {
        autoReconnectEnabled = false;
        writeRecoveryState();
      }
    }
  }

  async function disconnect() {
    if (isBusy()) return;
    cancelReconnect();
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
      writeRecoveryState();
    } catch (error) {
      status = String(error);
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

    const stored = readRecoveryState();
    const storedModes = new Map(
      (stored?.ptys ?? []).map((pty) => [pty.pty_id, pty.mode] satisfies [string, SessionMode]),
    );

    let resumed = 0;
    for (const session of sessions) {
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
        status = String(error);
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
        sessionPhase = "ready";
        await tick();
        ptyApis.get(payload.pty_id)?.writeText(
          `[pty] ${payload.shell} (${payload.cols}x${payload.rows})${payload.resumed ? " restored" : ""}\r\n`,
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
        if (pty) {
          upsertPty({
            ...pty,
            exited: true,
            exit_code: payload.exit_code,
          });
        }

        const api = ptyApis.get(payload.pty_id);
        api?.finish();
        api?.writeText(`\r\n[pty exited] code=${payload.exit_code ?? "unknown"}\r\n`);

        if (activePtyId === payload.pty_id) {
          const fallback =
            ptys.find((item) => item.pty_id !== payload.pty_id && !item.exited)?.pty_id ??
            ptys.find((item) => item.pty_id !== payload.pty_id)?.pty_id ??
            payload.pty_id;
          await selectPty(fallback);
        }

        status = `${payload.pty_id} 已退出`;
        sessionPhase = connected ? "ready" : "idle";
        writeRecoveryState();
        break;
      }
      case "error":
        status = payload.message;
        if (connected) sessionPhase = "ready";
        break;
    }
  }

  onMount(() => {
    const persisted = readRecoveryState();
    if (persisted) {
      ticket = persisted.ticket;
      shell = persisted.shell;
      autoReconnectEnabled = persisted.autoReconnect;
      for (const pty of persisted.ptys) {
        ptyModes.set(pty.pty_id, pty.mode);
      }
      if (persisted.autoReconnect && persisted.ticket.trim()) {
        void connect(true);
      }
    }

    let unlisten: (() => void) | undefined;
    void listen<RemoteEvent>("remote-event", (event) => void handleRemoteEvent(event.payload)).then(
      (dispose) => {
        unlisten = dispose;
      },
    );

    return () => {
      cancelReconnect();
      void invoke("disconnect_ticket").catch(() => undefined);
      unlisten?.();
    };
  });
</script>

<svelte:head>
  <title>dumbpipex</title>
</svelte:head>

<main class="app-shell">
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
      onTicketChange={(value) => {
        ticket = value;
        writeRecoveryState();
      }}
      onShellChange={(value) => {
        shell = value;
        writeRecoveryState();
      }}
      onConnect={() => void connect()}
    />
  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    background: #020617;
    color: #e2e8f0;
    font-family:
      Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  }

  .app-shell {
    min-height: 100svh;
    background:
      radial-gradient(circle at top, rgba(59, 130, 246, 0.18), transparent 30%),
      #020617;
  }
</style>
