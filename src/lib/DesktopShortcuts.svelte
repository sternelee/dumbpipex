<script lang="ts">
  import { shortcutSections } from "$lib/terminal-ui";
  import type { ShortcutSection } from "$lib/terminal-ui";

  let {
    hasActivePty,
    busy,
    onSendShortcut,
  }: {
    hasActivePty: boolean;
    busy: boolean;
    onSendShortcut: (data: string) => void;
  } = $props();
</script>

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
              onclick={() => onSendShortcut(shortcut.data)}
              disabled={!hasActivePty || busy}
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

<style>
  .shortcut-sections {
    display: grid;
    gap: 0.7rem;
    max-height: min(32vh, 22rem);
    overflow-y: auto;
    padding-right: 0.1rem;
  }

  .shortcut-section {
    display: grid;
    gap: 0.45rem;
  }

  .shortcut-section-title {
    font-weight: 700;
    color: #cbd5e1;
    font-size: 0.92rem;
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
    border: 1px solid rgba(148, 163, 184, 0.24);
    border-radius: 0.9rem;
    background: rgba(15, 23, 42, 0.92);
    color: inherit;
    font: inherit;
    touch-action: manipulation;
    font-weight: 600;
  }

  .shortcut small {
    color: #94a3b8;
    font-size: 0.76rem;
  }

  .shortcut:disabled {
    opacity: 0.5;
  }

  @media (max-width: 899px) {
    .shortcut-sections {
      max-height: none;
      padding-bottom: calc(0.35rem + env(safe-area-inset-bottom));
    }
  }
</style>
