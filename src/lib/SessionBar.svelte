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

  function hasActivePty() {
    return Boolean(activePtyId);
  }

  function selectPtyFromPicker(event: Event) {
    const nextPtyId = (event.currentTarget as HTMLSelectElement).value;
    if (nextPtyId) onSelectPty(nextPtyId);
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
</script>

{#if !compactLayout && ptys.length > 0}
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

{#if compactLayout && ptys.length > 0}
  <div class="panel-card mobile-session-bar" class:keyboard-open={keyboardOpen}>
    <div class="mobile-session-inline">
      <label class="mobile-session-picker session-label" for="mobile-pty-select">会话</label>
      <select
        id="mobile-pty-select"
        class="mobile-session-select"
        onchange={selectPtyFromPicker}
        value={activePtyId ?? ""}
      >
        {#each ptys as pty}
          <option value={pty.pty_id}>{pty.pty_id} · {pty.shell}{pty.exited ? " · exited" : ""}</option>
        {/each}
      </select>

      <button
        class="toolbar-button mobile-session-action mobile-session-add"
        onclick={onCreatePty}
        disabled={busy}
      >
        +PTY
      </button>
    </div>

    <div class="mobile-session-actions">
      <button
        class="toolbar-button mobile-session-action"
        onclick={onCopyActiveTerminal}
        disabled={!hasActivePty()}
      >
        复制
      </button>
      <button
        class="toolbar-button mobile-session-action"
        onclick={() => void pasteFromClipboard()}
        disabled={!hasActivePty() || busy}
      >
        粘贴
      </button>
      <button
        class="toolbar-button mobile-session-action"
        onclick={onFocusActivePty}
        disabled={!hasActivePty()}
      >
        键盘
      </button>
      <button
        class="toolbar-button mobile-session-action"
        onclick={onCloseActivePty}
        disabled={!hasActivePty() || busy}
      >
        关闭
      </button>
    </div>
  </div>
{/if}

<style>
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
    border: 1px solid rgba(148, 163, 184, 0.24);
    border-radius: 0.9rem;
    background: rgba(15, 23, 42, 0.92);
    color: inherit;
    font: inherit;
    min-height: 2.9rem;
    touch-action: manipulation;
    font-weight: 600;
  }

  .pty-tab.active-tab {
    border-color: rgba(59, 130, 246, 0.8);
    background: rgba(37, 99, 235, 0.18);
  }

  .pty-tab small {
    color: #94a3b8;
    font-size: 0.76rem;
  }

  .panel-card {
    background: rgba(15, 23, 42, 0.88);
    border: 1px solid rgba(148, 163, 184, 0.18);
    border-radius: 1rem;
    box-shadow: 0 10px 40px rgba(15, 23, 42, 0.45);
    padding: 0.75rem;
    display: grid;
    gap: 0.75rem;
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
    border: 1px solid rgba(148, 163, 184, 0.24);
    border-radius: 0.9rem;
    background: rgba(15, 23, 42, 0.92);
    color: inherit;
    font: inherit;
    min-height: 2.6rem;
    touch-action: manipulation;
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
    border: 1px solid rgba(148, 163, 184, 0.24);
    border-radius: 0.9rem;
    background: rgba(15, 23, 42, 0.92);
    color: inherit;
    font: inherit;
    touch-action: manipulation;
    font-weight: 600;
  }

  .mobile-session-add {
    min-width: 4.35rem;
  }

  .toolbar-button {
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

  .toolbar-button:disabled {
    opacity: 0.5;
  }

  @media (max-width: 899px) {
    .mobile-session-bar.keyboard-open {
      padding: 0.3rem;
      gap: 0.25rem;
    }

    .mobile-session-bar.keyboard-open .mobile-session-select {
      min-height: 1.85rem;
      padding: 0.35rem 0.5rem;
      font-size: 0.78rem;
    }

    .mobile-session-bar.keyboard-open .mobile-session-action {
      min-height: 1.75rem;
      padding: 0.28rem 0.38rem;
      font-size: 0.72rem;
    }

    .mobile-session-bar.keyboard-open .mobile-session-inline {
      gap: 0.25rem;
    }

    .mobile-session-bar.keyboard-open .mobile-session-actions {
      gap: 0.25rem;
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
  }
</style>
