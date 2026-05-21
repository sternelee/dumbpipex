<script lang="ts">
  import {
    mobileModeShortcuts,
    prioritizeMobileShortcuts,
    sessionModeLabel,
    mobilePlatformLabel,
    mobileShortcutHint,
  } from "$lib/terminal-ui";
  import type {
    SessionMode,
    MobilePlatform,
    MobileShortcutButton,
    StickyModifier,
  } from "$lib/terminal-ui";

  let {
    compactLayout,
    phoneCompactLayout,
    activeMode,
    mobilePlatform,
    hasActivePty,
    busy,
    onSendShortcut,
    onPaneNotice,
    onPaneData,
    onFocusActivePty,
    onTogglePanel,
  }: {
    compactLayout: boolean;
    phoneCompactLayout: boolean;
    activeMode: SessionMode;
    mobilePlatform: MobilePlatform;
    hasActivePty: boolean;
    busy: boolean;
    onSendShortcut: (data: string) => void;
    onPaneNotice: (message: string) => void;
    onPaneData: (data: string) => void;
    onFocusActivePty: () => void;
    onTogglePanel: () => void;
  } = $props();

  const LOCKED_MODIFIER_USES = 3;

  let shortcutBarMinimal = $state(true);
  let activeModifiers = $state<StickyModifier[]>([]);
  let lockedModifiers = $state<StickyModifier[]>([]);
  let modifierUsesRemaining = $state<Record<StickyModifier, number>>({
    ctrl: 0,
    alt: 0,
    esc: 0,
  });

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

  const activeMobileShortcuts = $derived(
    prioritizeMobileShortcuts(
      mobileModeShortcuts[activeMode] as MobileShortcutButton[],
      mobilePlatform,
    ),
  );

  const navigationShortcuts = $derived(
    activeMobileShortcuts.filter((shortcut) =>
      ["⌫", "Tab", "←", "↑", "↓", "→", "Enter"].includes(shortcut.label),
    ),
  );

  const commandShortcuts = $derived(
    activeMobileShortcuts.filter(
      (shortcut) => !["⌫", "Tab", "←", "↑", "↓", "→", "Enter"].includes(shortcut.label),
    ),
  );

  const primaryCommandShortcuts = $derived(commandShortcuts.slice(0, phoneCompactLayout ? 2 : 3));
  const primaryNavigationShortcuts = $derived(
    navigationShortcuts.filter((shortcut) => ["⌫", "↑", "↓", "Enter"].includes(shortcut.label)),
  );
  const primaryVisibleShortcuts = $derived([...primaryCommandShortcuts, ...primaryNavigationShortcuts]);
  const visibleCommandShortcuts = $derived(commandShortcuts.slice(0, phoneCompactLayout ? 6 : 8));

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
      .map((modifier) =>
        isModifierLocked(modifier)
          ? `${modifier.toUpperCase()}×${modifierUsesRemaining[modifier]}`
          : modifier.toUpperCase(),
      )
      .join(" + ");
  }

  function clearModifier(modifier: StickyModifier) {
    activeModifiers = activeModifiers.filter((item) => item !== modifier);
    lockedModifiers = lockedModifiers.filter((item) => item !== modifier);
    modifierUsesRemaining = { ...modifierUsesRemaining, [modifier]: 0 };
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
    modifierUsesRemaining = { ...modifierUsesRemaining, [modifier]: 0 };
  }

  function lockModifier(modifier: StickyModifier) {
    normalizeExclusiveModifiers(modifier);
    activeModifiers = [...activeModifiers.filter((item) => item !== modifier), modifier];
    lockedModifiers = [...lockedModifiers.filter((item) => item !== modifier), modifier];
    modifierUsesRemaining = { ...modifierUsesRemaining, [modifier]: LOCKED_MODIFIER_USES };
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
    if (!compactLayout || !hasActivePty || busy) return;
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
    if (!shortcut.repeatable || !compactLayout || !hasActivePty || busy) return;

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

  async function pasteFromClipboard() {
    if (!hasActivePty) return;
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
</script>

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
            <button
              class="quick-shortcut-action"
              onclick={() => void pasteFromClipboard()}
              disabled={!hasActivePty || busy}
            >
              {compactActionLabel("paste")}
            </button>
            <button class="quick-shortcut-action" onclick={onFocusActivePty} disabled={!hasActivePty}>
              {compactActionLabel("keyboard")}
            </button>
          {:else}
            <button class="quick-shortcut-action" onclick={onFocusActivePty} disabled={!hasActivePty}>
              {compactActionLabel("keyboard")}
            </button>
            <button
              class="quick-shortcut-action"
              onclick={() => void pasteFromClipboard()}
              disabled={!hasActivePty || busy}
            >
              {compactActionLabel("paste")}
            </button>
          {/if}
          <button
            class="quick-shortcut-action"
            onclick={() => (shortcutBarMinimal = !shortcutBarMinimal)}
          >
            {compactActionLabel("toggle")}
          </button>
          <button class="quick-shortcut-action more-shortcut" onclick={onTogglePanel}>
            {compactActionLabel("more")}
          </button>
        </div>
      {/if}
    </div>

    {#if shortcutBarMinimal}
      <div class="compact-shortcut-row" aria-label="primary mobile shortcuts">
        {#if mobilePlatform === "android"}
          <button
            class="quick-shortcut-action"
            onclick={() => void pasteFromClipboard()}
            disabled={!hasActivePty || busy}
          >
            {compactActionLabel("paste")}
          </button>
          <button class="quick-shortcut-action" onclick={onFocusActivePty} disabled={!hasActivePty}>
            {compactActionLabel("keyboard")}
          </button>
        {:else}
          <button class="quick-shortcut-action" onclick={onFocusActivePty} disabled={!hasActivePty}>
            {compactActionLabel("keyboard")}
          </button>
          <button
            class="quick-shortcut-action"
            onclick={() => void pasteFromClipboard()}
            disabled={!hasActivePty || busy}
          >
            {compactActionLabel("paste")}
          </button>
        {/if}
        <button
          class="quick-shortcut-action"
          onclick={() => (shortcutBarMinimal = !shortcutBarMinimal)}
        >
          {compactActionLabel("toggle")}
        </button>
        <button class="quick-shortcut-action more-shortcut" onclick={onTogglePanel}>
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
            disabled={!hasActivePty || busy}
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
                disabled={!hasActivePty || busy}
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
                disabled={!shortcut || !hasActivePty || busy}
              >
                {shortcut?.label ?? item.label}
              </button>
            {/each}
          </div>
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
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

  .quick-shortcuts-header {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    justify-content: space-between;
  }

  .quick-shortcuts-copy {
    display: grid;
    gap: 0.15rem;
  }

  .quick-shortcuts-copy strong {
    font-size: 0.95rem;
  }

  .quick-shortcuts-copy small {
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
    align-items: center;
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

  .shortcut-groups {
    display: grid;
    gap: 0.6rem;
  }

  .shortcut-group {
    display: grid;
    gap: 0.35rem;
  }

  .shortcut-group-title {
    color: #cbd5e1;
    font-size: 0.92rem;
    letter-spacing: 0.02em;
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
    border: 1px solid rgba(148, 163, 184, 0.24);
    border-radius: 0.72rem;
    background: rgba(15, 23, 42, 0.82);
    color: inherit;
    font: inherit;
    touch-action: manipulation;
    font-weight: 600;
  }

  .quick-shortcut-action {
    flex: 0 0 auto;
    min-height: 2.16rem;
  }

  .quick-shortcut {
    flex: 0 0 auto;
    min-width: 2.2rem;
    min-height: 2.16rem;
    background: rgba(15, 23, 42, 0.96);
  }

  .command-strip .quick-shortcut,
  .nav-cluster .quick-shortcut {
    width: auto;
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

  .platform-ios .quick-shortcut-action,
  .platform-ios .modifier-chip {
    border-radius: 1rem;
  }

  .platform-android .quick-shortcut-action,
  .platform-android .modifier-chip {
    min-height: 2.65rem;
    border-radius: 0.95rem;
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

  .repeatable-shortcut {
    position: relative;
    border-color: rgba(59, 130, 246, 0.45);
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

  .compact-shortcut-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.28rem;
    align-items: center;
  }

  @media (max-width: 899px) {
    .quick-shortcuts {
      margin-top: 0.25rem;
      gap: 0.48rem;
      padding: 0.52rem;
      padding-bottom: calc(0.52rem + env(safe-area-inset-bottom));
      border-radius: 0.84rem;
    }

    .quick-shortcuts-copy strong {
      font-size: 0.82rem;
    }

    .quick-shortcuts-copy small,
    .modifier-status,
    .shortcut-group-title {
      font-size: 0.68rem;
    }

    .quick-shortcuts-header {
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

    .command-strip {
      display: flex;
      flex-wrap: wrap;
      gap: 0.2rem;
    }
  }
</style>
