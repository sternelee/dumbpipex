<script lang="ts">
  import { tick } from "svelte";
  import type { SessionPhase, TerminalThemeOption } from "$lib/terminal-ui";

  type MenuId = "file" | "view" | null;
  type MenuItem =
    | { kind: "action"; label: string; shortcut?: string; action: () => void; disabled?: boolean }
    | { kind: "active"; label: string }
    | { kind: "separator" };

  let {
    agentName,
    sessionPhase,
    fontSize,
    showSearch,
    compact = false,
    hasActivePty,
    themes,
    themeId,
    onNewTab,
    onCloseTab,
    onSplitRight,
    onSplitLeft,
    splitActive = false,
    onCloseSplit,
    onDisconnect,
    onIncreaseFont,
    onDecreaseFont,
    onResetFont,
    onToggleSearch,
    onSelectTheme,
  }: {
    agentName: string;
    sessionPhase: SessionPhase;
    fontSize: number;
    showSearch: boolean;
    compact?: boolean;
    hasActivePty: boolean;
    themes: TerminalThemeOption[];
    themeId: string;
    onNewTab: () => void;
    onCloseTab: () => void;
    onSplitRight: () => void;
    onSplitLeft: () => void;
    splitActive?: boolean;
    onCloseSplit?: () => void;
    onDisconnect: () => void;
    onIncreaseFont: () => void;
    onDecreaseFont: () => void;
    onResetFont: () => void;
    onToggleSearch: () => void;
    onSelectTheme: (id: string) => void;
  } = $props();

  let openMenu = $state<MenuId>(null);
  let activeIndex = $state(0);
  let menuBarRef = $state<HTMLElement | null>(null);
  let triggerRefs = $state<Partial<Record<NonNullable<MenuId>, HTMLButtonElement>>>({});
  let itemRefs = $state<(HTMLButtonElement | null)[]>([]);

  const MENU_ORDER: NonNullable<MenuId>[] = ["file", "view"];

  function getActionItems(id: MenuId): Extract<MenuItem, { kind: "action" }>[] {
    return getMenuItems(id).filter(
      (i): i is Extract<MenuItem, { kind: "action" }> => i.kind === "action",
    );
  }

  function getMenuItems(id: MenuId): MenuItem[] {
    if (id === "file") {
      const items: MenuItem[] = [
        { kind: "action", label: "New Tab", shortcut: "⌘T", action: onNewTab },
        { kind: "action", label: "Close Tab", shortcut: "⌘W", action: onCloseTab, disabled: !hasActivePty },
        { kind: "separator" },
      ];
      if (splitActive) {
        items.push({ kind: "action", label: "Close Split", action: onCloseSplit! });
      } else {
        items.push({ kind: "action", label: "Split Right", action: onSplitRight });
        items.push({ kind: "action", label: "Split Left", action: onSplitLeft });
      }
      items.push({ kind: "separator" });
      items.push({ kind: "action", label: "Disconnect", action: onDisconnect });
      return items;
    }
    if (id === "view") {
      const items: MenuItem[] = [
        { kind: "active", label: `Font Size: ${fontSize}px` },
        { kind: "action", label: "Increase Font Size", shortcut: "⌘+", action: onIncreaseFont },
        { kind: "action", label: "Decrease Font Size", shortcut: "⌘-", action: onDecreaseFont },
        { kind: "action", label: "Reset Font Size", shortcut: "⌘0", action: onResetFont },
        { kind: "separator" },
        { kind: "active", label: `Theme: ${themes.find((t) => t.id === themeId)?.label ?? themeId}` },
      ];
      for (const theme of themes) {
        items.push({
          kind: "action",
          label: theme.id === themeId ? `  ✓ ${theme.label}` : `    ${theme.label}`,
          action: () => onSelectTheme(theme.id),
        });
      }
      items.push({ kind: "separator" });
      items.push({ kind: "action", label: showSearch ? "Hide Search" : "Search", shortcut: "⌘F", action: onToggleSearch });
      return items;
    }
    return [];
  }

  function phaseDot(phase: SessionPhase): string {
    if (phase === "ready") return "●";
    if (phase === "connecting" || phase === "creating_pty") return "◑";
    if (phase === "disconnecting") return "◌";
    return "○";
  }

  function phaseColor(phase: SessionPhase): string {
    if (phase === "ready") return "var(--menu-dot-ready, #34d399)";
    if (phase === "connecting" || phase === "creating_pty") return "var(--menu-dot-busy, #facc15)";
    if (phase === "disconnecting") return "var(--menu-dot-busy, #f87171)";
    return "var(--menu-dot-idle, #94a3b8)";
  }

  async function openWithFocus(id: NonNullable<MenuId>) {
    openMenu = id;
    activeIndex = 0;
    await tick();
    focusActiveItem();
  }

  function toggleMenu(id: NonNullable<MenuId>) {
    if (openMenu === id) {
      openMenu = null;
    } else {
      void openWithFocus(id);
    }
  }

  function focusActiveItem() {
    const item = itemRefs[activeIndex];
    item?.focus();
  }

  function focusNextItem() {
    const items = getActionItems(openMenu);
    if (items.length === 0) return;
    for (let i = 1; i <= items.length; i++) {
      const next = (activeIndex + i) % items.length;
      const candidate = itemRefs[next];
      if (candidate && !candidate.disabled) {
        activeIndex = next;
        candidate.focus();
        return;
      }
    }
  }

  function focusPrevItem() {
    const items = getActionItems(openMenu);
    if (items.length === 0) return;
    for (let i = 1; i <= items.length; i++) {
      const prev = (activeIndex - i + items.length) % items.length;
      const candidate = itemRefs[prev];
      if (candidate && !candidate.disabled) {
        activeIndex = prev;
        candidate.focus();
        return;
      }
    }
  }

  function focusFirstEnabled() {
    const items = getActionItems(openMenu);
    for (let i = 0; i < items.length; i++) {
      const candidate = itemRefs[i];
      if (candidate && !candidate.disabled) {
        activeIndex = i;
        candidate.focus();
        return;
      }
    }
  }

  function focusLastEnabled() {
    const items = getActionItems(openMenu);
    for (let i = items.length - 1; i >= 0; i--) {
      const candidate = itemRefs[i];
      if (candidate && !candidate.disabled) {
        activeIndex = i;
        candidate.focus();
        return;
      }
    }
  }

  function switchMenu(direction: 1 | -1) {
    if (!openMenu) return;
    const idx = MENU_ORDER.indexOf(openMenu);
    const next = (idx + direction + MENU_ORDER.length) % MENU_ORDER.length;
    const target = MENU_ORDER[next];
    triggerRefs[target]?.focus();
    void openWithFocus(target);
  }

  function closeMenus(returnFocus = false) {
    const wasOpen = openMenu;
    openMenu = null;
    if (returnFocus && wasOpen) {
      triggerRefs[wasOpen]?.focus();
    }
  }

  function handleItemKeydown(event: KeyboardEvent, item: MenuItem) {
    if (item.kind !== "action") return;
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        focusNextItem();
        return;
      case "ArrowUp":
        event.preventDefault();
        focusPrevItem();
        return;
      case "Home":
        event.preventDefault();
        focusFirstEnabled();
        return;
      case "End":
        event.preventDefault();
        focusLastEnabled();
        return;
      case "ArrowRight":
        event.preventDefault();
        switchMenu(1);
        return;
      case "ArrowLeft":
        event.preventDefault();
        switchMenu(-1);
        return;
      case "Escape":
        event.preventDefault();
        closeMenus(true);
        return;
      case "Tab":
        closeMenus(false);
        return;
    }
  }

  function handleTriggerKeydown(event: KeyboardEvent, id: NonNullable<MenuId>) {
    switch (event.key) {
      case "ArrowDown":
      case "Enter":
      case " ":
        event.preventDefault();
        void openWithFocus(id);
        return;
      case "ArrowRight":
        event.preventDefault();
        switchMenu(1);
        return;
      case "ArrowLeft":
        event.preventDefault();
        switchMenu(-1);
        return;
      case "Escape":
        if (openMenu === id) {
          event.preventDefault();
          openMenu = null;
        }
        return;
    }
  }

  function handleItemClick(item: MenuItem) {
    if (item.kind === "separator" || item.kind === "active") return;
    if (item.kind === "action" && item.disabled) return;
    item.action();
    openMenu = null;
  }

  function handleMenuHover(id: NonNullable<MenuId>) {
    if (openMenu !== null) {
      openMenu = id;
    }
  }

  function handleClickOutside(event: MouseEvent) {
    if (menuBarRef && !menuBarRef.contains(event.target as Node)) {
      openMenu = null;
    }
  }

  function globalEscape(event: KeyboardEvent) {
    if (event.key === "Escape" && openMenu) {
      event.preventDefault();
      closeMenus(true);
    }
  }
</script>

<svelte:window onclick={handleClickOutside} onkeydown={globalEscape} />

<div class="menu-bar" class:compact bind:this={menuBarRef} role="menubar" aria-label="应用菜单">
  <div class="menu-bar-left">
    <span class="menu-dot" style:color={phaseColor(sessionPhase)} aria-hidden="true">
      {phaseDot(sessionPhase)}
    </span>
    <span class="menu-agent-name" aria-label={`已连接: ${agentName || "dumbpipex"}`}>
      {agentName || "dumbpipex"}
    </span>
  </div>

  <div class="menu-bar-right">
    {#each MENU_ORDER as id (id)}
      {@const items = openMenu === id ? getMenuItems(id) : []}
      <div class="menu-item-wrapper">
        <button
          bind:this={triggerRefs[id]}
          class="menu-trigger"
          class:active={openMenu === id}
          aria-haspopup="menu"
          aria-expanded={openMenu === id}
          id={`menu-trigger-${id}`}
          onclick={() => toggleMenu(id)}
          onmouseenter={() => handleMenuHover(id)}
          onkeydown={(e) => handleTriggerKeydown(e, id)}
        >
          {id === "file" ? "File" : "View"}
        </button>
        {#if openMenu === id}
          <div
            class="menu-dropdown"
            role="menu"
            aria-labelledby={`menu-trigger-${id}`}
          >
            {#each items as item, idx (idx)}
              {#if item.kind === "separator"}
                <div class="menu-separator" role="separator"></div>
              {:else if item.kind === "active"}
                <div class="menu-section-label" role="presentation">{item.label}</div>
              {:else}
                <button
                  class="menu-item"
                  class:disabled={item.disabled ?? false}
                  role="menuitem"
                  bind:this={itemRefs[idx]}
                  disabled={item.disabled}
                  tabindex={openMenu === id ? -1 : -1}
                  onclick={() => handleItemClick(item)}
                  onkeydown={(e) => handleItemKeydown(e, item)}
                >
                  <span class="menu-item-label">{item.label}</span>
                  {#if item.shortcut}
                    <span class="menu-item-shortcut" aria-hidden="true">{item.shortcut}</span>
                  {/if}
                </button>
              {/if}
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .menu-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 2rem;
    padding: 0 0.6rem;
    background: #0f172a;
    border-bottom: 1px solid rgba(148, 163, 184, 0.12);
    flex-shrink: 0;
    user-select: none;
    -webkit-user-select: none;
    -webkit-app-region: drag;
  }

  .menu-bar-left {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    min-width: 0;
    overflow: hidden;
  }

  .menu-dot {
    font-size: 0.52rem;
    flex-shrink: 0;
  }

  .menu-agent-name {
    font-size: 0.78rem;
    font-weight: 600;
    color: #94a3b8;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .menu-bar-right {
    display: flex;
    align-items: center;
    gap: 0.15rem;
    -webkit-app-region: no-drag;
  }

  .menu-item-wrapper {
    position: relative;
  }

  .menu-trigger {
    padding: 0.18rem 0.55rem;
    border: none;
    border-radius: 0.3rem;
    background: transparent;
    color: #cbd5e1;
    font-size: 0.78rem;
    font-weight: 500;
    white-space: nowrap;
    transition: background-color 120ms ease, color 120ms ease;
  }

  @media (max-width: 680px) {
    .menu-trigger {
      min-height: 2.75rem; /* 44px tap target */
      padding: 0.5rem 0.85rem;
      font-size: 0.85rem;
    }

    .menu-bar {
      height: auto;
      min-height: 2.75rem;
      padding: 0.25rem 0.6rem;
    }
  }

  .menu-trigger:hover,
  .menu-trigger.active {
    background: rgba(59, 130, 246, 0.15);
    color: #e2e8f0;
  }

  .menu-trigger:focus-visible {
    outline: none;
    background: rgba(59, 130, 246, 0.22);
    color: #fff;
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.5);
  }

  .menu-dropdown {
    position: absolute;
    top: calc(100% + 0.25rem);
    right: 0;
    min-width: 14rem;
    padding: 0.25rem;
    background: rgba(15, 23, 42, 0.98);
    border: 1px solid rgba(148, 163, 184, 0.2);
    border-radius: 0.5rem;
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(20px);
    z-index: 100;
    display: flex;
    flex-direction: column;
    animation: menu-in 120ms ease both;
  }

  @keyframes menu-in {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .menu-dropdown {
      animation: none;
    }
  }

  .menu-section-label {
    padding: 0.28rem 0.6rem;
    font-size: 0.72rem;
    font-weight: 600;
    color: #64748b;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .menu-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.38rem 0.6rem;
    border: none;
    border-radius: 0.3rem;
    background: transparent;
    color: #e2e8f0;
    font-size: 0.8rem;
    text-align: left;
    white-space: nowrap;
    transition: background-color 80ms ease;
  }

  .menu-item:hover {
    background: rgba(59, 130, 246, 0.18);
    color: #fff;
  }

  .menu-item:focus-visible,
  .menu-item:focus {
    outline: none;
    background: rgba(59, 130, 246, 0.28);
    color: #fff;
    box-shadow: inset 0 0 0 1px rgba(147, 197, 253, 0.55);
  }

  .menu-item.disabled {
    opacity: 0.35;
    pointer-events: none;
  }

  .menu-item-label {
    font-weight: 500;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, monospace;
  }

  .menu-item-shortcut {
    color: #64748b;
    font-size: 0.72rem;
  }

  /* On phones the ⌘-prefixed shortcut hints are noise — there's no
     physical ⌘ key and the labels are long enough already. */
  .menu-bar.compact .menu-item-shortcut {
    display: none;
  }

  .menu-separator {
    height: 1px;
    margin: 0.2rem 0.5rem;
    background: rgba(148, 163, 184, 0.15);
  }
</style>

