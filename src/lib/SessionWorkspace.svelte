<script lang="ts">
  import { onMount } from "svelte";
  import type { RemotePtyApi } from "$lib/remote-pty-types";
  import RemotePtyPane from "$lib/RemotePtyPane.svelte";
  import {
    mobileModeShortcuts,
    prioritizeMobileShortcuts,
    shortcutSections,
    sessionModeLabel,
    terminalThemes,
  } from "$lib/terminal-ui";
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
  type StickyModifier = "ctrl" | "alt" | "esc";
  type MobileShortcutButton = { label: string; data: string; repeatable?: boolean };

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
  let activeModifiers = $state<StickyModifier[]>([]);
  let lockedModifiers = $state<StickyModifier[]>([]);
  let modifierUsesRemaining = $state<Record<StickyModifier, number>>({
    ctrl: 0,
    alt: 0,
    esc: 0,
  });
  let activeMobileShortcuts = $derived(
    prioritizeMobileShortcuts(
      mobileModeShortcuts[activeMode] as MobileShortcutButton[],
      mobilePlatform,
    ),
  );
  let navigationShortcuts = $derived(
    activeMobileShortcuts.filter((shortcut) =>
      ["⌫", "Tab", "←", "↑", "↓", "→", "Enter"].includes(shortcut.label),
    ),
  );
  let commandShortcuts = $derived(
    activeMobileShortcuts.filter(
      (shortcut) => !["⌫", "Tab", "←", "↑", "↓", "→", "Enter"].includes(shortcut.label),
    ),
  );
  let shortcutBarMinimal = $state(true);
  let primaryCommandShortcuts = $derived(commandShortcuts.slice(0, phoneCompactLayout ? 2 : 3));
  let primaryNavigationShortcuts = $derived(
    navigationShortcuts.filter((shortcut) => ["⌫", "↑", "↓", "Enter"].includes(shortcut.label)),
  );
  let primaryVisibleShortcuts = $derived([...primaryCommandShortcuts, ...primaryNavigationShortcuts]);
  let visibleCommandShortcuts = $derived(commandShortcuts.slice(0, phoneCompactLayout ? 6 : 8));
  let repeatTimeout: ReturnType<typeof setTimeout> | null = null;
  let repeatInterval: ReturnType<typeof setInterval> | null = null;
  let repeatFastTimeout: ReturnType<typeof setTimeout> | null = null;
  let repeatFasterTimeout: ReturnType<typeof setTimeout> | null = null;
  let modifierLockTimeout: ReturnType<typeof setTimeout> | null = null;
  let modifierLongPressed = $state<StickyModifier | null>(null);
  let repeatedShortcutData: string | null = null;

  const navPadLayout = [
    { label: "Tab", area: "tab" },
    { label: "↑", area: "up" },
    { label: "⌫", area: "backspace" },
    { label: "←", area: "left" },
    { label: "↓", area: "down" },
    { label: "→", area: "right" },
    { label: "Enter", area: "enter" },
  ] as const;

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
      clearModifiers();
    }
  }

  function detectMobilePlatform(): MobilePlatform {
    if (typeof navigator === "undefined") return "other";
    const ua = navigator.userAgent.toLowerCase();
    if (/iphone|ipad|ipod/.test(ua)) return "ios";
    if (ua.includes("android")) return "android";
    return "other";
  }

  function selectPtyFromPicker(event: Event) {
    const nextPtyId = (event.currentTarget as HTMLSelectElement).value;
    if (nextPtyId) onSelectPty(nextPtyId);
  }

  function closeMobilePanel() {
    if (!compactLayout) return;
    mobilePanel = null;
    applyMobilePanelState(null);
  }

  async function pasteFromClipboard() {
    if (!hasActivePty()) return;

    try {
      const text = await navigator.clipboard.readText();
      if (!text) {
        onPaneNotice("剪贴板为空");
        return;
      }
      onPaneData(text);
      onPaneNotice("已粘贴剪贴板");
    } catch {
      onPaneNotice("无法读取剪贴板");
    }
  }

  const LOCKED_MODIFIER_USES = 3;

  function pulseMobileFeedback(duration = 8) {
    if (compactLayout && typeof navigator !== "undefined" && "vibrate" in navigator) {
      navigator.vibrate(duration);
    }
  }

  function hasModifier(modifier: StickyModifier) {
    return activeModifiers.includes(modifier);
  }

  function isModifierLocked(modifier: StickyModifier) {
    return lockedModifiers.includes(modifier);
  }

  function modifierLabel(modifier: StickyModifier) {
    const baseLabel = compactLayout
      ? ({ ctrl: "⌃", alt: "⌥", esc: "⎋" } satisfies Record<StickyModifier, string>)[modifier]
      : modifier.toUpperCase();
    if (!isModifierLocked(modifier)) return baseLabel;
    return `${baseLabel}×${modifierUsesRemaining[modifier]}`;
  }

  function activeModifierSummary() {
    return activeModifiers
      .map((modifier) => (isModifierLocked(modifier) ? `${modifier.toUpperCase()}×${modifierUsesRemaining[modifier]}` : modifier.toUpperCase()))
      .join(" + ");
  }

  function clearModifier(modifier: StickyModifier) {
    activeModifiers = activeModifiers.filter((item) => item !== modifier);
    lockedModifiers = lockedModifiers.filter((item) => item !== modifier);
    modifierUsesRemaining = {
      ...modifierUsesRemaining,
      [modifier]: 0,
    };
  }

  function clearModifiers() {
    activeModifiers = [];
    lockedModifiers = [];
    modifierUsesRemaining = { ctrl: 0, alt: 0, esc: 0 };
  }

  function navigationShortcut(label: string) {
    return navigationShortcuts.find((shortcut) => shortcut.label === label) ?? null;
  }

  function normalizeExclusiveModifiers(modifier: StickyModifier) {
    if (modifier !== "alt" && modifier !== "esc") return;
    clearModifier(modifier === "alt" ? "esc" : "alt");
  }

  function armModifier(modifier: StickyModifier) {
    normalizeExclusiveModifiers(modifier);
    activeModifiers = [...activeModifiers.filter((item) => item !== modifier), modifier];
    modifierUsesRemaining = {
      ...modifierUsesRemaining,
      [modifier]: 0,
    };
  }

  function lockModifier(modifier: StickyModifier) {
    normalizeExclusiveModifiers(modifier);
    activeModifiers = [...activeModifiers.filter((item) => item !== modifier), modifier];
    lockedModifiers = [...lockedModifiers.filter((item) => item !== modifier), modifier];
    modifierUsesRemaining = {
      ...modifierUsesRemaining,
      [modifier]: LOCKED_MODIFIER_USES,
    };
    pulseMobileFeedback(16);
  }

  function toggleModifier(modifier: StickyModifier) {
    if (isModifierLocked(modifier)) {
      clearModifier(modifier);
      return;
    }

    if (hasModifier(modifier)) {
      lockModifier(modifier);
      return;
    }

    armModifier(modifier);
  }

  function stopModifierLockTimer() {
    if (modifierLockTimeout) {
      clearTimeout(modifierLockTimeout);
      modifierLockTimeout = null;
    }
  }

  function handleModifierPointerDown(modifier: StickyModifier) {
    if (!compactLayout || !hasActivePty() || busy) return;
    modifierLongPressed = null;
    stopModifierLockTimer();
    modifierLockTimeout = setTimeout(() => {
      modifierLongPressed = modifier;
      lockModifier(modifier);
    }, 360);
  }

  function handleModifierPointerEnd() {
    stopModifierLockTimer();
  }

  function handleModifierClick(modifier: StickyModifier) {
    if (modifierLongPressed === modifier) {
      modifierLongPressed = null;
      stopModifierLockTimer();
      return;
    }

    modifierLongPressed = null;
    stopModifierLockTimer();
    toggleModifier(modifier);
  }

  function ctrlModifiedData(shortcut: MobileShortcutButton) {
    if (/^Ctrl\+/.test(shortcut.label)) return shortcut.data;
    if (shortcut.label === "Enter") return "\r";
    if (shortcut.label === "Tab") return "\t";
    if (shortcut.label === "⌫") return "\b";
    if (shortcut.label === "Space") return "\u0000";

    const source = /^[a-z]$/i.test(shortcut.label)
      ? shortcut.label
      : /^[a-z]$/i.test(shortcut.data)
        ? shortcut.data
        : null;

    if (!source) return null;
    return String.fromCharCode(source.toUpperCase().charCodeAt(0) - 64);
  }

  function resolveShortcutData(shortcut: MobileShortcutButton) {
    let data = shortcut.data;

    if (hasModifier("ctrl")) {
      const ctrlData = ctrlModifiedData(shortcut);
      if (!ctrlData) {
        onPaneNotice(`当前快捷键不支持 Ctrl + ${shortcut.label}`);
        return null;
      }
      data = ctrlData;
    }

    if (hasModifier("alt")) {
      data = `\u001b${data}`;
    }

    if (hasModifier("esc")) {
      data = `\u001b${data}`;
    }

    return data;
  }

  function consumeModifiersAfterShortcut() {
    let nextActive = activeModifiers.filter((modifier) => lockedModifiers.includes(modifier));
    let nextLocked = [...lockedModifiers];
    const nextRemainingUses = { ...modifierUsesRemaining };

    for (const modifier of [...nextLocked]) {
      nextRemainingUses[modifier] = Math.max(0, nextRemainingUses[modifier] - 1);
      if (nextRemainingUses[modifier] === 0) {
        nextLocked = nextLocked.filter((item) => item !== modifier);
        nextActive = nextActive.filter((item) => item !== modifier);
      }
    }

    activeModifiers = nextActive;
    lockedModifiers = nextLocked;
    modifierUsesRemaining = nextRemainingUses;
  }

  function sendShortcut(data: string, feedback = true) {
    if (feedback) pulseMobileFeedback();
    onSendShortcut(data);
  }

  function restartShortcutRepeat(data: string, intervalMs: number) {
    if (repeatInterval) clearInterval(repeatInterval);
    repeatInterval = setInterval(() => sendShortcut(data, false), intervalMs);
  }

  function stopShortcutRepeat() {
    if (repeatTimeout) {
      clearTimeout(repeatTimeout);
      repeatTimeout = null;
    }
    if (repeatInterval) {
      clearInterval(repeatInterval);
      repeatInterval = null;
    }
    if (repeatFastTimeout) {
      clearTimeout(repeatFastTimeout);
      repeatFastTimeout = null;
    }
    if (repeatFasterTimeout) {
      clearTimeout(repeatFasterTimeout);
      repeatFasterTimeout = null;
    }
  }

  function handleShortcutPointerDown(shortcut: MobileShortcutButton) {
    if (activeModifiers.length > 0) return;
    if (!shortcut.repeatable || !compactLayout || !hasActivePty() || busy) return;

    repeatedShortcutData = null;
    stopShortcutRepeat();
    repeatTimeout = setTimeout(() => {
      repeatedShortcutData = shortcut.data;
      pulseMobileFeedback(12);
      sendShortcut(shortcut.data, false);
      restartShortcutRepeat(shortcut.data, 90);
      repeatFastTimeout = setTimeout(() => restartShortcutRepeat(shortcut.data, 55), 700);
      repeatFasterTimeout = setTimeout(() => restartShortcutRepeat(shortcut.data, 32), 1600);
    }, 320);
  }

  function handleShortcutPointerEnd() {
    stopShortcutRepeat();
  }

  function handleShortcutClick(shortcut: MobileShortcutButton) {
    if (repeatedShortcutData === shortcut.data) {
      repeatedShortcutData = null;
      return;
    }
    repeatedShortcutData = null;
    const resolved = resolveShortcutData(shortcut);
    if (!resolved) return;
    sendShortcut(resolved);
    consumeModifiersAfterShortcut();
  }

  function mobilePanelTitle(panel: MobilePanel | null) {
    switch (panel) {
      case "search":
        return "搜索终端";
      case "display":
        return "显示设置";
      case "shortcuts":
        return "完整快捷键";
      default:
        return "";
    }
  }

  function mobilePlatformLabel(platform: MobilePlatform) {
    switch (platform) {
      case "ios":
        return "iPhone / iPad";
      case "android":
        return "Android";
      default:
        return "Mobile";
    }
  }

  function mobileShortcutHint(mode: SessionMode) {
    switch (mode) {
      case "shell":
        return "长按修饰键锁定 · 可切换极简 / 展开";
      case "vim":
        return "长按修饰键锁定 · 可切换极简 / 展开";
      case "claude":
        return "长按修饰键锁定 · 可切换极简 / 展开";
      case "pager":
        return "长按修饰键锁定 · 可切换极简 / 展开";
      case "repl":
        return "长按修饰键锁定 · 可切换极简 / 展开";
      case "monitor":
        return "长按修饰键锁定 · 可切换极简 / 展开";
      default:
        return "长按修饰键锁定 · 按住方向键、⌫、Enter 可连发";
    }
  }

  function compactShortcutLabel(label: string) {
    switch (label) {
      case "Ctrl+C":
        return "⌃C";
      case "Ctrl+L":
        return "⌃L";
      case "Ctrl+A":
        return "⌃A";
      case "Ctrl+E":
        return "⌃E";
      case "Ctrl+W":
        return "⌃W";
      case "Ctrl+U":
        return "⌃U";
      case "Ctrl+D":
        return "⌃D";
      case "PgUp":
        return "Pg↑";
      case "PgDn":
        return "Pg↓";
      case "Space":
        return "␣";
      case "/help":
        return "/h";
      case "/clear":
        return "/cl";
      case "/compact":
        return "/cp";
      default:
        return label;
    }
  }

  function compactActionLabel(label: "paste" | "keyboard" | "more" | "toggle") {
    if (!phoneCompactLayout) {
      switch (label) {
        case "paste":
          return "Paste";
        case "keyboard":
          return "Keys";
        case "more":
          return "More";
        case "toggle":
          return shortcutBarMinimal ? "Expand" : "Less";
      }
    }

    switch (label) {
      case "paste":
        return "Paste";
      case "keyboard":
        return "Keys";
      case "more":
        return "More";
      case "toggle":
        return shortcutBarMinimal ? "More" : "Less";
    }
  }

  onMount(() => {
    mobilePlatform = detectMobilePlatform();
    syncLayoutState();
    const handleResize = () => syncLayoutState();
    window.addEventListener("resize", handleResize);
    window.visualViewport?.addEventListener("resize", handleResize);

    return () => {
      stopShortcutRepeat();
      stopModifierLockTimer();
      window.removeEventListener("resize", handleResize);
      window.visualViewport?.removeEventListener("resize", handleResize);
    };
  });
</script>

<section class:phone-compact={phoneCompactLayout} class="workspace-shell">
  <header class="workspace-header">
    <div class="workspace-copy">
      {#if !compactLayout}
        <span class="eyebrow">Session workspace</span>
      {/if}
      <h1>{agentName || "Remote session"}</h1>
      <div class="workspace-header-meta">
        <p>{status}</p>
        <span class="mode-chip">Mode: {sessionModeLabel(activeMode)}</span>
        {#if compactLayout}
          <div class="status-pill compact-status-pill">{phaseLabel(sessionPhase)}</div>
        {/if}
      </div>
    </div>

    <div class="header-actions">
      {#if !compactLayout}
        <div class="status-pill">{phaseLabel(sessionPhase)}</div>
      {/if}
      <button class="toolbar-button" onclick={onDisconnect} disabled={busy}>{compactLayout ? "断开" : "Disconnect"}</button>
    </div>
  </header>

  <section class="terminal-card">
    <div class:panel-card={compactLayout} class:terminal-toolbar-card={compactLayout} class="terminal-toolbar-shell">
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
            <button class="toolbar-button" onclick={onCloseActivePty} disabled={!hasActivePty() || busy}>
              Close PTY
            </button>
            <button class="toolbar-button" onclick={onFocusActivePty} disabled={!hasActivePty()}>
              Focus
            </button>
          </div>
        {/if}
      </div>
    </div>

    {#if compactLayout && ptys.length > 0}
      <div class="panel-card mobile-session-bar">
        <div class="mobile-session-inline">
          <label class="mobile-session-picker session-label" for="mobile-pty-select">会话</label>
          <select id="mobile-pty-select" class="mobile-session-select" onchange={selectPtyFromPicker} value={activePtyId ?? ""}>
            {#each ptys as pty}
              <option value={pty.pty_id}>{pty.pty_id} · {pty.shell}{pty.exited ? " · exited" : ""}</option>
            {/each}
          </select>

          <button class="toolbar-button mobile-session-action mobile-session-add" onclick={onCreatePty} disabled={busy}>
            +PTY
          </button>
        </div>

        <div class="mobile-session-actions">
          <button class="toolbar-button mobile-session-action" onclick={onCopyActiveTerminal} disabled={!hasActivePty()}>
            复制
          </button>
          <button class="toolbar-button mobile-session-action" onclick={() => void pasteFromClipboard()} disabled={!hasActivePty() || busy}>
            粘贴
          </button>
          <button class="toolbar-button mobile-session-action" onclick={onFocusActivePty} disabled={!hasActivePty()}>
            键盘
          </button>
          <button class="toolbar-button mobile-session-action" onclick={onCloseActivePty} disabled={!hasActivePty() || busy}>
            关闭
          </button>
        </div>
      </div>
    {/if}

    {#if !compactLayout && showSearch}
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

    {#if !compactLayout && showDisplay}
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

    {#if !compactLayout && showShortcuts}
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
                    onclick={() => sendShortcut(shortcut.data)}
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
      {#if !compactLayout}
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
      {/if}

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

    {#if compactLayout}
      <div
        class:platform-android={mobilePlatform === "android"}
        class:platform-ios={mobilePlatform === "ios"}
        class="quick-shortcuts"
        aria-label="mobile terminal shortcuts"
      >
        <div class="quick-shortcuts-header">
          <div class="quick-shortcuts-copy">
            <strong>{sessionModeLabel(activeMode)}</strong>
            <small>{mobilePlatformLabel(mobilePlatform)} · {mobileShortcutHint(activeMode)}</small>
          </div>

          {#if !shortcutBarMinimal}
            <div class="quick-shortcuts-actions">
              {#if mobilePlatform === "android"}
                <button class="quick-shortcut-action" onclick={() => void pasteFromClipboard()} disabled={!hasActivePty() || busy}>
                  {compactActionLabel("paste")}
                </button>
                <button class="quick-shortcut-action" onclick={onFocusActivePty} disabled={!hasActivePty()}>
                  {compactActionLabel("keyboard")}
                </button>
              {:else}
                <button class="quick-shortcut-action" onclick={onFocusActivePty} disabled={!hasActivePty()}>
                  {compactActionLabel("keyboard")}
                </button>
                <button class="quick-shortcut-action" onclick={() => void pasteFromClipboard()} disabled={!hasActivePty() || busy}>
                  {compactActionLabel("paste")}
                </button>
              {/if}
              <button class="quick-shortcut-action" onclick={() => (shortcutBarMinimal = !shortcutBarMinimal)}>
                {compactActionLabel("toggle")}
              </button>
              <button class="quick-shortcut-action more-shortcut" onclick={() => togglePanel("shortcuts")}>
                {compactActionLabel("more")}
              </button>
            </div>
          {/if}
        </div>

        {#if shortcutBarMinimal}
          <div class="compact-shortcut-row" aria-label="primary mobile shortcuts">
            {#if mobilePlatform === "android"}
              <button class="quick-shortcut-action" onclick={() => void pasteFromClipboard()} disabled={!hasActivePty() || busy}>
                {compactActionLabel("paste")}
              </button>
              <button class="quick-shortcut-action" onclick={onFocusActivePty} disabled={!hasActivePty()}>
                {compactActionLabel("keyboard")}
              </button>
            {:else}
              <button class="quick-shortcut-action" onclick={onFocusActivePty} disabled={!hasActivePty()}>
                {compactActionLabel("keyboard")}
              </button>
              <button class="quick-shortcut-action" onclick={() => void pasteFromClipboard()} disabled={!hasActivePty() || busy}>
                {compactActionLabel("paste")}
              </button>
            {/if}
            <button class="quick-shortcut-action" onclick={() => (shortcutBarMinimal = !shortcutBarMinimal)}>
              {compactActionLabel("toggle")}
            </button>
            <button class="quick-shortcut-action more-shortcut" onclick={() => togglePanel("shortcuts")}>
              {compactActionLabel("more")}
            </button>
            {#each primaryVisibleShortcuts as shortcut}
              <button
                class:repeatable-shortcut={shortcut.repeatable}
                class="quick-shortcut"
                onclick={() => handleShortcutClick(shortcut)}
                onpointerdown={() => handleShortcutPointerDown(shortcut)}
                onpointerup={handleShortcutPointerEnd}
                onpointercancel={handleShortcutPointerEnd}
                onpointerleave={handleShortcutPointerEnd}
                disabled={!hasActivePty() || busy}
              >
                {compactShortcutLabel(shortcut.label)}
              </button>
            {/each}
          </div>
        {:else}
          <div class="modifier-strip" aria-label="sticky terminal modifiers">
            <button
              class:modifier-active={hasModifier("ctrl")}
              class:modifier-locked={isModifierLocked("ctrl")}
              class="modifier-chip"
              onpointerdown={() => handleModifierPointerDown("ctrl")}
              onpointerup={handleModifierPointerEnd}
              onpointercancel={handleModifierPointerEnd}
              onpointerleave={handleModifierPointerEnd}
              onclick={() => handleModifierClick("ctrl")}
            >
              {modifierLabel("ctrl")}
            </button>
            <button
              class:modifier-active={hasModifier("alt")}
              class:modifier-locked={isModifierLocked("alt")}
              class="modifier-chip"
              onpointerdown={() => handleModifierPointerDown("alt")}
              onpointerup={handleModifierPointerEnd}
              onpointercancel={handleModifierPointerEnd}
              onpointerleave={handleModifierPointerEnd}
              onclick={() => handleModifierClick("alt")}
            >
              {modifierLabel("alt")}
            </button>
            <button
              class:modifier-active={hasModifier("esc")}
              class:modifier-locked={isModifierLocked("esc")}
              class="modifier-chip"
              onpointerdown={() => handleModifierPointerDown("esc")}
              onpointerup={handleModifierPointerEnd}
              onpointercancel={handleModifierPointerEnd}
              onpointerleave={handleModifierPointerEnd}
              onclick={() => handleModifierClick("esc")}
            >
              {modifierLabel("esc")}
            </button>
            {#if activeModifiers.length > 0}
              <span class="modifier-status">修饰键：{activeModifierSummary()}</span>
            {/if}
          </div>

          <div class="shortcut-groups">
            <div class="shortcut-group">
              <div class="shortcut-group-title">扩展命令</div>
              <div class="command-strip" aria-label="command shortcuts">
                {#each visibleCommandShortcuts as shortcut}
                  <button
                    class:repeatable-shortcut={shortcut.repeatable}
                    class="quick-shortcut"
                    onclick={() => handleShortcutClick(shortcut)}
                    onpointerdown={() => handleShortcutPointerDown(shortcut)}
                    onpointerup={handleShortcutPointerEnd}
                    onpointercancel={handleShortcutPointerEnd}
                    onpointerleave={handleShortcutPointerEnd}
                    disabled={!hasActivePty() || busy}
                  >
                    {compactShortcutLabel(shortcut.label)}
                  </button>
                {/each}
              </div>
            </div>

            <div class="shortcut-group">
              <div class="shortcut-group-title">导航 / 输入</div>
              <div class="nav-cluster" aria-label="navigation shortcuts">
                {#each navPadLayout as item}
                  {@const shortcut = navigationShortcut(item.label)}
                  <button
                    class:nav-placeholder={!shortcut}
                    class:repeatable-shortcut={shortcut?.repeatable}
                    class="quick-shortcut nav-shortcut"
                    style={`grid-area:${item.area}`}
                    onclick={() => shortcut && handleShortcutClick(shortcut)}
                    onpointerdown={() => shortcut && handleShortcutPointerDown(shortcut)}
                    onpointerup={handleShortcutPointerEnd}
                    onpointercancel={handleShortcutPointerEnd}
                    onpointerleave={handleShortcutPointerEnd}
                    disabled={!shortcut || !hasActivePty() || busy}
                  >
                    {shortcut?.label ?? item.label}
                  </button>
                {/each}
              </div>
            </div>
          </div>
        {/if}
      </div>

      {#if mobilePanel}
        <button class="mobile-sheet-backdrop" aria-label="关闭移动面板" onclick={closeMobilePanel}></button>
        <div class="mobile-sheet" role="dialog" aria-modal="true" aria-label={mobilePanelTitle(mobilePanel)}>
          <div class="mobile-sheet-handle"></div>
          <div class="mobile-sheet-header">
            <div class="mobile-sheet-copy">
              <strong>{mobilePanelTitle(mobilePanel)}</strong>
              <small>{sessionModeLabel(activeMode)} · 针对触摸操作优化</small>
            </div>
            <button class="quick-shortcut-action" onclick={closeMobilePanel}>完成</button>
          </div>

          {#if showSearch}
            <div class="mobile-sheet-panel search-panel">
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
          {:else if showDisplay}
            <div class="mobile-sheet-panel display-panel">
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
          {:else if showShortcuts}
            <div class="mobile-sheet-panel shortcut-sections compact-shortcut-sections">
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
                          onclick={() => sendShortcut(shortcut.data)}
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
        </div>
      {/if}
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

  .workspace-header-meta {
    display: grid;
    gap: 0.35rem;
    margin-top: 0.35rem;
  }

  .workspace-copy p {
    margin: 0;
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
  input,
  select {
    border: 1px solid rgba(148, 163, 184, 0.24);
    border-radius: 0.9rem;
    background: rgba(15, 23, 42, 0.92);
    color: inherit;
    font: inherit;
  }

  button,
  select {
    min-height: 2.9rem;
    touch-action: manipulation;
  }

  button {
    padding: 0.75rem 0.9rem;
    font-weight: 600;
  }

  button:disabled {
    opacity: 0.5;
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

  .terminal-toolbar-shell {
    display: grid;
    gap: 0.75rem;
  }

  .terminal-toolbar-card {
    padding: 0.62rem;
    gap: 0.62rem;
  }

  .quick-shortcuts {
    display: grid;
    gap: 0.75rem;
    position: sticky;
    bottom: calc(env(safe-area-inset-bottom) + 0.5rem);
    z-index: 10;
    padding: 0.75rem;
    border: 1px solid rgba(148, 163, 184, 0.22);
    border-radius: 1rem;
    background: rgba(2, 6, 23, 0.94);
    box-shadow: 0 12px 32px rgba(15, 23, 42, 0.45);
    backdrop-filter: blur(18px);
  }

  .quick-shortcuts-header,
  .quick-shortcuts-actions,
  .mobile-sheet-header,
  .modifier-strip {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .quick-shortcuts-header,
  .mobile-sheet-header {
    justify-content: space-between;
  }

  .quick-shortcuts-copy,
  .mobile-sheet-copy,
  .shortcut-groups,
  .shortcut-group {
    display: grid;
    gap: 0.15rem;
  }

  .shortcut-groups {
    gap: 0.6rem;
  }

  .shortcut-group {
    gap: 0.35rem;
  }

  .quick-shortcuts-copy strong,
  .mobile-sheet-copy strong {
    font-size: 0.95rem;
  }

  .quick-shortcuts-copy small,
  .mobile-sheet-copy small {
    color: #94a3b8;
    font-size: 0.76rem;
  }

  .quick-shortcuts-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 0.28rem;
  }

  .modifier-strip {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-start;
    gap: 0.28rem;
  }

  .command-strip {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-start;
    gap: 0.28rem;
  }

  .nav-cluster {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-start;
    gap: 0.32rem;
  }

  .quick-shortcut-action,
  .quick-shortcut,
  .modifier-chip {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    line-height: 1.05;
    padding-inline: 0.56rem;
    transition:
      transform 140ms ease,
      border-color 140ms ease,
      background-color 140ms ease;
  }

  .quick-shortcut-action {
    flex: 0 0 auto;
    min-height: 2.16rem;
    border-radius: 0.72rem;
    background: rgba(15, 23, 42, 0.82);
  }

  .nav-shortcut {
    flex: 0 0 auto;
    width: auto;
    min-width: 2.25rem;
  }

  .nav-placeholder {
    opacity: 0.28;
    border-style: dashed;
  }

  .modifier-chip {
    flex: 0 0 auto;
    min-height: 2.08rem;
    border-radius: 0.72rem;
    background: rgba(15, 23, 42, 0.72);
  }

  .modifier-chip.modifier-active {
    border-color: rgba(96, 165, 250, 0.7);
    background: rgba(30, 64, 175, 0.34);
    color: #dbeafe;
  }

  .modifier-chip.modifier-locked {
    box-shadow: inset 0 0 0 1px rgba(147, 197, 253, 0.28);
    background: rgba(37, 99, 235, 0.42);
  }

  .modifier-status {
    width: 100%;
    color: #93c5fd;
    font-size: 0.72rem;
    white-space: normal;
  }

  .quick-shortcut-action:active,
  .quick-shortcut:active,
  .modifier-chip:active {
    transform: scale(0.97);
  }

  .quick-shortcut {
    flex: 0 0 auto;
    min-width: 2.2rem;
    border-radius: 0.72rem;
    background: rgba(15, 23, 42, 0.96);
  }

  .command-strip .quick-shortcut,
  .nav-cluster .quick-shortcut {
    width: auto;
  }

  .platform-ios .quick-shortcut-action,
  .platform-ios .modifier-chip {
    border-radius: 1rem;
  }

  .platform-android .quick-shortcut-action,
  .platform-android .modifier-chip {
    min-height: 2.65rem;
    border-radius: 0.95rem;
  }

  .repeatable-shortcut {
    position: relative;
    border-color: rgba(59, 130, 246, 0.45);
  }

  .platform-ios {
    border-color: rgba(191, 219, 254, 0.28);
    background: linear-gradient(180deg, rgba(15, 23, 42, 0.9), rgba(2, 6, 23, 0.96));
    box-shadow: 0 16px 40px rgba(15, 23, 42, 0.42);
  }

  .platform-android {
    border-color: rgba(71, 85, 105, 0.32);
    background: rgba(2, 6, 23, 0.98);
    box-shadow: 0 10px 28px rgba(15, 23, 42, 0.54);
  }

  .repeatable-shortcut::after {
    content: "";
    position: absolute;
    top: 0.42rem;
    right: 0.42rem;
    width: 0.35rem;
    height: 0.35rem;
    border-radius: 999px;
    background: rgba(96, 165, 250, 0.9);
  }

  .more-shortcut {
    border-color: rgba(59, 130, 246, 0.5);
  }

  .mobile-sheet-backdrop {
    position: fixed;
    inset: 0;
    z-index: 20;
    border: 0;
    border-radius: 0;
    background: rgba(2, 6, 23, 0.52);
  }

  .mobile-sheet {
    position: fixed;
    left: max(0.75rem, env(safe-area-inset-left));
    right: max(0.75rem, env(safe-area-inset-right));
    bottom: max(0.75rem, env(safe-area-inset-bottom));
    z-index: 21;
    display: grid;
    gap: 0.9rem;
    max-height: min(68svh, 34rem);
    overflow: hidden;
    padding: 0.9rem;
    border: 1px solid rgba(148, 163, 184, 0.22);
    border-radius: 1.2rem;
    background: rgba(2, 6, 23, 0.98);
    box-shadow: 0 18px 40px rgba(15, 23, 42, 0.52);
    backdrop-filter: blur(22px);
  }

  .mobile-sheet-handle {
    width: 2.8rem;
    height: 0.3rem;
    margin: 0 auto;
    border-radius: 999px;
    background: rgba(148, 163, 184, 0.45);
  }

  .mobile-sheet-panel {
    display: grid;
    gap: 0.75rem;
    min-height: 0;
    overflow-y: auto;
    padding-right: 0.1rem;
  }

  .compact-shortcut-sections {
    max-height: none;
    padding-right: 0;
  }

  .search-panel {
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
  }

  .search-input {
    width: 100%;
    padding: 0.85rem 0.9rem;
  }

  .mobile-session-bar {
    display: grid;
    gap: 0.75rem;
  }

  .mobile-session-inline {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 0.45rem;
    align-items: center;
  }

  .mobile-session-picker {
    display: inline-flex;
    align-items: center;
    white-space: nowrap;
  }

  .session-label {
    font-size: 0.78rem;
  }

  .mobile-session-select {
    width: 100%;
    min-height: 2.3rem;
    padding: 0.6rem 0.75rem;
  }

  .mobile-session-actions {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.45rem;
  }

  .mobile-session-action {
    min-height: 2.2rem;
    padding: 0.5rem 0.55rem;
    font-size: 0.82rem;
  }

  .mobile-session-add {
    min-width: 4.35rem;
  }

  .panel-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .panel-label,
  .shortcut-section-title,
  .shortcut-group-title {
    color: #cbd5e1;
    font-size: 0.92rem;
  }

  .shortcut-group-title {
    letter-spacing: 0.02em;
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

  .workspace-shell.phone-compact {
    gap: 0.8rem;
    padding-top: calc(0.8rem + env(safe-area-inset-top));
    padding-right: calc(0.8rem + env(safe-area-inset-right));
    padding-bottom: calc(0.8rem + env(safe-area-inset-bottom));
    padding-left: calc(0.8rem + env(safe-area-inset-left));
  }

  .workspace-shell.phone-compact .workspace-header,
  .workspace-shell.phone-compact .terminal-card,
  .workspace-shell.phone-compact .panel-card {
    border-radius: 0.9rem;
  }

  .workspace-shell.phone-compact .workspace-header {
    padding: 0.72rem;
    gap: 0.6rem;
  }

  .workspace-shell.phone-compact .workspace-header-meta {
    gap: 0.22rem;
    margin-top: 0.22rem;
  }

  .workspace-shell.phone-compact .workspace-copy h1,
  .workspace-shell.phone-compact .empty-terminal h2 {
    font-size: 1.1rem;
  }

  .workspace-shell.phone-compact .workspace-copy p,
  .workspace-shell.phone-compact .mode-chip,
  .workspace-shell.phone-compact .status-pill {
    font-size: 0.7rem;
  }

  .workspace-shell.phone-compact .workspace-header .toolbar-button {
    min-height: 2.05rem;
    padding: 0.38rem 0.58rem;
    font-size: 0.76rem;
  }

  .workspace-shell.phone-compact .quick-shortcuts {
    gap: 0.4rem;
    padding: 0.48rem;
    padding-bottom: calc(0.48rem + env(safe-area-inset-bottom));
  }

  .workspace-shell.phone-compact .quick-shortcuts-copy strong,
  .workspace-shell.phone-compact .mobile-sheet-copy strong {
    font-size: 0.78rem;
  }

  .workspace-shell.phone-compact .quick-shortcuts-copy small,
  .workspace-shell.phone-compact .mobile-sheet-copy small,
  .workspace-shell.phone-compact .modifier-status,
  .workspace-shell.phone-compact .shortcut-group-title {
    font-size: 0.64rem;
  }

  .workspace-shell.phone-compact .quick-shortcut-action,
  .workspace-shell.phone-compact .modifier-chip,
  .workspace-shell.phone-compact .quick-shortcut {
    padding-inline: 0.34rem;
    font-size: 0.72rem;
  }

  .workspace-shell.phone-compact .quick-shortcut-action {
    min-height: 1.84rem;
  }

  .workspace-shell.phone-compact .modifier-chip {
    min-height: 1.76rem;
  }

  .workspace-shell.phone-compact .quick-shortcut {
    min-height: 1.86rem;
  }

  .workspace-shell.phone-compact .nav-shortcut {
    min-height: 1.96rem;
  }

  .workspace-shell.phone-compact .quick-shortcuts-actions,
  .workspace-shell.phone-compact .command-strip,
  .workspace-shell.phone-compact .nav-cluster,
  .workspace-shell.phone-compact .modifier-strip {
    gap: 0.16rem;
  }

  .workspace-shell.phone-compact .quick-shortcut-action,
  .workspace-shell.phone-compact .modifier-chip,
  .workspace-shell.phone-compact .quick-shortcut {
    border-radius: 0.56rem;
  }

  .workspace-shell.phone-compact .modifier-status {
    white-space: normal;
  }

  .workspace-shell.phone-compact .terminal-card,
  .workspace-shell.phone-compact .panel-card {
    padding: 0.65rem;
    gap: 0.65rem;
  }

  .workspace-shell.phone-compact .terminal-toolbar-card {
    padding: 0.5rem;
    gap: 0.5rem;
  }

  .workspace-shell.phone-compact .header-actions,
  .workspace-shell.phone-compact .terminal-toolbar,
  .workspace-shell.phone-compact .terminal-toolbar-group,
  .workspace-shell.phone-compact .display-row {
    gap: 0.5rem;
  }

  .workspace-shell.phone-compact .terminal-toolbar-group {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    align-items: stretch;
  }

  .workspace-shell.phone-compact .mobile-session-bar {
    gap: 0.48rem;
  }

  .workspace-shell.phone-compact .mobile-session-inline {
    gap: 0.35rem;
  }

  .workspace-shell.phone-compact .mobile-session-select {
    min-height: 2.1rem;
    padding: 0.48rem 0.65rem;
  }

  .workspace-shell.phone-compact .mobile-session-action {
    min-height: 2.02rem;
    padding: 0.38rem 0.45rem;
    font-size: 0.74rem;
  }

  .workspace-shell.phone-compact .mobile-session-actions {
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.35rem;
  }

  @media (max-width: 899px) {
    .workspace-header {
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto;
      align-items: start;
      gap: 0.65rem;
    }

    .workspace-copy h1,
    .empty-terminal h2 {
      font-size: 1.25rem;
    }

    .workspace-header-meta {
      gap: 0.28rem;
      margin-top: 0.28rem;
    }

    .workspace-copy p {
      font-size: 0.88rem;
    }

    .eyebrow,
    .mode-chip,
    .status-pill {
      font-size: 0.74rem;
    }

    button,
    select {
      min-height: 2.6rem;
      font-size: 0.92rem;
    }

    button {
      padding: 0.62rem 0.78rem;
    }

    .workspace-copy,
    .terminal-toolbar,
    .terminal-toolbar-group,
    .search-panel {
      width: 100%;
    }

    .header-actions {
      width: auto;
      justify-self: end;
      justify-content: flex-end;
      align-self: start;
      gap: 0.45rem;
    }

    .workspace-header .toolbar-button {
      flex: 0 0 auto;
      min-height: 2.2rem;
      padding: 0.46rem 0.68rem;
      font-size: 0.82rem;
    }

    .compact-status-pill {
      width: fit-content;
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
    }

    .terminal-toolbar-card {
      padding: 0.58rem;
      gap: 0.52rem;
    }

    .mobile-session-bar {
      padding: 0.58rem;
      gap: 0.52rem;
    }

    .mobile-session-select {
      min-height: 2.18rem;
      padding: 0.52rem 0.7rem;
      font-size: 0.84rem;
    }

    .mobile-session-actions {
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 0.4rem;
    }

    .mobile-session-action {
      min-height: 2.06rem;
      padding: 0.42rem 0.48rem;
      font-size: 0.78rem;
    }

    .quick-shortcuts {
      margin-top: 0.25rem;
      gap: 0.48rem;
      padding: 0.52rem;
      padding-bottom: calc(0.52rem + env(safe-area-inset-bottom));
      border-radius: 0.84rem;
    }

    .quick-shortcuts-copy strong,
    .mobile-sheet-copy strong {
      font-size: 0.82rem;
    }

    .quick-shortcuts-copy small,
    .mobile-sheet-copy small,
    .modifier-status,
    .shortcut-group-title {
      font-size: 0.68rem;
    }

    .quick-shortcuts-header,
    .mobile-sheet-header {
      flex-direction: column;
      align-items: stretch;
    }

    .quick-shortcuts-actions {
      display: flex;
      flex-wrap: wrap;
      justify-content: flex-start;
      gap: 0.2rem;
    }

    .modifier-strip {
      display: flex;
      flex-wrap: wrap;
      align-items: stretch;
      gap: 0.2rem;
    }

    .quick-shortcut-action,
    .modifier-chip,
    .quick-shortcut {
      padding-inline: 0.32rem;
      font-size: 0.7rem;
    }

    .quick-shortcut-action {
      min-height: 1.78rem;
    }

    .modifier-chip {
      min-height: 1.72rem;
    }

    .quick-shortcut {
      min-height: 1.78rem;
      min-width: 2rem;
    }

    .nav-shortcut {
      min-height: 1.88rem;
      min-width: 2.05rem;
    }

    .nav-cluster {
      display: flex;
      flex-wrap: wrap;
      gap: 0.2rem;
    }

    .modifier-status {
      width: 100%;
      white-space: normal;
    }

    .search-panel {
      grid-template-columns: 1fr;
    }

    .search-input,
    .mobile-session-select {
      padding: 0.72rem 0.82rem;
    }

    .command-strip {
      display: flex;
      flex-wrap: wrap;
      gap: 0.2rem;
    }

    .mobile-sheet {
      gap: 0.75rem;
      padding: 0.8rem;
      border-radius: 1rem;
    }

    .theme-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .shortcut-sections {
      max-height: none;
      padding-bottom: calc(0.35rem + env(safe-area-inset-bottom));
    }

    .terminal-stack {
      height: clamp(16rem, 42dvh, 24rem);
    }
  }
</style>
