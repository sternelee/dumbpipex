<script lang="ts">
  import { onMount } from "svelte";
  import type { RemotePtyApi } from "$lib/remote-pty-types";
  import RemotePtyPane from "$lib/RemotePtyPane.svelte";
  import SessionHeader from "$lib/SessionHeader.svelte";
  import SessionBar from "$lib/SessionBar.svelte";
  import DesktopShortcuts from "$lib/DesktopShortcuts.svelte";
  import SearchPanel from "$lib/SearchPanel.svelte";
  import DisplayPanel from "$lib/DisplayPanel.svelte";
  import MobileShortcutBar from "$lib/MobileShortcutBar.svelte";
  import MobileSheet from "$lib/MobileSheet.svelte";
  import { terminalThemes, sessionModeLabel } from "$lib/terminal-ui";
  import type { PtySession, SessionMode, SessionPhase } from "$lib/terminal-ui";

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

  type MobilePanel = "shortcuts" | "search" | "display";
  type MobilePlatform = "ios" | "android" | "other";

  const DESKTOP_FONT_SIZE = 15;
  const TABLET_COMPACT_FONT_SIZE = 14;
  const PHONE_COMPACT_FONT_SIZE = 13;

  let showShortcuts = $state(true);
  let showSearch = $state(false);
  let showDisplay = $state(false);
  let searchQuery = $state("");
  let fontSize = $state(DESKTOP_FONT_SIZE);
  let fontSizeCustomized = $state(false);
  let themeId = $state("night");
  let compactLayout = $state(false);
  let phoneCompactLayout = $state(false);
  let mobilePanel = $state<MobilePanel | null>(null);
  let mobilePlatform = $state<MobilePlatform>("other");

  function currentTheme() {
    return terminalThemes.find((item) => item.id === themeId)?.theme ?? terminalThemes[0].theme;
  }

  function adjustFontSize(delta: number) {
    fontSizeCustomized = true;
    fontSize = Math.max(11, Math.min(24, fontSize + delta));
  }

  function hasActivePty() {
    return Boolean(activePtyId);
  }

  function applyMobilePanelState(panel: MobilePanel | null) {
    showShortcuts = panel === "shortcuts";
    showSearch = panel === "search";
    showDisplay = panel === "display";
  }

  function togglePanel(panel: MobilePanel) {
    if (!compactLayout) {
      if (panel === "shortcuts") showShortcuts = !showShortcuts;
      if (panel === "search") showSearch = !showSearch;
      if (panel === "display") showDisplay = !showDisplay;
      return;
    }
    mobilePanel = mobilePanel === panel ? null : panel;
    applyMobilePanelState(mobilePanel);
  }

  function syncLayoutState() {
    const nextCompact = window.innerWidth < 900;
    const nextPhoneCompact = nextCompact && window.innerWidth < 680;
    compactLayout = nextCompact;
    phoneCompactLayout = nextPhoneCompact;

    if (!fontSizeCustomized) {
      fontSize = nextCompact
        ? nextPhoneCompact
          ? PHONE_COMPACT_FONT_SIZE
          : TABLET_COMPACT_FONT_SIZE
        : DESKTOP_FONT_SIZE;
    }

    if (nextCompact) {
      applyMobilePanelState(mobilePanel);
    } else {
      mobilePanel = null;
      showShortcuts = true;
      showSearch = false;
      showDisplay = false;
    }
  }

  function detectMobilePlatform(): MobilePlatform {
    if (typeof navigator === "undefined") return "other";
    const ua = navigator.userAgent.toLowerCase();
    if (/iphone|ipad|ipod/.test(ua)) return "ios";
    if (ua.includes("android")) return "android";
    return "other";
  }

  function closeMobilePanel() {
    if (!compactLayout) return;
    mobilePanel = null;
    applyMobilePanelState(null);
  }

  onMount(() => {
    mobilePlatform = detectMobilePlatform();
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

<section class:phone-compact={phoneCompactLayout} class="workspace-shell">
  <SessionHeader
    {agentName}
    {status}
    {activeMode}
    {sessionPhase}
    {compactLayout}
    {busy}
    onDisconnect={onDisconnect}
  />

  <section class="terminal-card">
    <div
      class:panel-card={compactLayout}
      class:terminal-toolbar-card={compactLayout}
      class="terminal-toolbar-shell"
    >
      <div class="terminal-toolbar">
        <div class="terminal-toolbar-group">
          <button class="toolbar-button" onclick={() => togglePanel("shortcuts")}>
            {compactLayout ? "快捷键" : showShortcuts ? "Hide shortcuts" : "Show shortcuts"}
          </button>
          <button class="toolbar-button" onclick={() => togglePanel("search")}>
            {compactLayout ? "搜索" : showSearch ? "Hide search" : "Search"}
          </button>
          <button class="toolbar-button" onclick={() => togglePanel("display")}>
            {compactLayout ? "显示" : showDisplay ? "Hide display" : "Display"}
          </button>
        </div>

        {#if !compactLayout}
          <div class="terminal-toolbar-group">
            <button class="toolbar-button" onclick={onCopyActiveTerminal} disabled={!hasActivePty()}>
              Copy
            </button>
            <button class="toolbar-button" onclick={onCreatePty} disabled={busy}>+ PTY</button>
            <button
              class="toolbar-button"
              onclick={onCloseActivePty}
              disabled={!hasActivePty() || busy}
            >
              Close PTY
            </button>
            <button class="toolbar-button" onclick={onFocusActivePty} disabled={!hasActivePty()}>
              Focus
            </button>
          </div>
        {/if}
      </div>
    </div>

    <SessionBar
      {ptys}
      {activePtyId}
      {compactLayout}
      {busy}
      onSelectPty={onSelectPty}
      onCreatePty={onCreatePty}
      onCloseActivePty={onCloseActivePty}
      onFocusActivePty={onFocusActivePty}
      onCopyActiveTerminal={onCopyActiveTerminal}
      onPaneData={onPaneData}
      onPaneNotice={onPaneNotice}
    />

    {#if !compactLayout && showSearch}
      <SearchPanel bind:searchQuery onSearch={onSearch} />
    {/if}

    {#if !compactLayout && showDisplay}
      <DisplayPanel {fontSize} bind:themeId onAdjustFontSize={adjustFontSize} />
    {/if}

    {#if !compactLayout && showShortcuts}
      <DesktopShortcuts hasActivePty={hasActivePty()} {busy} onSendShortcut={onSendShortcut} />
    {/if}

    {#if ptys.length > 0}
      <div class="terminal-stack">
        {#each ptys as pty (pty.pty_id)}
          <RemotePtyPane
            active={pty.pty_id === activePtyId}
            {fontSize}
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

    {#if compactLayout}
      <MobileShortcutBar
        {compactLayout}
        {phoneCompactLayout}
        {activeMode}
        {mobilePlatform}
        hasActivePty={hasActivePty()}
        {busy}
        onSendShortcut={onSendShortcut}
        onPaneNotice={onPaneNotice}
        onPaneData={onPaneData}
        onFocusActivePty={onFocusActivePty}
        onTogglePanel={() => togglePanel("shortcuts")}
      />
    {/if}

    {#if mobilePanel}
      <MobileSheet
        title={mobilePanel === "search"
          ? "搜索终端"
          : mobilePanel === "display"
            ? "显示设置"
            : "完整快捷键"}
        subtitle={`${sessionModeLabel(activeMode)} · 针对触摸操作优化`}
        onClose={closeMobilePanel}
      >
        {#if showSearch}
          <SearchPanel bind:searchQuery onSearch={onSearch} />
        {:else if showDisplay}
          <DisplayPanel {fontSize} bind:themeId onAdjustFontSize={adjustFontSize} />
        {:else if showShortcuts}
          <DesktopShortcuts hasActivePty={hasActivePty()} {busy} onSendShortcut={onSendShortcut} />
        {/if}
      </MobileSheet>
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

  .terminal-card {
    min-height: 0;
    padding: 0.75rem;
    display: grid;
    gap: 0.75rem;
    background: rgba(15, 23, 42, 0.88);
    border: 1px solid rgba(148, 163, 184, 0.18);
    border-radius: 1rem;
    box-shadow: 0 10px 40px rgba(15, 23, 42, 0.45);
  }

  .terminal-toolbar-shell {
    display: grid;
    gap: 0.75rem;
  }

  .terminal-toolbar-card {
    padding: 0.62rem;
    gap: 0.62rem;
    background: rgba(15, 23, 42, 0.88);
    border: 1px solid rgba(148, 163, 184, 0.18);
    border-radius: 1rem;
    box-shadow: 0 10px 40px rgba(15, 23, 42, 0.45);
  }

  .terminal-toolbar {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
  }

  .terminal-toolbar-group {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .toolbar-button {
    border: 1px solid rgba(148, 163, 184, 0.24);
    border-radius: 0.9rem;
    background: rgba(15, 23, 42, 0.92);
    color: inherit;
    font: inherit;
    min-height: 2.9rem;
    touch-action: manipulation;
    padding: 0.75rem 0.9rem;
    font-weight: 600;
  }

  .toolbar-button:disabled {
    opacity: 0.5;
  }

  .terminal-stack {
    min-height: 18rem;
    height: clamp(18rem, 48dvh, 36rem);
  }

  .terminal-stack,
  .empty-terminal {
    min-height: 0;
  }

  .empty-terminal {
    display: grid;
    place-items: center;
    text-align: center;
    padding: 1.5rem;
    border-radius: 0.75rem;
    background: #0f172a;
  }

  .empty-terminal h2 {
    margin: 0;
  }

  .workspace-shell.phone-compact {
    gap: 0.8rem;
    padding-top: calc(0.8rem + env(safe-area-inset-top));
    padding-right: calc(0.8rem + env(safe-area-inset-right));
    padding-bottom: calc(0.8rem + env(safe-area-inset-bottom));
    padding-left: calc(0.8rem + env(safe-area-inset-left));
  }

  .workspace-shell.phone-compact .terminal-card,
  .workspace-shell.phone-compact .terminal-toolbar-card {
    border-radius: 0.9rem;
  }

  .workspace-shell.phone-compact .terminal-card {
    padding: 0.65rem;
    gap: 0.65rem;
  }

  .workspace-shell.phone-compact .terminal-toolbar-card {
    padding: 0.5rem;
    gap: 0.5rem;
  }

  .workspace-shell.phone-compact .terminal-toolbar,
  .workspace-shell.phone-compact .terminal-toolbar-group {
    gap: 0.5rem;
  }

  .workspace-shell.phone-compact .terminal-toolbar-group {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    align-items: stretch;
  }

  .workspace-shell.phone-compact .toolbar-button {
    min-height: 2.05rem;
    padding: 0.38rem 0.58rem;
    font-size: 0.76rem;
  }

  @media (max-width: 899px) {
    .workspace-shell {
      gap: 0.8rem;
    }

    .terminal-toolbar {
      flex-direction: column;
      align-items: stretch;
    }

    .terminal-toolbar-group {
      justify-content: flex-start;
    }

    .toolbar-button {
      flex: 1 1 calc(50% - 0.375rem);
      min-height: 2.6rem;
      font-size: 0.92rem;
      padding: 0.62rem 0.78rem;
    }

    .terminal-toolbar-card {
      padding: 0.58rem;
      gap: 0.52rem;
    }

    .terminal-card {
      padding: 0.58rem;
      gap: 0.52rem;
    }

    .terminal-stack {
      height: clamp(16rem, 42dvh, 24rem);
    }
  }
</style>
