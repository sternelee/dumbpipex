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
  {#each shortcutSections as section (section.title)}
    <section class="shortcut-section">
      <header class="shortcut-section-title">{section.title}</header>
      {#each section.rows as row, rowIdx (rowIdx)}
        <div
          class="shortcut-row"
          data-cols={row.length}
        >
          {#each row as shortcut, scIdx (scIdx)}
            <button
              class="shortcut"
              type="button"
              onclick={() => onSendShortcut(shortcut.data)}
              disabled={!hasActivePty || busy}
              title={shortcut.hint ?? shortcut.label}
              aria-label={shortcut.hint ? `${shortcut.label} · ${shortcut.hint}` : shortcut.label}
            >
              <span class="shortcut-label">{shortcut.label}</span>
              {#if shortcut.hint}
                <small class="shortcut-hint">{shortcut.hint}</small>
              {/if}
            </button>
          {/each}
        </div>
      {/each}
    </section>
  {/each}
</div>

<style>
  .shortcut-sections {
    display: grid;
    gap: 0.85rem;
    max-height: min(32vh, 22rem);
    overflow-y: auto;
    padding: 0.15rem 0.2rem 0.15rem 0.1rem;
    scrollbar-gutter: stable;
  }

  .shortcut-section {
    display: grid;
    gap: 0.5rem;
    padding: 0.55rem 0.6rem 0.6rem;
    background: rgba(15, 23, 42, 0.5);
    border: 1px solid rgba(148, 163, 184, 0.12);
    border-radius: 0.85rem;
  }

  .shortcut-section-title {
    font-weight: 700;
    color: #cbd5e1;
    font-size: 0.86rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .shortcut-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(5.5rem, 1fr));
    gap: 0.45rem;
  }

  .shortcut {
    padding: 0.7rem 0.5rem;
    min-height: 3.4rem;
    display: grid;
    gap: 0.15rem;
    place-items: center;
    text-align: center;
    border: 1px solid rgba(148, 163, 184, 0.24);
    border-radius: 0.85rem;
    background: rgba(15, 23, 42, 0.92);
    color: inherit;
    font: inherit;
    touch-action: manipulation;
    font-weight: 600;
    transition:
      border-color 140ms ease,
      background-color 140ms ease,
      transform 80ms ease;
  }

  .shortcut:hover:not(:disabled) {
    border-color: rgba(59, 130, 246, 0.6);
    background: rgba(37, 99, 235, 0.18);
  }

  .shortcut:focus-visible {
    outline: none;
    border-color: rgba(59, 130, 246, 0.7);
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.22);
  }

  .shortcut:active:not(:disabled) {
    transform: scale(0.97);
  }

  .shortcut-label {
    font-size: 0.92rem;
    line-height: 1.1;
    word-break: break-word;
  }

  .shortcut-hint {
    color: #94a3b8;
    font-size: 0.7rem;
    line-height: 1.15;
    font-weight: 500;
  }

  .shortcut:disabled {
    opacity: 0.5;
  }

  @media (max-width: 899px) {
    .shortcut-sections {
      max-height: none;
      padding-bottom: calc(0.5rem + env(safe-area-inset-bottom));
    }
  }
</style>
