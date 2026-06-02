<script lang="ts">
  import { onMount } from "svelte";
  import type { RemotePtyApi } from "$lib/remote-pty-types";
  import RemotePtyPane from "$lib/RemotePtyPane.svelte";
  import SessionBar from "$lib/SessionBar.svelte";
  import MenuBar from "$lib/MenuBar.svelte";
  import DesktopShortcuts from "$lib/DesktopShortcuts.svelte";
  import SearchPanel from "$lib/SearchPanel.svelte";

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
    onSearch: (query: string, direction: "next" | "previous") => boolean;
    onSendShortcut: (data: string) => void;
    onPaneData: (data: string) => void;
    onPaneNotice: (message: string) => void;
    onRegisterPtyApi: (ptyId: string, api: RemotePtyApi | null) => void;
    onResizePty: (ptyId: string, size: { cols: number; rows: number }) => void;
  } = $props();

  type MobilePanel = "shortcuts" | "search";
  type MobilePlatform = "ios" | "android" | "other";

  const DESKTOP_FONT_SIZE = 15;
  const TABLET_COMPACT_FONT_SIZE = 14;
  const PHONE_COMPACT_FONT_SIZE = 13;

  let showShortcuts = $state(false);
  let showSearch = $state(false);
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

  /* ── split pane ── */
  let splitPtyId = $state<string | null>(null);
  let pendingSplitDir = $state<"right" | "left" | null>(null);
  let splitRatio = $state(0.5);
  const SPLIT_RATIO_KEY = "dumbpipex:split-ratio";
  const SPLIT_RATIO_MIN = 0.2;
  const SPLIT_RATIO_MAX = 0.8;
  let isDraggingSplit = $state(false);
  let splitContainerEl = $state<HTMLElement | null>(null);

  function clampSplitRatio(value: number) {
    if (Number.isNaN(value)) return 0.5;
    return Math.min(SPLIT_RATIO_MAX, Math.max(SPLIT_RATIO_MIN, value));
  }

  function loadSplitRatio() {
    if (typeof localStorage === "undefined") return;
    const raw = localStorage.getItem(SPLIT_RATIO_KEY);
    if (!raw) return;
    const parsed = Number.parseFloat(raw);
    if (Number.isFinite(parsed)) splitRatio = clampSplitRatio(parsed);
  }

  function persistSplitRatio(value: number) {
    if (typeof localStorage === "undefined") return;
    try {
      localStorage.setItem(SPLIT_RATIO_KEY, value.toString());
    } catch {
      // ignore quota / private-mode errors
    }
  }

  /* ── keyboard detection ── */
  let baseWindowHeight = $state(0);
  let baseViewportHeight = $state(0);
  let focusInputCount = $state(0);

  function currentViewportHeight() {
    return window.visualViewport?.height ?? window.innerHeight;
  }

  function detectKeyboard() {
    if (typeof window === "undefined") return;

    const winH = window.innerHeight;
    const vvH = currentViewportHeight();

    // 基线取两者较大值（初始化时键盘大概率未弹出）
    const baseline = Math.max(baseWindowHeight, baseViewportHeight);
    const diff = baseline === 0 ? 0 : baseline - Math.min(winH, vvH);
    const threshold = Math.max(100, baseline * 0.12);

    // Two signals: viewport-shrink (best on portrait) OR a text input
    // currently focused (best on landscape, where WebViews sometimes
    // don't resize at all). Whichever fires wins.
    const viewportSays = baseline > 0 && diff > threshold;
    const focusSays = focusInputCount > 0;
    const next = viewportSays || focusSays;

    if (next !== keyboardOpen) {
      keyboardOpen = next;
      if (next) {
        // 键盘弹出时关闭浮动面板、收起 header
        mobilePanel = null;
        applyMobilePanelState(null);
      }
    }
  }

  function isTextInput(node: Element | null): boolean {
    if (!node) return false;
    const tag = node.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
    if ((node as HTMLElement).isContentEditable) return true;
    return false;
  }

  function handleFocusIn(event: FocusEvent) {
    if (isTextInput(event.target as Element)) {
      focusInputCount += 1;
      // Re-evaluate immediately so the layout responds before the
      // viewport resize event lands (WebView animations are slow).
      detectKeyboard();
    }
  }

  function handleFocusOut(event: FocusEvent) {
    if (isTextInput(event.target as Element)) {
      focusInputCount = Math.max(0, focusInputCount - 1);
      // Defer one frame — focusout fires before the next focusin if the
      // user tabs between inputs, and we don't want a flicker.
      requestAnimationFrame(detectKeyboard);
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

  function increaseFont() {
    adjustFontSize(1);
  }

  function decreaseFont() {
    adjustFontSize(-1);
  }

  function resetFont() {
    fontSizeCustomized = false;
    fontSize = compactLayout
      ? phoneCompactLayout
        ? PHONE_COMPACT_FONT_SIZE
        : TABLET_COMPACT_FONT_SIZE
      : DESKTOP_FONT_SIZE;
  }

  function toggleSearchPanel() {
    showSearch = !showSearch;
  }

  function closeSearch() {
    showSearch = false;
  }

  function selectTheme(id: string) {
    themeId = id;
  }

  const hasActivePty = $derived(Boolean(activePtyId));

  function applyMobilePanelState(panel: MobilePanel | null) {
    showShortcuts = panel === "shortcuts";
    showSearch = panel === "search";
  }

  function togglePanel(panel: MobilePanel) {
    if (!compactLayout) {
      if (panel === "shortcuts") showShortcuts = !showShortcuts;
      if (panel === "search") showSearch = !showSearch;
      return;
    }
    mobilePanel = mobilePanel === panel ? null : panel;
    applyMobilePanelState(mobilePanel);
  }

  /* ── split pane functions ── */
  function handleSplitRight() {
    if (splitPtyId || pendingSplitDir) return;
    pendingSplitDir = "right";
    onCreatePty();
  }

  function handleSplitLeft() {
    if (splitPtyId || pendingSplitDir) return;
    pendingSplitDir = "left";
    onCreatePty();
  }

  function closeSplit() {
    splitPtyId = null;
    pendingSplitDir = null;
    isDraggingSplit = false;
  }

  function handleSplitDividerPointerDown(event: PointerEvent) {
    if (!splitPtyId) return;
    event.preventDefault();
    isDraggingSplit = true;
    const target = event.currentTarget as HTMLElement;
    target.setPointerCapture?.(event.pointerId);
  }

  function handleSplitDividerPointerMove(event: PointerEvent) {
    if (!isDraggingSplit || !splitContainerEl) return;
    const rect = splitContainerEl.getBoundingClientRect();
    if (rect.width <= 0) return;
    const offset = event.clientX - rect.left;
    const next = clampSplitRatio(offset / rect.width);
    if (Math.abs(next - splitRatio) > 0.001) splitRatio = next;
  }

  function endSplitDrag(event: PointerEvent) {
    if (!isDraggingSplit) return;
    isDraggingSplit = false;
    const target = event.currentTarget as HTMLElement;
    target.releasePointerCapture?.(event.pointerId);
    persistSplitRatio(splitRatio);
  }

  function handleSplitDividerKeyDown(event: KeyboardEvent) {
    if (!splitPtyId) return;
    const step = event.shiftKey ? 0.1 : 0.02;
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      splitRatio = clampSplitRatio(splitRatio - step);
      persistSplitRatio(splitRatio);
    } else if (event.key === "ArrowRight") {
      event.preventDefault();
      splitRatio = clampSplitRatio(splitRatio + step);
      persistSplitRatio(splitRatio);
    } else if (event.key === "Home") {
      event.preventDefault();
      splitRatio = 0.5;
      persistSplitRatio(splitRatio);
    }
  }

  // Watch for a new PTY to arrive after split request
  $effect(() => {
    if (!pendingSplitDir || ptys.length === 0) return;
    const newest = ptys[ptys.length - 1];
    if (newest.pty_id === activePtyId || newest.exited) return;
    splitPtyId = newest.pty_id;
    pendingSplitDir = null;
  });

  // Exit split when split PTY exits
  $effect(() => {
    if (!splitPtyId) return;
    const splitPty = ptys.find((p) => p.pty_id === splitPtyId);
    if (!splitPty || splitPty.exited) splitPtyId = null;
  });

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
      showShortcuts = false;
      showSearch = false;
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
    loadSplitRatio();
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
    // focusin/focusout capture fires even on iOS WebView landscape
    // (where visualViewport sometimes doesn't shrink at all when the
    // soft keyboard appears).
    document.addEventListener("focusin", handleFocusIn);
    document.addEventListener("focusout", handleFocusOut);

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
      document.removeEventListener("focusin", handleFocusIn);
      document.removeEventListener("focusout", handleFocusOut);
      ro?.disconnect();
    };
  });
</script>

<section
  class:phone-compact={phoneCompactLayout}
  class:keyboard-open={keyboardOpen}
  class="workspace-shell"
>
  <MenuBar
    {agentName}
    {sessionPhase}
    {fontSize}
    {showSearch}
    compact={compactLayout}
    hasActivePty={hasActivePty}
    themes={terminalThemes}
    {themeId}
    onNewTab={onCreatePty}
    onCloseTab={onCloseActivePty}
    onSplitRight={handleSplitRight}
    onSplitLeft={handleSplitLeft}
    splitActive={!!splitPtyId}
    onCloseSplit={closeSplit}
    onDisconnect={onDisconnect}
    onIncreaseFont={increaseFont}
    onDecreaseFont={decreaseFont}
    onResetFont={resetFont}
    onToggleSearch={toggleSearchPanel}
    onSelectTheme={selectTheme}
  />

  <section class="terminal-card" bind:this={terminalCardRef}>
    <SessionBar
      {ptys}
      {activePtyId}
      {compactLayout}
      {keyboardOpen}
      {busy}
      onSelectPty={onSelectPty}
      onCreatePty={onCreatePty}
      onCloseActivePty={onCloseActivePty}
    />

    {#if !compactLayout && showSearch}
      <SearchPanel bind:searchQuery onSearch={onSearch} onClose={closeSearch} />
    {/if}


    {#if ptys.length > 0}
      <div
        class="terminal-stack"
        class:split={!!splitPtyId}
        class:dragging={isDraggingSplit}
        bind:this={splitContainerEl}
      >
        {#if !splitPtyId}
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
        {:else}
          {@const splitPty = ptys.find((p) => p.pty_id === splitPtyId)}
          {@const activePty = ptys.find((p) => p.pty_id === activePtyId)}
          {@const splitLeft = pendingSplitDir === "left"}
          {#if activePty}
            {#if splitLeft && splitPty}
              <RemotePtyPane
                active
                flexBasisPct={splitRatio * 100}
                {fontSize}
                theme={currentTheme()}
                ondata={onPaneData}
                onnotice={onPaneNotice}
                onregisterApi={(api) => onRegisterPtyApi(splitPty.pty_id, api)}
                onresize={(size) => onResizePty(splitPty.pty_id, size)}
              />
            {/if}
            <RemotePtyPane
              active
              flexBasisPct={splitLeft ? (1 - splitRatio) * 100 : splitRatio * 100}
              {fontSize}
              theme={currentTheme()}
              ondata={onPaneData}
              onnotice={onPaneNotice}
              onregisterApi={(api) => onRegisterPtyApi(activePty.pty_id, api)}
              onresize={(size) => onResizePty(activePty.pty_id, size)}
            />
            <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
            <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
            <div
              class="split-divider"
              role="separator"
              tabindex="0"
              aria-orientation="vertical"
              aria-valuenow={Math.round(splitRatio * 100)}
              aria-valuemin={Math.round(SPLIT_RATIO_MIN * 100)}
              aria-valuemax={Math.round(SPLIT_RATIO_MAX * 100)}
              aria-label="调整分屏宽度"
              title="拖动调整分屏宽度（← → 精细调节）"
              onpointerdown={handleSplitDividerPointerDown}
              onpointermove={handleSplitDividerPointerMove}
              onpointerup={endSplitDrag}
              onpointercancel={endSplitDrag}
              onlostpointercapture={endSplitDrag}
              onkeydown={handleSplitDividerKeyDown}
            >
              <span class="split-grip" aria-hidden="true"></span>
            </div>
            {#if !splitLeft && splitPty}
              <RemotePtyPane
                active
                flexBasisPct={(1 - splitRatio) * 100}
                {fontSize}
                theme={currentTheme()}
                ondata={onPaneData}
                onnotice={onPaneNotice}
                onregisterApi={(api) => onRegisterPtyApi(splitPty.pty_id, api)}
                onresize={(size) => onResizePty(splitPty.pty_id, size)}
              />
            {/if}
          {/if}
        {/if}
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
        hasActivePty={hasActivePty}
        {busy}
        onSendShortcut={onSendShortcut}
        onPaneNotice={onPaneNotice}
        onPaneData={onPaneData}
        onFocusActivePty={onFocusActivePty}
      />

    {/if}

    {#if mobilePanel && !keyboardOpen}
      <MobileSheet
        title={mobilePanel === "search" ? "搜索终端" : "快捷键"}
        subtitle={`${sessionModeLabel(activeMode)} · 针对触摸操作优化`}
        onClose={closeMobilePanel}
      >
        {#if showSearch}
          <SearchPanel bind:searchQuery onSearch={onSearch} onClose={closeMobilePanel} />
        {:else if showShortcuts}
          <DesktopShortcuts hasActivePty={hasActivePty} {busy} onSendShortcut={onSendShortcut} />
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
    /* safe-area-inset-* with a 1.2rem fallback for devices that don't
       report one. The fallback doubles as the minimum visual padding
       so the content never sits flush against the screen edge. */
    padding-top: max(1.2rem, env(safe-area-inset-top, 1.2rem));
    padding-right: calc(1rem + env(safe-area-inset-right, 0px));
    padding-bottom: max(1rem, env(safe-area-inset-bottom, 1rem));
    padding-left: calc(1rem + env(safe-area-inset-left, 0px));
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


  .terminal-stack {
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    transition: height 200ms ease;
  }

  /* split layout */

  .terminal-stack.split {
    flex-direction: row;
    gap: 0;
  }

  .terminal-stack.split > :global(.pane.active) {
    flex-grow: 1;
    flex-shrink: 1;
    min-width: 0;
  }

  .terminal-stack.split.dragging > :global(.pane.active) {
    transition: none;
  }

  .split-divider {
    /* Visual width: 6px. Hit zone: 28px (44px target not possible in
       a 2-pane horizontal split; 28px is the compromise). */
    position: relative;
    width: 6px;
    flex-shrink: 0;
    flex-grow: 0;
    background: rgba(148, 163, 184, 0.18);
    cursor: col-resize;
    display: flex;
    align-items: center;
    justify-content: center;
    touch-action: none;
    user-select: none;
    transition: background-color 140ms ease;
  }

  /* Invisible hit-area extension on phone */
  .split-divider::before {
    content: "";
    position: absolute;
    top: 0;
    bottom: 0;
    left: -11px;
    right: -11px;
  }

  .split-divider:hover,
  .split-divider:focus-visible {
    background: rgba(59, 130, 246, 0.4);
    outline: none;
  }

  .split-divider:focus-visible::after {
    content: "";
    position: absolute;
    inset: -2px;
    border: 2px solid rgba(59, 130, 246, 0.6);
    border-radius: 2px;
    pointer-events: none;
  }

  .terminal-stack.dragging .split-divider {
    background: rgba(59, 130, 246, 0.55);
  }

  .split-grip {
    width: 2px;
    height: 1.6rem;
    border-radius: 999px;
    background: rgba(148, 163, 184, 0.55);
    transition: background-color 140ms ease, height 140ms ease;
  }

  .split-divider:hover .split-grip,
  .split-divider:focus-visible .split-grip,
  .terminal-stack.dragging .split-grip {
    background: rgba(219, 234, 254, 0.85);
    height: 2.4rem;
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

  /* ===== keyboard-open (通用) ===== */

  .workspace-shell.keyboard-open {
    gap: 0.4rem;
    /* When keyboard is up the top safe-area shrinks to 0 in most
       WebViews; preserve a small breathing room above the menu bar. */
    padding-top: max(0.4rem, env(safe-area-inset-top, 0.4rem));
    padding-bottom: max(0.2rem, env(safe-area-inset-bottom, 0.2rem));
  }

  .workspace-shell.keyboard-open .terminal-card {
    padding: 0.4rem;
    gap: 0.4rem;
    border-radius: 0.75rem;
  }


  /* ===== responsive (max-width: 899px) ===== */

  @media (max-width: 899px) {
    .workspace-shell {
      gap: 0.6rem;
      padding-top: max(0.6rem, env(safe-area-inset-top, 0.6rem));
      padding-right: calc(0.6rem + env(safe-area-inset-right, 0px));
      padding-bottom: max(0.3rem, env(safe-area-inset-bottom, 0.3rem));
      padding-left: calc(0.6rem + env(safe-area-inset-left, 0px));
    }

    .terminal-card {
      padding: 0.55rem;
      gap: 0.55rem;
      border-radius: 0.85rem;
    }

    .workspace-shell.keyboard-open {
      gap: 0.35rem;
      padding-top: max(0.35rem, env(safe-area-inset-top, 0.35rem));
      padding-bottom: max(0.15rem, env(safe-area-inset-bottom, 0.15rem));
    }

    .workspace-shell.keyboard-open .terminal-card {
      padding: 0.35rem;
      gap: 0.35rem;
      border-radius: 0.7rem;
    }
  }
</style>
