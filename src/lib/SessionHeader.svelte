<script lang="ts">
  import { sessionModeLabel } from "$lib/terminal-ui";
  import type { SessionMode, SessionPhase } from "$lib/terminal-ui";

  let {
    agentName,
    status,
    activeMode,
    sessionPhase,
    compactLayout,
    busy,
    onDisconnect,
  }: {
    agentName: string;
    status: string;
    activeMode: SessionMode;
    sessionPhase: SessionPhase;
    compactLayout: boolean;
    busy: boolean;
    onDisconnect: () => void;
  } = $props();

  let collapsed = $state(false);
  let autoCollapsedOnce = $state(false);

  // 连接成功后（第一次进入 ready）自动收起
  $effect(() => {
    if (sessionPhase === "ready" && !autoCollapsedOnce) {
      collapsed = true;
      autoCollapsedOnce = true;
    }
  });

  function toggle() {
    collapsed = !collapsed;
  }

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
</script>

<header class="workspace-header" class:collapsed class:compact={compactLayout}>
  <!-- 收缩条：始终可见 -->
  <div class="header-bar">
    <button class="header-bar-inner" onclick={toggle} type="button">
      <div class="header-bar-info">
        <span class="header-bar-name">{agentName || "Remote session"}</span>
        {#if collapsed}
          <span class="status-pill compact-pill">{phaseLabel(sessionPhase)}</span>
        {/if}
      </div>
      <span class="toggle-icon" aria-label={collapsed ? "展开" : "收起"}>
        {collapsed ? "▾" : "▴"}
      </span>
    </button>

    <button class="disconnect-btn" onclick={onDisconnect} disabled={busy}>
      {compactLayout ? "断开" : "Disconnect"}
    </button>
  </div>

  <!-- 展开内容 -->
  <div class="header-body">
    <div class="header-body-inner">
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
      </div>
    </div>
  </div>
</header>

<style>
  .workspace-header {
    display: grid;
    background: rgba(15, 23, 42, 0.88);
    border: 1px solid rgba(148, 163, 184, 0.18);
    border-radius: 1rem;
    box-shadow: 0 10px 40px rgba(15, 23, 42, 0.45);
    transition:
      padding 280ms ease,
      gap 280ms ease,
      border-radius 280ms ease;
    padding: 1rem;
    gap: 0.75rem;
  }

  .workspace-header.collapsed {
    padding: 0.55rem 0.75rem;
    gap: 0;
    border-radius: 0.85rem;
  }

  /* ===== 收缩条 ===== */
  .header-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    min-height: 2.2rem;
  }

  .header-bar-inner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    flex: 1;
    min-width: 0;
    padding: 0.35rem 0.6rem;
    border: 1px solid rgba(148, 163, 184, 0.18);
    border-radius: 0.75rem;
    background: rgba(30, 41, 59, 0.5);
    color: inherit;
    font: inherit;
    cursor: pointer;
    touch-action: manipulation;
    transition:
      background-color 180ms ease,
      border-color 180ms ease;
  }

  .header-bar-inner:hover {
    background: rgba(59, 130, 246, 0.12);
    border-color: rgba(59, 130, 246, 0.35);
  }

  .header-bar-inner:active {
    transform: scale(0.99);
  }

  .header-bar-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
    overflow: hidden;
  }

  .header-bar-name {
    font-weight: 700;
    font-size: 0.95rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .toggle-icon {
    font-size: 0.75rem;
    color: #94a3b8;
    flex-shrink: 0;
    transition: transform 200ms ease;
    width: 1.2rem;
    text-align: center;
  }

  .workspace-header.collapsed .toggle-icon {
    transform: rotate(0deg);
  }

  .workspace-header:not(.collapsed) .toggle-icon {
    transform: rotate(180deg);
  }

  .disconnect-btn {
    border: 1px solid rgba(148, 163, 184, 0.24);
    border-radius: 0.9rem;
    background: rgba(15, 23, 42, 0.92);
    color: inherit;
    font: inherit;
    min-height: 2.4rem;
    touch-action: manipulation;
    padding: 0.6rem 0.85rem;
    font-weight: 600;
    font-size: 0.9rem;
    flex-shrink: 0;
    transition:
      background-color 140ms ease,
      border-color 140ms ease;
  }

  .disconnect-btn:hover:not(:disabled) {
    border-color: rgba(239, 68, 68, 0.45);
    background: rgba(127, 29, 29, 0.2);
  }

  .disconnect-btn:active:not(:disabled) {
    transform: scale(0.97);
  }

  .disconnect-btn:disabled {
    opacity: 0.5;
  }

  /* ===== 展开内容 ===== */
  .header-body {
    display: grid;
    grid-template-rows: 1fr;
    opacity: 1;
    overflow: hidden;
    transition:
      grid-template-rows 280ms ease,
      opacity 220ms ease,
      margin-top 280ms ease;
    margin-top: 0;
  }

  .workspace-header.collapsed .header-body {
    grid-template-rows: 0fr;
    opacity: 0;
    margin-top: 0;
  }

  .header-body-inner {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
    min-height: 0;
    overflow: hidden;
  }

  .workspace-copy {
    min-width: 0;
    overflow: hidden;
  }

  .workspace-copy h1 {
    margin: 0;
    font-size: 1.5rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .workspace-header-meta {
    display: grid;
    gap: 0.35rem;
    margin-top: 0.35rem;
    min-width: 0;
  }

  .workspace-copy p {
    margin: 0;
    color: #94a3b8;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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

  .header-actions {
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

  .compact-pill {
    font-size: 0.75rem;
    padding: 0.2rem 0.55rem;
  }

  /* ===== Compact / Mobile ===== */
  .workspace-header.compact {
    padding: 0.75rem;
  }

  .workspace-header.compact.collapsed {
    padding: 0.45rem 0.55rem;
  }

  .workspace-header.compact .header-bar {
    min-height: 2rem;
  }

  .workspace-header.compact .header-bar-inner {
    padding: 0.3rem 0.5rem;
    border-radius: 0.65rem;
  }

  .workspace-header.compact .header-bar-name {
    font-size: 0.88rem;
  }

  .workspace-header.compact .disconnect-btn {
    min-height: 2rem;
    padding: 0.42rem 0.68rem;
    font-size: 0.82rem;
    border-radius: 0.7rem;
  }

  .workspace-header.compact .workspace-copy h1 {
    font-size: 1.25rem;
  }

  .workspace-header.compact .workspace-header-meta {
    gap: 0.28rem;
    margin-top: 0.28rem;
  }

  .workspace-header.compact .workspace-copy p {
    font-size: 0.88rem;
  }

  .workspace-header.compact .eyebrow,
  .workspace-header.compact .mode-chip,
  .workspace-header.compact .status-pill {
    font-size: 0.74rem;
  }

  .workspace-header.compact .header-actions {
    width: auto;
    justify-self: end;
    justify-content: flex-end;
    align-self: start;
    gap: 0.45rem;
  }

  .workspace-header.compact .compact-status-pill {
    width: fit-content;
  }

  @media (max-width: 899px) {
    .workspace-header {
      padding: 0.75rem;
    }

    .workspace-header.collapsed {
      padding: 0.45rem 0.55rem;
    }

    .header-bar {
      min-height: 2rem;
    }

    .header-bar-inner {
      padding: 0.3rem 0.5rem;
      border-radius: 0.65rem;
    }

    .header-bar-name {
      font-size: 0.88rem;
    }

    .disconnect-btn {
      min-height: 2rem;
      padding: 0.42rem 0.68rem;
      font-size: 0.82rem;
      border-radius: 0.7rem;
    }

    .workspace-copy h1 {
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

    .header-actions {
      width: auto;
      justify-self: end;
      justify-content: flex-end;
      align-self: start;
      gap: 0.45rem;
    }

    .compact-status-pill {
      width: fit-content;
    }
  }
</style>
