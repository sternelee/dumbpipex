<script lang="ts">
  import type { PtySession } from "$lib/terminal-ui";

  let {
    ptys,
    activePtyId,
    compactLayout,
    keyboardOpen = false,
    busy,
    onSelectPty,
    onCreatePty,
    onCloseActivePty,
  }: {
    ptys: PtySession[];
    activePtyId: string | null;
    compactLayout: boolean;
    keyboardOpen?: boolean;
    busy: boolean;
    onSelectPty: (ptyId: string) => void;
    onCreatePty: () => void;
    onCloseActivePty: () => void;
  } = $props();

  let containerWidth = $state(0);
  let tabWidths = $state<Map<string, number>>(new Map());
  let dropdownOpen = $state(false);
  let dropdownUp = $state(false);
  let dropdownRef = $state<HTMLDivElement | null>(null);
  let sessionBarRef = $state<HTMLElement | null>(null);

  function hasActivePty() {
    return Boolean(activePtyId);
  }

  function getPtyShortName(shell: string) {
    return shell.split("/").pop() ?? shell;
  }

  // 计算可见标签：按顺序累加宽度，超出则收起
  const visiblePtys = $derived.by(() => {
    if (ptys.length === 0) return [];
    if (containerWidth <= 0) return ptys;

    const MORE_BTN_WIDTH = compactLayout ? 60 : 76;
    const GAP = compactLayout ? 4 : 8;
    let available = containerWidth - MORE_BTN_WIDTH;
    let count = 0;

    for (const pty of ptys) {
      const w = tabWidths.get(pty.pty_id) ?? (compactLayout ? 72 : 116);
      const need = w + (count > 0 ? GAP : 0);
      if (need > available && count > 0) break;
      available -= need;
      count++;
    }

    return ptys.slice(0, Math.max(1, count));
  });

  const hiddenPtys = $derived.by(() => {
    const visibleIds = new Set(visiblePtys.map((p) => p.pty_id));
    return ptys.filter((p) => !visibleIds.has(p.pty_id));
  });

  const hasHidden = $derived(hiddenPtys.length > 0);

  function selectPty(ptyId: string) {
    dropdownOpen = false;
    onSelectPty(ptyId);
  }

  function toggleDropdown() {
    if (!dropdownOpen && sessionBarRef) {
      // Decide direction at open time. If the bar sits in the top
      // half of the viewport, opening downward would push the menu
      // over the terminal; opening upward keeps it visible above
      // the bar instead.
      const rect = sessionBarRef.getBoundingClientRect();
      const vh = window.innerHeight || document.documentElement.clientHeight;
      dropdownUp = rect.top > vh / 2;
    }
    dropdownOpen = !dropdownOpen;
  }

  // 点击外部关闭下拉
  function handleClickOutside(event: MouseEvent) {
    if (dropdownRef && !dropdownRef.contains(event.target as Node)) {
      dropdownOpen = false;
    }
  }

  // Horizontal swipe between tabs on the tabs-scroll container. We
  // defer the axis decision until ~8px of travel so vertical scrolling
  // still works, and only fire the tab switch when the swipe passes a
  // 60px threshold (and is mostly horizontal). The action binds
  // non-passive listeners so we can preventDefault the browser's
  // back-swipe gesture once the axis is decided.
  function bindSwipe(
    node: HTMLDivElement,
    options: { onSwipeLeft: () => void; onSwipeRight: () => void },
  ) {
    let startX = 0;
    let startY = 0;
    let active = false;
    let axis: "h" | "v" | null = null;
    const onStart = (event: TouchEvent) => {
      if (event.touches.length !== 1) return;
      const touch = event.touches[0];
      startX = touch.clientX;
      startY = touch.clientY;
      active = true;
      axis = null;
    };
    const onMove = (event: TouchEvent) => {
      if (!active) return;
      const touch = event.touches[0];
      const dx = touch.clientX - startX;
      const dy = touch.clientY - startY;
      if (axis === null) {
        if (Math.abs(dx) < 8 && Math.abs(dy) < 8) return;
        axis = Math.abs(dx) > Math.abs(dy) * 1.4 ? "h" : "v";
      }
      if (axis === "h") event.preventDefault();
    };
    const onEnd = (event: TouchEvent) => {
      if (!active) return;
      active = false;
      const a = axis;
      axis = null;
      if (a !== "h") return;
      const touch = event.changedTouches[0];
      if (!touch) return;
      const dx = touch.clientX - startX;
      if (Math.abs(dx) < 60) return;
      if (dx < 0) options.onSwipeLeft();
      else options.onSwipeRight();
    };
    const onCancel = () => {
      active = false;
      axis = null;
    };
    node.addEventListener("touchstart", onStart, { passive: true });
    node.addEventListener("touchmove", onMove, { passive: false });
    node.addEventListener("touchend", onEnd, { passive: true });
    node.addEventListener("touchcancel", onCancel, { passive: true });
    return {
      update(next: typeof options) {
        options = next;
      },
      destroy() {
        node.removeEventListener("touchstart", onStart);
        node.removeEventListener("touchmove", onMove);
        node.removeEventListener("touchend", onEnd);
        node.removeEventListener("touchcancel", onCancel);
      },
    };
  }

  // ResizeObserver action 测量标签宽度
  function observeTab(node: HTMLElement, ptyId: string) {
    const ro = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const w =
          entry.borderBoxSize?.[0]?.inlineSize ?? entry.contentRect.width;
        if (tabWidths.get(ptyId) !== w) {
          tabWidths.set(ptyId, w);
          tabWidths = new Map(tabWidths);
        }
      }
    });
    ro.observe(node);
    return {
      destroy() {
        ro.disconnect();
      },
    };
  }
</script>

<svelte:window onclick={handleClickOutside} />

{#if ptys.length > 0}
  <div
    bind:this={sessionBarRef}
    class="session-bar"
    class:compact={compactLayout}
    class:keyboard-open={keyboardOpen}
  >
    <div
      class="tabs-scroll"
      bind:clientWidth={containerWidth}
      use:bindSwipe={{
        onSwipeLeft: () => {
          const idx = visiblePtys.findIndex((p) => p.pty_id === activePtyId);
          if (idx < 0) return;
          const next = visiblePtys[idx + 1];
          if (next) onSelectPty(next.pty_id);
        },
        onSwipeRight: () => {
          const idx = visiblePtys.findIndex((p) => p.pty_id === activePtyId);
          if (idx < 0) return;
          const prev = visiblePtys[idx - 1];
          if (prev) onSelectPty(prev.pty_id);
        },
      }}
    >
      <div class="tabs-row">
        {#each visiblePtys as pty (pty.pty_id)}
          <button
            class="tab"
            class:active={pty.pty_id === activePtyId}
            class:exited={pty.exited}
            aria-current={pty.pty_id === activePtyId ? "page" : undefined}
            use:observeTab={pty.pty_id}
            onclick={() => selectPty(pty.pty_id)}
            title={`${pty.pty_id} · ${pty.shell}${pty.exited ? " · exited" : ""}`}
          >
            <span class="tab-glow"></span>
            <span class="tab-content">
              <span class="tab-name">
                {compactLayout ? getPtyShortName(pty.shell) : pty.pty_id}
              </span>
              {#if !compactLayout}
                <span class="tab-meta"
                  >{pty.shell}{pty.exited ? " · exited" : ""}</span
                >
              {/if}
            </span>
            {#if pty.exited}
              <span class="tab-exit" aria-label="已退出">●</span>
            {/if}
            <span
              class="tab-close"
              role="button"
              tabindex="0"
              aria-label={`关闭 ${pty.pty_id}`}
              title={`关闭 ${pty.pty_id}`}
              onpointerdown={(e) => e.stopPropagation()}
              onclick={(e) => {
                e.stopPropagation();
                if (busy) return;
                if (pty.pty_id === activePtyId) {
                  onCloseActivePty();
                } else {
                  onSelectPty(pty.pty_id);
                  onCloseActivePty();
                }
              }}
              onkeydown={(e) => {
                if (e.key !== "Enter" && e.key !== " ") return;
                e.preventDefault();
                e.stopPropagation();
                if (busy) return;
                if (pty.pty_id !== activePtyId) onSelectPty(pty.pty_id);
                onCloseActivePty();
              }}
            >×</span>
          </button>
        {/each}

        {#if hasHidden}
          <div class="dropdown-wrapper" bind:this={dropdownRef}>
            <button
              class="tab more-tab"
              class:active={dropdownOpen}
              onclick={toggleDropdown}
              title={`${hiddenPtys.length} 个隐藏会话`}
            >
              <span class="tab-content">
                <span class="more-label">更多</span>
                <span class="more-badge">{hiddenPtys.length}</span>
              </span>
              <span class="more-arrow" class:open={dropdownOpen} class:flip={dropdownUp}>▾</span>
            </button>

            {#if dropdownOpen}
              <div class="dropdown-menu" class:up={dropdownUp}>
                {#each hiddenPtys as pty (pty.pty_id)}
                  <button
                    class="dropdown-item"
                    class:active={pty.pty_id === activePtyId}
                    class:exited={pty.exited}
                    aria-current={pty.pty_id === activePtyId ? "page" : undefined}
                    onclick={() => selectPty(pty.pty_id)}
                  >
                    <span
                      class="item-dot"
                      class:active={pty.pty_id === activePtyId}
                    ></span>
                    <span class="item-info">
                      <span class="item-name">{pty.pty_id}</span>
                      <span class="item-shell"
                        >{pty.shell}{pty.exited ? " · exited" : ""}</span
                      >
                    </span>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <div class="tab-inline-actions">
      <button
        class="tab-action-btn"
        onclick={onCreatePty}
        disabled={busy}
        title="新建 PTY (⌘T)"
      >
        <span class="tab-action-icon">＋</span>
      </button>
      <button
        class="tab-action-btn close-btn"
        onclick={onCloseActivePty}
        disabled={!hasActivePty() || busy}
        title="关闭 (⌘W)"
      >
        <span class="tab-action-icon">✕</span>
      </button>
    </div>
  </div>
{/if}

<style>
  .session-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
    padding: 0.65rem 0.75rem;
    background: rgba(15, 23, 42, 0.88);
    border: 1px solid rgba(148, 163, 184, 0.18);
    border-radius: 1rem;
    box-shadow: 0 10px 40px rgba(15, 23, 42, 0.45);
  }

  .tabs-scroll {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
    touch-action: pan-y;
  }

  .tabs-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  /* ===== Tab ===== */
  .tab {
    position: relative;
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.55rem 0.85rem;
    min-height: 2.6rem;
    border: 1px solid rgba(148, 163, 184, 0.24);
    border-radius: 0.85rem;
    background: rgba(15, 23, 42, 0.92);
    color: inherit;
    font: inherit;
    font-weight: 600;
    touch-action: manipulation;
    white-space: nowrap;
    flex-shrink: 0;

    transition:
      border-color 180ms ease,
      background-color 180ms ease,
      box-shadow 180ms ease;
    overflow: hidden;
  }

  .tab:hover {
    border-color: rgba(148, 163, 184, 0.45);
    background: rgba(30, 41, 59, 0.92);
  }

  .tab.active {
    border-color: rgba(59, 130, 246, 0.8);
    background: rgba(37, 99, 235, 0.18);
    box-shadow:
      0 0 0 1px rgba(59, 130, 246, 0.25),
      inset 0 1px 0 rgba(96, 165, 250, 0.15);
  }

  .tab.exited {
    opacity: 0.65;
  }

  .tab-glow {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    opacity: 0;
    transition: opacity 200ms ease;
    pointer-events: none;
  }

  .tab.active .tab-glow {
    opacity: 1;
    box-shadow: inset 0 0 20px rgba(59, 130, 246, 0.12);
  }

  .tab-content {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.1rem;
    line-height: 1.15;
  }

  .tab-name {
    font-size: 0.88rem;
    font-weight: 600;
  }

  .tab-meta {
    font-size: 0.72rem;
    color: #94a3b8;
    font-weight: 500;
  }

  .tab-exit {
    font-size: 0.6rem;
    color: #ef4444;
    margin-left: 0.15rem;
  }

  .tab-close {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.3rem;
    height: 1.3rem;
    margin-left: 0.2rem;
    border-radius: 0.4rem;
    color: rgba(148, 163, 184, 0.6);
    font-size: 0.95rem;
    line-height: 1;
    opacity: 0;
    transition: opacity 120ms ease, color 120ms ease, background-color 120ms ease;
    flex-shrink: 0;
  }

  .tab:hover .tab-close,
  .tab:focus-within .tab-close {
    opacity: 1;
  }

  .tab-close:hover,
  .tab-close:focus-visible {
    color: #fecaca;
    background: rgba(239, 68, 68, 0.18);
    outline: none;
  }

  .session-bar.compact .tab-close {
    opacity: 1;
    width: 1.15rem;
    height: 1.15rem;
    font-size: 0.85rem;
  }

  /* ===== More Tab ===== */
  .more-tab {
    padding: 0.55rem 0.65rem;
    gap: 0.25rem;
  }

  .more-tab .tab-content {
    flex-direction: row;
    align-items: center;
    gap: 0.35rem;
  }

  .more-label {
    font-size: 0.85rem;
  }

  .more-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 1.35rem;
    height: 1.35rem;
    padding: 0 0.3rem;
    border-radius: 999px;
    background: rgba(59, 130, 246, 0.25);
    color: #bfdbfe;
    font-size: 0.72rem;
    font-weight: 700;
  }

  .more-arrow {
    font-size: 0.7rem;
    color: #94a3b8;
    transition: transform 200ms ease;
    margin-left: 0.1rem;
  }

  .more-arrow.open {
    transform: rotate(180deg);
  }

  /* ===== Dropdown ===== */
  .dropdown-wrapper {
    position: relative;
    flex-shrink: 0;
  }

  .dropdown-menu {
    position: absolute;
    top: calc(100% + 0.4rem);
    right: 0;
    min-width: 16rem;
    max-width: min(80vw, calc(100vw - 2rem));
    max-height: 60vh;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    padding: 0.4rem;
    background: rgba(15, 23, 42, 0.96);
    border: 1px solid rgba(148, 163, 184, 0.25);
    border-radius: 0.85rem;
    box-shadow: 0 16px 48px rgba(15, 23, 42, 0.55);
    backdrop-filter: blur(16px);
    z-index: 50;
    animation: dropdown-in 160ms ease both;
  }

  /* Open upward when the bar is in the top half of the viewport,
     so the menu doesn't cover the terminal it sits above. */
  .dropdown-menu.up {
    top: auto;
    bottom: calc(100% + 0.4rem);
    animation-name: dropdown-in-up;
  }

  .more-arrow.flip {
    transform: rotate(180deg);
  }

  .more-arrow.open.flip {
    transform: rotate(0deg);
  }

  @keyframes dropdown-in-up {
    from {
      opacity: 0;
      transform: translateY(6px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  @keyframes dropdown-in {
    from {
      opacity: 0;
      transform: translateY(-6px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .dropdown-menu,
    .dropdown-menu.up {
      animation: none;
    }
  }

  .dropdown-item {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    padding: 0.55rem 0.7rem;
    border: none;
    border-radius: 0.65rem;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;

    touch-action: manipulation;
    transition: background-color 140ms ease;
  }

  .dropdown-item:hover {
    background: rgba(59, 130, 246, 0.12);
  }

  .dropdown-item.active {
    background: rgba(37, 99, 235, 0.18);
  }

  .dropdown-item.exited {
    opacity: 0.6;
  }

  .item-dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 999px;
    background: rgba(148, 163, 184, 0.5);
    flex-shrink: 0;
  }

  .item-dot.active {
    background: #3b82f6;
    box-shadow: 0 0 6px rgba(59, 130, 246, 0.5);
  }

  .item-info {
    display: flex;
    flex-direction: column;
    gap: 0.08rem;
    min-width: 0;
  }

  .item-name {
    font-weight: 600;
    font-size: 0.88rem;
  }

  .item-shell {
    font-size: 0.76rem;
    color: #94a3b8;
  }

  /* ===== Inline Actions ===== */
  .tab-inline-actions {
    display: flex;
    align-items: center;
    gap: 0.2rem;
    flex-shrink: 0;
  }

  .tab-action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.75rem;
    height: 1.75rem;
    border: 1px solid rgba(148, 163, 184, 0.15);
    border-radius: 0.4rem;
    background: transparent;
    color: #94a3b8;
    font: inherit;
    font-size: 0.8rem;
    transition:
      border-color 140ms ease,
      background-color 140ms ease,
      color 140ms ease;
  }

  .tab-action-btn:hover:not(:disabled) {
    border-color: rgba(148, 163, 184, 0.35);
    background: rgba(30, 41, 59, 0.6);
    color: #e2e8f0;
  }

  .tab-action-btn:disabled {
    opacity: 0.3;
  }

  .tab-action-icon {
    line-height: 1;
  }

  .tab-action-btn.close-btn:hover:not(:disabled) {
    border-color: rgba(239, 68, 68, 0.4);
    background: rgba(127, 29, 29, 0.25);
    color: #fca5a5;
  }

  /* ===== Compact / Mobile ===== */
  .session-bar.compact {
    gap: 0.35rem;
    padding: 0.45rem 0.55rem;
    border-radius: 0.9rem;
  }

  .session-bar.compact .tabs-row {
    gap: 0.35rem;
  }

  .session-bar.compact .tab {
    padding: 0.18rem 0.58rem;
    min-height: 1.6rem;
    border-radius: 0.7rem;
  }

  .session-bar.compact .tab-name {
    font-size: 0.8rem;
  }

  .session-bar.compact .more-tab {
    padding: 0.38rem 0.55rem;
  }

  .session-bar.compact .more-label {
    font-size: 0.78rem;
  }

  .session-bar.compact .more-badge {
    min-width: 1.15rem;
    height: 1.15rem;
    font-size: 0.68rem;
  }

  .session-bar.compact .tab-inline-actions {
    gap: 0.18rem;
  }

  .session-bar.compact .tab-action-btn {
    width: 1.55rem;
    height: 1.55rem;
    border-radius: 0.35rem;
  }

  /* Phone-compact (< 680px) — enforce 44px tap targets */
  @media (max-width: 680px) {
    .session-bar.compact {
      padding: 0.4rem 0.5rem;
    }

    .session-bar.compact .tab {
      min-height: 2.75rem; /* 44px iOS HIG */
      padding: 0.45rem 0.65rem;
    }

    .session-bar.compact .tab-name {
      font-size: 0.82rem;
    }

    .session-bar.compact .tab-close {
      width: 1.75rem;  /* ~28px visual, but with 0.4rem hit padding below */
      height: 1.75rem;
      font-size: 0.95rem;
      margin-left: 0.3rem;
    }

    .session-bar.compact .tab-action-btn {
      width: 2.75rem; /* 44px */
      height: 2.75rem;
      border-radius: 0.5rem;
    }

    .session-bar.compact .more-tab {
      min-height: 2.75rem;
      padding: 0.45rem 0.65rem;
    }

    .session-bar.compact .more-badge {
      min-width: 1.4rem;
      height: 1.4rem;
      font-size: 0.75rem;
    }
  }

  /* Keyboard open - ultra compact */
  .session-bar.compact.keyboard-open {
    padding: 0.32rem 0.42rem;
    gap: 0.25rem;
  }

  .session-bar.compact.keyboard-open .tab {
    padding: 0.28rem 0.45rem;
    min-height: 1.85rem;
    border-radius: 0.55rem;
  }

  .session-bar.compact.keyboard-open .tab-name {
    font-size: 0.74rem;
  }

  .session-bar.compact.keyboard-open .more-badge {
    min-width: 1rem;
    height: 1rem;
    font-size: 0.62rem;
  }

  .session-bar.compact.keyboard-open .tab-action-btn {
    width: 1.35rem;
    height: 1.35rem;
    border-radius: 0.3rem;
  }

  .session-bar.compact.keyboard-open .dropdown-menu {
    min-width: 13rem;
    padding: 0.3rem;
    border-radius: 0.7rem;
  }

  .session-bar.compact.keyboard-open .dropdown-item {
    padding: 0.42rem 0.55rem;
    border-radius: 0.55rem;
  }

  .session-bar.compact.keyboard-open .item-name {
    font-size: 0.82rem;
  }

  .session-bar.compact.keyboard-open .item-shell {
    font-size: 0.7rem;
  }

  /* ===== Responsive ===== */
  @media (max-width: 899px) {
    .session-bar {
      gap: 0.35rem;
      padding: 0.45rem 0.55rem;
      border-radius: 0.9rem;
    }

    .tabs-row {
      gap: 0.35rem;
    }

    .tab {
      padding: 0.18rem 0.58rem;
      min-height: 1.6rem;
      border-radius: 0.7rem;
    }

    .tab-name {
      font-size: 0.8rem;
    }

    .more-tab {
      padding: 0.38rem 0.55rem;
    }

    .tab-inline-actions {
      gap: 0.18rem;
    }

    .tab-action-btn {
      width: 1.55rem;
      height: 1.55rem;
      border-radius: 0.35rem;
    }

    .dropdown-menu {
      min-width: 14rem;
      max-width: 85vw;
    }
  }
</style>
