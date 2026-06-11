<script lang="ts">
  import type { SessionPhase } from "$lib/terminal-ui";

  let {
    agentName,
    status,
    sessionPhase,
    busy,
    keyboardOpen = false,
    onDisconnect,
  }: {
    agentName: string;
    status: string;
    sessionPhase: SessionPhase;
    compactLayout: boolean;
    keyboardOpen?: boolean;
    busy: boolean;
    onDisconnect: () => void;
  } = $props();

  function phaseDot(phase: SessionPhase): string {
    if (phase === "ready") return "●";
    if (phase === "connecting" || phase === "creating_pty") return "◑";
    if (phase === "disconnecting") return "◌";
    return "○";
  }

  function phaseColor(phase: SessionPhase): string {
    if (phase === "ready") return "#34d399";
    if (phase === "connecting" || phase === "creating_pty") return "#facc15";
    if (phase === "disconnecting") return "#f87171";
    return "#94a3b8";
  }
</script>

<header
  class="mobile-header"
  class:keyboard-open={keyboardOpen}
>
  <div class="mobile-header-left">
    <span class="mobile-dot" style:color={phaseColor(sessionPhase)}>
      {phaseDot(sessionPhase)}
    </span>
    <span class="mobile-agent-name">{agentName || "Remote session"}</span>
  </div>
  <div class="mobile-header-center" class:hidden={keyboardOpen}>
    <span class="mobile-status">{status}</span>
  </div>
  <button class="mobile-disconnect" onclick={onDisconnect} disabled={busy}>
    断开
  </button>
</header>

<style>
  .mobile-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.45rem 0.65rem;
    background: rgba(15, 23, 42, 0.88);
    border: 1px solid rgba(148, 163, 184, 0.18);
    border-radius: 0.75rem;
    min-height: 2.4rem;
    flex-shrink: 0;
    position: sticky;
    top: 0;
    z-index: 10;
  }

  .mobile-header.keyboard-open {
    padding: 0.28rem 0.5rem;
    min-height: 0;
    gap: 0.35rem;
    border-radius: 0.55rem;
  }

  .mobile-header-left {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    min-width: 0;
    overflow: hidden;
  }

  .mobile-dot {
    font-size: 0.5rem;
    flex-shrink: 0;
  }

  .mobile-agent-name {
    font-size: 0.82rem;
    font-weight: 700;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .mobile-header.keyboard-open .mobile-agent-name {
    font-size: 0.75rem;
  }

  .mobile-header-center {
    min-width: 0;
    overflow: hidden;
  }

  .mobile-header-center.hidden {
    display: none;
  }

  .mobile-status {
    font-size: 0.72rem;
    color: #94a3b8;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .mobile-disconnect {
    border: 1px solid rgba(148, 163, 184, 0.2);
    border-radius: 0.55rem;
    background: transparent;
    color: inherit;
    font: inherit;
    padding: 0.3rem 0.55rem;
    font-size: 0.78rem;
    font-weight: 600;
    flex-shrink: 0;
    touch-action: manipulation;
    transition:
      background-color 140ms ease,
      border-color 140ms ease;
  }

  .mobile-header.keyboard-open .mobile-disconnect {
    padding: 0.18rem 0.4rem;
    font-size: 0.72rem;
    border-radius: 0.4rem;
  }

  .mobile-disconnect:hover:not(:disabled) {
    border-color: rgba(239, 68, 68, 0.4);
    background: rgba(127, 29, 29, 0.2);
  }

  .mobile-disconnect:disabled {
    opacity: 0.5;
  }
</style>
