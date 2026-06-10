<script lang="ts">
  import {
    mobileModeShortcuts,
    prioritizeMobileShortcuts,
    sessionModeLabel,
    mobileShortcutHint,
    specialCharGrid,
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
  } = $props();

  const LOCKED_MODIFIER_USES = 3;

  let shortcutBarMinimal = $state(true);
  let activeModifiers = $state<StickyModifier[]>([]);
  let lockedModifiers = $state<StickyModifier[]>([]);
  let modifierUsesRemaining = $state<Record<StickyModifier, number>>({
    ctrl: 0,
    alt: 0,
    esc: 0,
    shift: 0,
  });

  let repeatTimeout: ReturnType<typeof setTimeout> | null = null;
  let repeatInterval: ReturnType<typeof setInterval> | null = null;
  let modifierLockTimeout: ReturnType<typeof setTimeout> | null = null;
  let modifierLongPressed = $state<StickyModifier | null>(null);
  let repeatedShortcutData: string | null = null;

  const activeMobileShortcuts = $derived(
    prioritizeMobileShortcuts(
      mobileModeShortcuts[activeMode] as MobileShortcutButton[],
      mobilePlatform,
    ),
  );

  const SHIFT_MAP: Record<string, string> = {
    "1": "!", "2": "@", "3": "#", "4": "$", "5": "%",
    "6": "^", "7": "&", "8": "*", "9": "(", "0": ")",
    "-": "_", "=": "+", "[": "{", "]": "}", "\\": "|",
    ";": ":", "'": '"', ",": "<", ".": ">", "/": "?",
    "`": "~",
  };

  function shiftModified(data: string): string {
    if (data.length !== 1) return data;
    return SHIFT_MAP[data] ?? data.toUpperCase();
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

    if (hasModifier("shift")) {
      data = shiftModified(data);
    }

    if (hasModifier("alt")) {
      data = `\u001b${data}`;
    }

    if (hasModifier("esc")) {
      data = `\u001b${data}`;
    }

    return data;
  }

  /* ── modifier helpers (unchanged from original) ── */
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
    const labels: Record<StickyModifier, string> = { ctrl: "Ctrl", alt: "Alt", esc: "Esc", shift: "Shift" };
    const base = compactLayout ? ({ ctrl: "⌃", alt: "⌥", esc: "⎋", shift: "⇧" } as const)[modifier] : labels[modifier];
    if (!isModifierLocked(modifier)) return base;
    return `${base}×${modifierUsesRemaining[modifier]}`;
  }

  function activeModifierSummary() {
    return activeModifiers
      .map((m) => isModifierLocked(m) ? ` ${modifierLabel(m)}` : ` ${modifierLabel(m)}`)
      .join(" + ").trim();
  }

  function clearModifier(modifier: StickyModifier) {
    activeModifiers = activeModifiers.filter((item) => item !== modifier);
    lockedModifiers = lockedModifiers.filter((item) => item !== modifier);
    modifierUsesRemaining = { ...modifierUsesRemaining, [modifier]: 0 };
  }

  function clearModifiers() {
    activeModifiers = [];
    lockedModifiers = [];
    modifierUsesRemaining = { ctrl: 0, alt: 0, esc: 0, shift: 0 };
  }

  function lockModifier(modifier: StickyModifier) {
    activeModifiers = [...activeModifiers.filter((item) => item !== modifier), modifier];
    lockedModifiers = [...lockedModifiers.filter((item) => item !== modifier), modifier];
    modifierUsesRemaining = { ...modifierUsesRemaining, [modifier]: LOCKED_MODIFIER_USES };
    pulseMobileFeedback(16);
  }

  function armorSingle(modifier: StickyModifier) {
    if (modifier === "alt" || modifier === "esc") {
      clearModifier(modifier === "alt" ? "esc" : "alt");
    }
    activeModifiers = [...activeModifiers.filter((item) => item !== modifier), modifier];
    modifierUsesRemaining = { ...modifierUsesRemaining, [modifier]: 0 };
  }

  function toggleModifier(modifier: StickyModifier) {
    if (isModifierLocked(modifier)) { clearModifier(modifier); return; }
    if (hasModifier(modifier)) { lockModifier(modifier); return; }
    armorSingle(modifier);
  }

  function stopModifierLockTimer() {
    if (modifierLockTimeout) { clearTimeout(modifierLockTimeout); modifierLockTimeout = null; }
  }

  function handleModifierPointerDown(modifier: StickyModifier) {
    if (!hasActivePty || busy) return;
    modifierLongPressed = null;
    stopModifierLockTimer();
    modifierLockTimeout = setTimeout(() => {
      modifierLongPressed = modifier;
      lockModifier(modifier);
    }, 360);
  }

  function handleModifierPointerEnd() { stopModifierLockTimer(); }

  function handleModifierClick(modifier: StickyModifier) {
    if (!hasActivePty || busy) return;
    if (modifierLongPressed === modifier) { modifierLongPressed = null; stopModifierLockTimer(); return; }
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
      ? shortcut.label : /^[a-z]$/i.test(shortcut.data) ? shortcut.data : null;
    if (!source) return null;
    return String.fromCharCode(source.toUpperCase().charCodeAt(0) - 64);
  }

  function consumeModifiersAfterShortcut() {
    let nextActive = activeModifiers.filter((m) => lockedModifiers.includes(m));
    let nextLocked = [...lockedModifiers];
    const nextUses = { ...modifierUsesRemaining };
    for (const m of [...nextLocked]) {
      nextUses[m] = Math.max(0, nextUses[m] - 1);
      if (nextUses[m] === 0) {
        nextLocked = nextLocked.filter((x) => x !== m);
        nextActive = nextActive.filter((x) => x !== m);
      }
    }
    activeModifiers = nextActive;
    lockedModifiers = nextLocked;
    modifierUsesRemaining = nextUses;
  }

  function sendShortcut(data: string, feedback = true) {
    if (feedback) pulseMobileFeedback();
    onSendShortcut(data);
  }

  function stopShortcutRepeat() {
    if (repeatTimeout) { clearTimeout(repeatTimeout); repeatTimeout = null; }
    if (repeatInterval) { clearInterval(repeatInterval); repeatInterval = null; }
  }

  function startShortcutRepeat(shortcut: MobileShortcutButton) {
    if (activeModifiers.length > 0) return;
    if (!shortcut.repeatable || !hasActivePty || busy) return;
    repeatedShortcutData = null;
    stopShortcutRepeat();
    repeatTimeout = setTimeout(() => {
      repeatedShortcutData = shortcut.data;
      pulseMobileFeedback(12);
      sendShortcut(shortcut.data, false);
      repeatInterval = setInterval(() => sendShortcut(shortcut.data, false), 90);
      setTimeout(() => { if (repeatInterval) { clearInterval(repeatInterval); repeatInterval = setInterval(() => sendShortcut(shortcut.data, false), 55); } }, 700);
    }, 320);
  }

  function handleShortcutClick(shortcut: MobileShortcutButton) {
    if (repeatedShortcutData === shortcut.data) { repeatedShortcutData = null; return; }
    repeatedShortcutData = null;
    const resolved = resolveShortcutData(shortcut);
    if (!resolved) return;
    sendShortcut(resolved);
    consumeModifiersAfterShortcut();
  }

  function compactShortcutLabel(label: string) {
    switch (label) {
      case "Ctrl+C": return "⌃C"; case "Ctrl+L": return "⌃L";
      case "Ctrl+A": return "⌃A"; case "Ctrl+E": return "⌃E";
      case "Ctrl+W": return "⌃W"; case "Ctrl+U": return "⌃U";
      case "Ctrl+D": return "⌃D";
      case "PgUp": return "Pg↑"; case "PgDn": return "Pg↓";
      case "Space": return "␣";
      case "/help": return "/h"; case "/clear": return "/cl"; case "/compact": return "/cp";
      default: return label;
    }
  }

  async function pasteFromClipboard() {
    if (!hasActivePty) return;
    try {
      const text = await navigator.clipboard.readText();
      if (!text) { onPaneNotice("剪贴板为空"); return; }
      onPaneData(text);
      onPaneNotice("已粘贴剪贴板");
    } catch { onPaneNotice("无法读取剪贴板"); }
  }
</script>

{#if compactLayout}
<div class="shortcuts-bar" aria-label="mobile terminal shortcuts">
  <!-- Modifier strip — always visible -->
  <div class="modifier-strip">
    {#each (["ctrl", "alt", "esc", "shift"] as const) as mod}
      <button
        class:modifier-active={hasModifier(mod)}
        class:modifier-locked={isModifierLocked(mod)}
        class="modifier-chip"
        onpointerdown={() => handleModifierPointerDown(mod)}
        onpointerup={handleModifierPointerEnd}
        onpointercancel={handleModifierPointerEnd}
        onpointerleave={handleModifierPointerEnd}
        onclick={() => handleModifierClick(mod)}
        disabled={!hasActivePty || busy}
      >
        {modifierLabel(mod)}
      </button>
    {/each}
    <button class="quick-btn paste-btn" onclick={() => void pasteFromClipboard()} disabled={!hasActivePty || busy}>Paste</button>
    <button class="quick-btn" onclick={onFocusActivePty} disabled={!hasActivePty}>Focus</button>
    <button class="quick-btn toggle-btn" onclick={() => (shortcutBarMinimal = !shortcutBarMinimal)}>
      {shortcutBarMinimal ? "⌨" : "✕"}
    </button>
  </div>

  {#if !shortcutBarMinimal}
    <div class="shortcuts-expanded">
      <!-- Special characters -->
      <div class="special-char-grid">
        {#each specialCharGrid as char}
          <button
            class="quick-btn char-btn"
            onclick={() => handleShortcutClick(char)}
            disabled={!hasActivePty || busy}
          >{char.label}</button>
        {/each}
      </div>

      <!-- Command shortcuts -->
      <div class="command-strip">
        {#each activeMobileShortcuts.filter(s => !["⌫","Tab","←","↑","↓","→","Enter"].includes(s.label)).slice(0, 8) as shortcut}
          <button
            class:repeatable-shortcut={shortcut.repeatable}
            class="quick-btn cmd-btn"
            onclick={() => handleShortcutClick(shortcut)}
            onpointerdown={() => startShortcutRepeat(shortcut)}
            onpointerup={stopShortcutRepeat}
            onpointercancel={stopShortcutRepeat}
            onpointerleave={stopShortcutRepeat}
            disabled={!hasActivePty || busy}
          >{compactShortcutLabel(shortcut.label)}</button>
        {/each}
      </div>

      <!-- Navigation -->
      <div class="nav-strip">
        {#each (activeMobileShortcuts.filter(s => ["Tab","←","↑","↓","→","⌫","Enter"].includes(s.label))) as shortcut}
          <button
            class:repeatable-shortcut={shortcut.repeatable}
            class="quick-btn nav-btn"
            onclick={() => handleShortcutClick(shortcut)}
            onpointerdown={() => startShortcutRepeat(shortcut)}
            onpointerup={stopShortcutRepeat}
            onpointercancel={stopShortcutRepeat}
            onpointerleave={stopShortcutRepeat}
            disabled={!hasActivePty || busy}
          >{shortcut.label}</button>
        {/each}
      </div>
    </div>
  {/if}
</div>
{/if}

<style>
  .shortcuts-bar {
    display: grid;
    gap: 0.35rem;
    padding: 0.35rem 0.4rem;
    padding-bottom: calc(0.35rem + env(safe-area-inset-bottom));
    background: rgba(2, 6, 23, 0.95);
    border-top: 1px solid rgba(148, 163, 184, 0.12);
    backdrop-filter: blur(18px);
    flex-shrink: 0;
  }

  .modifier-strip {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.25rem;
  }

  .quick-btn {
    border: 1px solid rgba(148, 163, 184, 0.2);
    border-radius: 0.5rem;
    background: rgba(15, 23, 42, 0.85);
    color: #e2e8f0;
    font: inherit;
    font-size: 0.75rem;
    font-weight: 600;
    touch-action: manipulation;
    padding: 0.4rem 0.5rem;
    min-height: 2.4rem;
    white-space: nowrap;
    transition: background-color 120ms ease, border-color 120ms ease;
  }

  .quick-btn:disabled { opacity: 0.35; }

  .modifier-chip {
    border: 1px solid rgba(148, 163, 184, 0.25);
    border-radius: 0.5rem;
    background: rgba(15, 23, 42, 0.7);
    color: #cbd5e1;
    font: inherit;
    font-size: 0.72rem;
    font-weight: 700;
    touch-action: manipulation;
    padding: 0.4rem 0.5rem;
    min-height: 2.4rem;
    min-width: 2.6rem;
    text-align: center;
    transition: all 120ms ease;
  }
  .modifier-chip.modifier-active {
    border-color: rgba(96, 165, 250, 0.7);
    background: rgba(30, 64, 175, 0.34);
    color: #dbeafe;
  }
  .modifier-chip.modifier-locked {
    box-shadow: inset 0 0 0 1px rgba(147, 197, 253, 0.35);
    background: rgba(37, 99, 235, 0.42);
  }

  .paste-btn { border-color: rgba(59, 130, 246, 0.35); }
  .toggle-btn { min-width: 2.4rem; min-height: 2.4rem; padding: 0.3rem 0.4rem; font-size: 0.85rem; }

  .shortcuts-expanded {
    display: grid;
    gap: 0.3rem;
  }

  .special-char-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }
  .char-btn {
    min-width: 2.2rem;
    min-height: 2.2rem;
    padding: 0.3rem 0.4rem;
    font-size: 0.72rem;
  }

  .command-strip {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }
  .cmd-btn {
    font-size: 0.7rem;
    padding: 0.3rem 0.45rem;
    min-height: 2.2rem;
  }

  .nav-strip {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }
  .nav-btn {
    font-size: 0.72rem;
    padding: 0.3rem 0.45rem;
    min-width: 2.2rem;
    min-height: 2.2rem;
    text-align: center;
  }

  /* Extra-narrow phones (≤420pt / iPhone SE 375pt): compress every chip
     further so the modifier strip (4 modifiers + Paste + Focus + ⌨) fits
     on a single row, and each strip row stays under 36pt tall to keep the
     overall bar from wrapping into two visible rows. */
  @media (max-width: 420px) {
    .shortcuts-bar {
      gap: 0.25rem;
      padding: 0.25rem 0.3rem;
    }

    .modifier-strip {
      gap: 0.2rem;
    }

    .modifier-chip {
      font-size: 0.66rem;
      padding: 0.3rem 0.4rem;
      min-height: 2rem;
      min-width: 2.2rem;
    }

    .quick-btn {
      font-size: 0.66rem;
      padding: 0.3rem 0.4rem;
      min-height: 2rem;
    }

    .toggle-btn {
      min-width: 2rem;
      min-height: 2rem;
      padding: 0.2rem 0.3rem;
    }

    .char-btn,
    .cmd-btn,
    .nav-btn {
      font-size: 0.65rem;
      padding: 0.25rem 0.35rem;
      min-height: 1.9rem;
      min-width: 1.9rem;
    }
  }

  .repeatable-shortcut {
    border-color: rgba(59, 130, 246, 0.45);
    position: relative;
  }
  .repeatable-shortcut::after {
    content: "";
    position: absolute;
    top: 3px; right: 3px;
    width: 4px; height: 4px;
    border-radius: 50%;
    background: rgba(96, 165, 250, 0.9);
  }
</style>
