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
    onFocusActivePty,
    onCopyActiveTerminal,
    onPaneData,
    onPaneNotice,
  }: {
    ptys: PtySession[];
    activePtyId: string | null;
    compactLayout: boolean;
    keyboardOpen?: boolean;
    busy: boolean;
    onSelectPty: (ptyId: string) => void;
    onCreatePty: () => void;
    onCloseActivePty: () => void;
    onFocusActivePty: () => void;
    onCopyActiveTerminal: () => void;
    onPaneData: (data: string) => void;
    onPaneNotice: (message: string) => void;
  } = $props();

  let containerWidth = $state(0);
  let tabWidths = $state<Map<string, number>>(new Map());
  let dropdownOpen = $state(false);
  let dropdownRef = $state<HTMLDivElement | null>(null);

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

  // 点击外部关闭下拉
  function handleClickOutside(event: MouseEvent) {
    if (dropdownRef && !dropdownRef.contains(event.target as Node)) {
      dropdownOpen = false;
    }
  }

  // ResizeObserver action 测量标签宽度
  function observeTab(node: HTMLElement, ptyId: string) {
    const ro = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const w = entry.borderBoxSize?.[0]?.inlineSize ?? entry.contentRect.width;
        if (tabWidths.get(ptyId) !== w) {
          tabWidths.set(ptyId, w);
          tabWidths = new Map(tabWidths);
        }
      }
    });
    ro.observe(node);
    return { destroy() { ro.disconnect(); } };
  }
</script>

<svelte:window onclick={handleClickOutside} />

{#if ptys.length > 0}
  <div class="session-bar" class:compact={compactLayout} class:keyboard-open={keyboardOpen}>
    <div class="tabs-scroll" bind:clientWidth={containerWidth}>
      <div class="tabs-row">
        {#each visiblePtys as pty (pty.pty_id)}
          <button
            class="tab"
            class:active={pty.pty_id === activePtyId}
            class:exited={pty.exited}
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
                <span class="tab-meta">{pty.shell}{pty.exited ? " · exited" : ""}</span>
              {/if}
            </span>
            {#if pty.exited}
              <span class="tab-exit">●</span>
            {/if}
          </button>
        {/each}

        {#if hasHidden}
          <div class="dropdown-wrapper" bind:this={dropdownRef}>
            <button
              class="tab more-tab"
              class:active={dropdownOpen}
              onclick={() => dropdownOpen = !dropdownOpen}
              title={`${hiddenPtys.length} 个隐藏会话`}
            >
              <span class="tab-content">
                <span class="more-label">更多</span>
                <span class="more-badge">{hiddenPtys.length}</span>
              </span>
              <span class="more-arrow" class:open={dropdownOpen}>▾</span>
            </button>

            {#if dropdownOpen}
              <div class="dropdown-menu">
                {#each hiddenPtys as pty (pty.pty_id)}
                  <button
                    class="dropdown-item"
                    class:active={pty.pty_id === activePtyId}
                    class:exited={pty.exited}
                    onclick={() => selectPty(pty.pty_id)}
                  >
                    <span class="item-dot" class:active={pty.pty_id === activePtyId}></span>
                    <span class="item-info">
                      <span class="item-name">{pty.pty_id}</span>
                      <span class="item-shell">{pty.shell}{pty.exited ? " · exited" : ""}</span>
                    </span>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <div class="tab-actions">
      <button class="action-btn" onclick={onCreatePty} disabled={busy} title="新建 PTY">
        <span class="action-icon">＋</span>
        {#if !compactLayout}<span class="action-label">新建</span>{/if}
      </button>
      <button class="action-btn" onclick={onCopyActiveTerminal} disabled={!hasActivePty()} title="复制">
        <span class="action-icon">⎘</span>
        {#if !compactLayout}<span class="action-label">复制</span>{/if}
      </button>
      <button class="action-btn" onclick={() => void pasteFromClipboard()} disabled={!hasActivePty() || busy} title="粘贴">
        <span class="action-icon">⎗</span>
        {#if !compactLayout}<span class="action-label">粘贴</span>{/if}
      </button>
      <button class="action-btn" onclick={onFocusActivePty} disabled={!hasActivePty()} title="聚焦键盘">
        <span class="action-icon">⌨</span>
        {#if !compactLayout}<span class="action-label">键盘</span>{/if}
      </button>
      <button class="action-btn close-btn" onclick={onCloseActivePty} disabled={!hasActivePty() || busy} title="关闭">
        <span class="action-icon">✕</span>
        {#if !compactLayout}<span class="action-label">关闭</span>{/if}
      </button>
    </div>
  </div>
{/if}

<style>
  .session-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
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
    cursor: pointer;
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

  .tab:active {
    transform: scale(0.97);
  }

  .tab.active {
    border-color: rgba(59, 130, 246, 0.8);
    background: rgba(37, 99, 235, 0.18);
    box-shadow: 0 0 0 1px rgba(59, 130, 246, 0.25), inset 0 1px 0 rgba(96, 165, 250, 0.15);
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
    max-width: 80vw;
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
    cursor: pointer;
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

  /* ===== Actions ===== */
  .tab-actions {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex-shrink: 0;
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.5rem 0.75rem;
    min-height: 2.4rem;
    border: 1px solid rgba(148, 163, 184, 0.24);
    border-radius: 0.8rem;
    background: rgba(15, 23, 42, 0.92);
    color: inherit;
    font: inherit;
    font-weight: 600;
    font-size: 0.85rem;
    cursor: pointer;
    touch-action: manipulation;
    white-space: nowrap;
    transition:
      border-color 140ms ease,
      background-color 140ms ease;
  }

  .action-btn:hover:not(:disabled) {
    border-color: rgba(148, 163, 184, 0.45);
    background: rgba(30, 41, 59, 0.92);
  }

  .action-btn:active:not(:disabled) {
    transform: scale(0.97);
  }

  .action-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .action-icon {
    font-size: 0.95rem;
    line-height: 1;
  }

  .close-btn:hover:not(:disabled) {
    border-color: rgba(239, 68, 68, 0.45);
    background: rgba(127, 29, 29, 0.2);
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
    padding: 0.38rem 0.58rem;
    min-height: 2.1rem;
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

  .session-bar.compact .tab-actions {
    gap: 0.25rem;
  }

  .session-bar.compact .action-btn {
    padding: 0.38rem 0.48rem;
    min-height: 2rem;
    border-radius: 0.65rem;
  }

  .session-bar.compact .action-icon {
    font-size: 0.9rem;
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

  .session-bar.compact.keyboard-open .action-btn {
    padding: 0.28rem 0.38rem;
    min-height: 1.75rem;
    border-radius: 0.55rem;
  }

  .session-bar.compact.keyboard-open .action-icon {
    font-size: 0.82rem;
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
      padding: 0.38rem 0.58rem;
      min-height: 2.1rem;
      border-radius: 0.7rem;
    }

    .tab-name {
      font-size: 0.8rem;
    }

    .more-tab {
      padding: 0.38rem 0.55rem;
    }

    .tab-actions {
      gap: 0.25rem;
    }

    .action-btn {
      padding: 0.38rem 0.48rem;
      min-height: 2rem;
      border-radius: 0.65rem;
      font-size: 0.8rem;
    }

    .action-icon {
      font-size: 0.9rem;
    }

    .dropdown-menu {
      min-width: 14rem;
      max-width: 85vw;
    }
  }
</style>
