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
  let keyboardOpen = $state(false);
  let terminalCardRef = $state<HTMLElement | null>(null);

  /* ── keyboard detection ── */
  let baseWindowHeight = $state(0);
  let baseViewportHeight = $state(0);

  function currentViewportHeight() {
    return window.visualViewport?.height ?? window.innerHeight;
  }

  function detectKeyboard() {
    if (typeof window === "undefined") return;

    const winH = window.innerHeight;
    const vvH = currentViewportHeight();

    // 基线取两者较大值（初始化时键盘大概率未弹出）
    const baseline = Math.max(baseWindowHeight, baseViewportHeight);
    if (baseline === 0) return;

    const diff = baseline - Math.min(winH, vvH);
    const threshold = Math.max(100, baseline * 0.12);
    const next = compactLayout && diff > threshold;

    if (next !== keyboardOpen) {
      keyboardOpen = next;
      if (next) {
        // 键盘弹出时关闭浮动面板、收起 header
        mobilePanel = null;
        applyMobilePanelState(null);
      }
    }
  }

  function recordBaseline() {
    // 延迟记录，避开启动时可能的 transient resize
    requestAnimationFrame(() => {
      baseWindowHeight = Math.max(baseWindowHeight, window.innerHeight);
      baseViewportHeight = Math.max(baseViewportHeight, currentViewportHeight());
      detectKeyboard();
    });
  }

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

    // 初始化基线（延迟确保布局稳定）
    recordBaseline();
    const baselineTimer = setTimeout(recordBaseline, 500);

    const handleResize = () => {
      syncLayoutState();
      detectKeyboard();
    };

    window.addEventListener("resize", handleResize);
    window.visualViewport?.addEventListener("resize", detectKeyboard);

    // ResizeObserver 监控 terminal-card 实际可用高度
    let ro: ResizeObserver | null = null;
    if (terminalCardRef && "ResizeObserver" in window) {
      ro = new ResizeObserver(() => detectKeyboard());
      ro.observe(terminalCardRef);
    }

    return () => {
      clearTimeout(baselineTimer);
      window.removeEventListener("resize", handleResize);
      window.visualViewport?.removeEventListener("resize", detectKeyboard);
      ro?.disconnect();
    };
  });
</script>

<section
  class:phone-compact={phoneCompactLayout}
  class:keyboard-open={keyboardOpen}
  class="workspace-shell"
>
  <SessionHeader
    {agentName}
    {status}
    {activeMode}
    {sessionPhase}
    {compactLayout}
    {busy}
    onDisconnect={onDisconnect}
  />

  <section class="terminal-card" bind:this={terminalCardRef}>
    {#if !keyboardOpen}
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
    {/if}

    <SessionBar
      {ptys}
      {activePtyId}
      {compactLayout}
      {keyboardOpen}
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
      <div class="empty-terminal" class:hidden={keyboardOpen}>
        <h2>暂无远程终端</h2>
        <p>连接 agent 后会自动创建第一个 PTY，后续可继续创建多个会话并在这里切换。</p>
      </div>
    {/if}

    {#if compactLayout && !keyboardOpen}
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

    {#if mobilePanel && !keyboardOpen}
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
    height: 100%;
    max-height: 100%;
    min-width: 0;
    display: grid;
    gap: 1rem;
    padding-top: max(1.5rem, calc(1rem + env(safe-area-inset-top)));
    padding-right: calc(1rem + env(safe-area-inset-right));
    padding-bottom: max(1rem, calc(0.5rem + env(safe-area-inset-bottom)));
    padding-left: calc(1rem + env(safe-area-inset-left));
    box-sizing: border-box;
    grid-template-rows: auto 1fr;
    overflow: hidden;
  }

  .terminal-card {
    min-height: 0;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    background: rgba(15, 23, 42, 0.88);
    border: 1px solid rgba(148, 163, 184, 0.18);
    border-radius: 1rem;
    box-shadow: 0 10px 40px rgba(15, 23, 42, 0.45);
    overflow: hidden;
    transition: padding 200ms ease, gap 200ms ease;
  }

  .terminal-toolbar-shell {
    display: grid;
    gap: 0.75rem;
    flex-shrink: 0;
    transition:
      opacity 200ms ease,
      height 200ms ease,
      margin 200ms ease;
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
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    transition: height 200ms ease;
  }

  .empty-terminal {
    flex: 1 1 auto;
    display: grid;
    place-items: center;
    text-align: center;
    padding: 1.5rem;
    border-radius: 0.75rem;
    background: #0f172a;
    min-height: 0;
    transition: opacity 200ms ease;
  }

  .empty-terminal.hidden {
    display: none;
  }

  .empty-terminal h2 {
    margin: 0;
  }

  /* ===== phone-compact ===== */
  .workspace-shell.phone-compact {
    gap: 0.6rem;
    padding-top: max(1.2rem, calc(0.6rem + env(safe-area-inset-top)));
    padding-right: calc(0.6rem + env(safe-area-inset-right));
    padding-bottom: max(0.6rem, calc(0.3rem + env(safe-area-inset-bottom)));
    padding-left: calc(0.6rem + env(safe-area-inset-left));
  }

  .workspace-shell.phone-compact .terminal-card {
    padding: 0.55rem;
    gap: 0.55rem;
    border-radius: 0.85rem;
  }

  .workspace-shell.phone-compact .terminal-toolbar-card {
    padding: 0.45rem;
    gap: 0.45rem;
    border-radius: 0.8rem;
  }

  .workspace-shell.phone-compact .terminal-toolbar,
  .workspace-shell.phone-compact .terminal-toolbar-group {
    gap: 0.4rem;
  }

  .workspace-shell.phone-compact .terminal-toolbar-group {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    align-items: stretch;
  }

  .workspace-shell.phone-compact .toolbar-button {
    min-height: 1.95rem;
    padding: 0.35rem 0.5rem;
    font-size: 0.76rem;
  }

  /* ===== keyboard-open (通用) ===== */
  .workspace-shell.keyboard-open {
    gap: 0.4rem;
    padding-top: max(1.2rem, calc(0.4rem + env(safe-area-inset-top)));
    padding-bottom: max(0.3rem, calc(0.2rem + env(safe-area-inset-bottom)));
  }

  .workspace-shell.keyboard-open .terminal-card {
    padding: 0.4rem;
    gap: 0.4rem;
    border-radius: 0.75rem;
  }

  .workspace-shell.keyboard-open .terminal-toolbar-shell {
    display: none;
  }

  /* ===== responsive (max-width: 899px) ===== */
  @media (max-width: 899px) {
    .workspace-shell {
      gap: 0.6rem;
      padding-top: max(1.2rem, calc(0.6rem + env(safe-area-inset-top)));
      padding-right: calc(0.6rem + env(safe-area-inset-right));
      padding-bottom: max(0.6rem, calc(0.3rem + env(safe-area-inset-bottom)));
      padding-left: calc(0.6rem + env(safe-area-inset-left));
    }

    .terminal-card {
      padding: 0.55rem;
      gap: 0.55rem;
      border-radius: 0.85rem;
    }

    .terminal-toolbar-card {
      padding: 0.45rem;
      gap: 0.45rem;
      border-radius: 0.8rem;
    }

    .terminal-toolbar {
      flex-direction: column;
      align-items: stretch;
    }

    .terminal-toolbar-group {
      justify-content: flex-start;
      gap: 0.4rem;
    }

    .toolbar-button {
      flex: 1 1 calc(50% - 0.375rem);
      min-height: 2.4rem;
      font-size: 0.88rem;
      padding: 0.55rem 0.7rem;
    }

    .workspace-shell.keyboard-open {
      gap: 0.35rem;
      padding-top: max(1.2rem, calc(0.35rem + env(safe-area-inset-top)));
      padding-bottom: max(0.25rem, calc(0.15rem + env(safe-area-inset-bottom)));
    }

    .workspace-shell.keyboard-open .terminal-card {
      padding: 0.35rem;
      gap: 0.35rem;
      border-radius: 0.7rem;
    }

    .workspace-shell.keyboard-open .terminal-toolbar-group {
      gap: 0.25rem;
    }

    .workspace-shell.keyboard-open .toolbar-button {
      min-height: 1.85rem;
      padding: 0.3rem 0.45rem;
      font-size: 0.72rem;
    }
  }
</style>
