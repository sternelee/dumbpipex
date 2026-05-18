<script lang="ts">
  import { onMount } from "svelte";
  import RemotePtyPane, { type RemotePtyApi } from "$lib/RemotePtyPane.svelte";
  import {
    mobileModeShortcuts,
    shortcutSections,
    sessionModeLabel,
    terminalThemes,
    type PtySession,
    type SessionMode,
    type SessionPhase,
  } from "$lib/terminal-ui";

  let {
    agentName,
    status,
    sessionPhase,
    activeMode,
    ptys,
    activePtyId,
    busy,
    onDisconnect,
    onCreatePty,
    onCloseActivePty,
    onSelectPty,
    onFocusActivePty,
    onCopyActiveTerminal,
    onSearch,
    onSendShortcut,
    onPaneData,
    onPaneNotice,
    onRegisterPtyApi,
    onResizePty,
  }: {
    agentName: string;
    status: string;
    sessionPhase: SessionPhase;
    activeMode: SessionMode;
    ptys: PtySession[];
    activePtyId: string | null;
    busy: boolean;
    onDisconnect: () => void;
    onCreatePty: () => void;
    onCloseActivePty: () => void;
    onSelectPty: (ptyId: string) => void;
    onFocusActivePty: () => void;
    onCopyActiveTerminal: () => void;
    onSearch: (query: string, direction: "next" | "previous") => void;
    onSendShortcut: (data: string) => void;
    onPaneData: (data: string) => void;
    onPaneNotice: (message: string) => void;
    onRegisterPtyApi: (ptyId: string, api: RemotePtyApi | null) => void;
    onResizePty: (ptyId: string, size: { cols: number; rows: number }) => void;
  } = $props();

  let showShortcuts = $state(true);
  let showSearch = $state(false);
  let showDisplay = $state(false);
  let searchQuery = $state("");
  let fontSize = $state(15);
  let themeId = $state("night");
  let compactLayout = $state(false);

  function phaseLabel(phase: SessionPhase) {
    switch (phase) {
      case "connecting":
        return "Connecting";
      case "creating_pty":
        return "Starting PTY";
      case "disconnecting":
        return "Disconnecting";
      case "ready":
        return "Ready";
      default:
        return "Idle";
    }
  }

  function currentTheme() {
    return terminalThemes.find((item) => item.id === themeId)?.theme ?? terminalThemes[0].theme;
  }

  function adjustFontSize(delta: number) {
    fontSize = Math.max(11, Math.min(24, fontSize + delta));
  }

  function hasActivePty() {
    return Boolean(activePtyId);
  }

  function syncLayoutState() {
    compactLayout = window.innerWidth < 900;
    if (compactLayout) {
      showShortcuts = false;
    } else {
      showShortcuts = true;
    }
  }

  onMount(() => {
    syncLayoutState();
    const handleResize = () => syncLayoutState();
    window.addEventListener("resize", handleResize);
    window.visualViewport?.addEventListener("resize", handleResize);

    return () => {
      window.removeEventListener("resize", handleResize);
      window.visualViewport?.removeEventListener("resize", handleResize);
    };
  });
</script>

<section class="workspace-shell">
  <header class="workspace-header">
    <div class="workspace-copy">
      <span class="eyebrow">Session workspace</span>
      <h1>{agentName || "Remote session"}</h1>
      <p>{status}</p>
      <span class="mode-chip">Mode: {sessionModeLabel(activeMode)}</span>
    </div>

    <div class="header-actions">
      <div class="status-pill">{phaseLabel(sessionPhase)}</div>
      <button class="toolbar-button" onclick={onDisconnect} disabled={busy}>Disconnect</button>
    </div>
  </header>

  <section class="terminal-card">
    <div class="terminal-toolbar">
      <div class="terminal-toolbar-group">
        <button class="toolbar-button" onclick={() => (showShortcuts = !showShortcuts)}>
          {showShortcuts ? "Hide shortcuts" : "Show shortcuts"}
        </button>
        <button class="toolbar-button" onclick={() => (showSearch = !showSearch)}>
          {showSearch ? "Hide search" : "Search"}
        </button>
        <button class="toolbar-button" onclick={() => (showDisplay = !showDisplay)}>
          {showDisplay ? "Hide display" : "Display"}
        </button>
      </div>

      <div class="terminal-toolbar-group">
        <button class="toolbar-button" onclick={onCopyActiveTerminal} disabled={!hasActivePty()}>
          Copy
        </button>
        <button class="toolbar-button" onclick={onCreatePty} disabled={busy}>+ PTY</button>
        <button class="toolbar-button" onclick={onCloseActivePty} disabled={!hasActivePty() || busy}>
          Close PTY
        </button>
        <button class="toolbar-button" onclick={onFocusActivePty} disabled={!hasActivePty()}>
          Focus
        </button>
      </div>
    </div>

    {#if compactLayout}
      <div class="quick-shortcuts">
        {#each mobileModeShortcuts[activeMode] as shortcut}
          <button
            class="quick-shortcut"
            onclick={() => onSendShortcut(shortcut.data)}
            disabled={!hasActivePty() || busy}
          >
            {shortcut.label}
          </button>
        {/each}
        <button class="quick-shortcut more-shortcut" onclick={() => (showShortcuts = !showShortcuts)}>
          更多
        </button>
      </div>
    {/if}

    {#if showSearch}
      <div class="panel-card search-panel">
        <input
          bind:value={searchQuery}
          class="search-input"
          placeholder="Search active terminal"
          onkeydown={(event) => event.key === "Enter" && onSearch(searchQuery, "next")}
        />
        <div class="panel-actions">
          <button class="toolbar-button" onclick={() => onSearch(searchQuery, "previous")} disabled={!searchQuery.trim()}>
            Prev
          </button>
          <button class="toolbar-button" onclick={() => onSearch(searchQuery, "next")} disabled={!searchQuery.trim()}>
            Next
          </button>
        </div>
      </div>
    {/if}

    {#if showDisplay}
      <div class="panel-card display-panel">
        <div class="display-row">
          <span class="panel-label">Font size</span>
          <div class="panel-actions">
            <button class="toolbar-button" onclick={() => adjustFontSize(-1)}>-</button>
            <span class="value-chip">{fontSize}px</span>
            <button class="toolbar-button" onclick={() => adjustFontSize(1)}>+</button>
          </div>
        </div>

        <div class="theme-grid">
          {#each terminalThemes as item}
            <button
              class:active-theme={item.id === themeId}
              class="theme-chip"
              onclick={() => (themeId = item.id)}
            >
              {item.label}
            </button>
          {/each}
        </div>
      </div>
    {/if}

    {#if showShortcuts}
      <div class="shortcut-sections">
        {#each shortcutSections as section}
          <div class="shortcut-section">
            <div class="shortcut-section-title">{section.title}</div>
            {#each section.rows as row}
              <div
                class="shortcut-row"
                style:grid-template-columns={`repeat(${Math.max(row.length, 1)}, minmax(0, 1fr))`}
              >
                {#each row as shortcut}
                  <button
                    class="shortcut"
                    onclick={() => onSendShortcut(shortcut.data)}
                    disabled={!hasActivePty() || busy}
                  >
                    <span>{shortcut.label}</span>
                    {#if shortcut.hint}
                      <small>{shortcut.hint}</small>
                    {/if}
                  </button>
                {/each}
              </div>
            {/each}
          </div>
        {/each}
      </div>
    {/if}

    {#if ptys.length > 0}
      <div class="pty-tabs">
        {#each ptys as pty (pty.pty_id)}
          <button
            class:active-tab={pty.pty_id === activePtyId}
            class="pty-tab"
            onclick={() => onSelectPty(pty.pty_id)}
          >
            <span>{pty.pty_id}</span>
            <small>{pty.shell}{pty.exited ? " · exited" : ""}</small>
          </button>
        {/each}
      </div>

      <div class="terminal-stack">
        {#each ptys as pty (pty.pty_id)}
          <RemotePtyPane
            active={pty.pty_id === activePtyId}
            fontSize={fontSize}
            theme={currentTheme()}
            ondata={onPaneData}
            onnotice={onPaneNotice}
            onregisterApi={(api) => onRegisterPtyApi(pty.pty_id, api)}
            onresize={(size) => onResizePty(pty.pty_id, size)}
          />
        {/each}
      </div>
    {:else}
      <div class="empty-terminal">
        <h2>暂无远程终端</h2>
        <p>连接 agent 后会自动创建第一个 PTY，后续可继续创建多个会话并在这里切换。</p>
      </div>
    {/if}
  </section>
</section>

<style>
  .workspace-shell {
    min-height: 100svh;
    display: grid;
    gap: 1rem;
    padding-top: calc(1rem + env(safe-area-inset-top));
    padding-right: calc(1rem + env(safe-area-inset-right));
    padding-bottom: calc(1rem + env(safe-area-inset-bottom));
    padding-left: calc(1rem + env(safe-area-inset-left));
    box-sizing: border-box;
  }

  .workspace-header,
  .terminal-card,
  .panel-card {
    background: rgba(15, 23, 42, 0.88);
    border: 1px solid rgba(148, 163, 184, 0.18);
    border-radius: 1rem;
    box-shadow: 0 10px 40px rgba(15, 23, 42, 0.45);
  }

  .workspace-header {
    padding: 1rem;
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
  }

  .workspace-copy h1,
  .empty-terminal h2 {
    margin: 0;
  }

  .workspace-copy p {
    margin: 0.35rem 0 0;
    color: #94a3b8;
  }

  .mode-chip {
    display: inline-flex;
    margin-top: 0.7rem;
    padding: 0.3rem 0.65rem;
    border-radius: 999px;
    background: rgba(59, 130, 246, 0.14);
    color: #bfdbfe;
    font-size: 0.82rem;
    width: fit-content;
  }

  .eyebrow {
    display: inline-block;
    margin-bottom: 0.55rem;
    color: #60a5fa;
    font-size: 0.85rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .header-actions,
  .terminal-toolbar,
  .terminal-toolbar-group,
  .display-row {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
  }

  .status-pill {
    padding: 0.35rem 0.7rem;
    border-radius: 999px;
    background: rgba(34, 197, 94, 0.18);
    color: #bbf7d0;
    font-size: 0.875rem;
    white-space: nowrap;
  }

  button,
  input {
    border: 1px solid rgba(148, 163, 184, 0.24);
    border-radius: 0.9rem;
    background: rgba(15, 23, 42, 0.92);
    color: inherit;
    font: inherit;
  }

  button {
    padding: 0.75rem 0.9rem;
    font-weight: 600;
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .terminal-card {
    min-height: 0;
    padding: 0.75rem;
    display: grid;
    gap: 0.75rem;
  }

  .panel-card {
    padding: 0.75rem;
    display: grid;
    gap: 0.75rem;
  }

  .quick-shortcuts {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 0.5rem;
    overflow-x: auto;
  }

  .quick-shortcut {
    min-width: 0;
    white-space: nowrap;
    padding-inline: 0.55rem;
  }

  .more-shortcut {
    border-color: rgba(59, 130, 246, 0.5);
  }

  .search-panel {
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
  }

  .search-input {
    width: 100%;
    padding: 0.85rem 0.9rem;
  }

  .panel-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .panel-label,
  .shortcut-section-title {
    color: #cbd5e1;
    font-size: 0.92rem;
  }

  .value-chip {
    min-width: 4rem;
    text-align: center;
    padding: 0.55rem 0.8rem;
    border-radius: 999px;
    background: rgba(59, 130, 246, 0.16);
  }

  .theme-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.55rem;
  }

  .theme-chip.active-theme {
    border-color: rgba(59, 130, 246, 0.8);
    background: rgba(37, 99, 235, 0.18);
  }

  .shortcut-sections {
    display: grid;
    gap: 0.7rem;
    max-height: min(32svh, 22rem);
    overflow-y: auto;
    padding-right: 0.1rem;
  }

  .shortcut-section {
    display: grid;
    gap: 0.45rem;
  }

  .shortcut-section-title {
    font-weight: 700;
  }

  .shortcut-row {
    display: grid;
    gap: 0.55rem;
  }

  .shortcut {
    padding: 0.75rem 0.6rem;
    min-height: 3.4rem;
    display: grid;
    gap: 0.15rem;
    place-items: center;
  }

  .shortcut small,
  .pty-tab small {
    color: #94a3b8;
    font-size: 0.76rem;
  }

  .pty-tabs {
    display: flex;
    gap: 0.65rem;
    overflow-x: auto;
    padding-bottom: 0.1rem;
  }

  .pty-tab {
    padding: 0.75rem 0.9rem;
    min-width: 9rem;
    text-align: left;
    display: grid;
    gap: 0.15rem;
  }

  .pty-tab.active-tab {
    border-color: rgba(59, 130, 246, 0.8);
    background: rgba(37, 99, 235, 0.18);
  }

  .terminal-stack,
  .empty-terminal {
    min-height: 0;
    height: 100%;
  }

  .empty-terminal {
    display: grid;
    place-items: center;
    text-align: center;
    padding: 1.5rem;
    border-radius: 0.75rem;
    background: #0f172a;
  }

  @media (max-width: 899px) {
    .workspace-header,
    .search-panel {
      grid-template-columns: 1fr;
    }

    .theme-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .shortcut-sections {
      max-height: none;
    }
  }
</style>
