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
    <button class="toolbar-button" onclick={onDisconnect} disabled={busy}>
      {compactLayout ? "断开" : "Disconnect"}
    </button>
  </div>
</header>

<style>
  .workspace-header {
    padding: 1rem;
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
    background: rgba(15, 23, 42, 0.88);
    border: 1px solid rgba(148, 163, 184, 0.18);
    border-radius: 1rem;
    box-shadow: 0 10px 40px rgba(15, 23, 42, 0.45);
  }

  .workspace-copy h1 {
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

  button {
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

  button:disabled {
    opacity: 0.5;
  }

  @media (max-width: 899px) {
    .workspace-header {
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto;
      align-items: start;
      gap: 0.65rem;
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

    button {
      min-height: 2.6rem;
      font-size: 0.92rem;
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
  }
</style>
