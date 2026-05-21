<script lang="ts">
  import { terminalThemes } from "$lib/terminal-ui";
  import type { TerminalThemeOption } from "$lib/terminal-ui";

  let {
    fontSize,
    themeId = $bindable(),
    onAdjustFontSize,
  }: {
    fontSize: number;
    themeId: string;
    onAdjustFontSize: (delta: number) => void;
  } = $props();
</script>

<div class="panel-card display-panel">
  <div class="display-row">
    <span class="panel-label">Font size</span>
    <div class="panel-actions">
      <button class="toolbar-button" onclick={() => onAdjustFontSize(-1)}>-</button>
      <span class="value-chip">{fontSize}px</span>
      <button class="toolbar-button" onclick={() => onAdjustFontSize(1)}>+</button>
    </div>
  </div>

  <div class="theme-grid">
    {#each terminalThemes as item}
      <button
        class:active-theme={item.id === themeId}
        class="theme-chip"
        onclick={() => (themeId = item.id)}
      >
        {item.label}
      </button>
    {/each}
  </div>
</div>

<style>
  .panel-card {
    background: rgba(15, 23, 42, 0.88);
    border: 1px solid rgba(148, 163, 184, 0.18);
    border-radius: 1rem;
    box-shadow: 0 10px 40px rgba(15, 23, 42, 0.45);
    padding: 0.75rem;
    display: grid;
    gap: 0.75rem;
  }

  .display-row {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
  }

  .panel-label {
    color: #cbd5e1;
    font-size: 0.92rem;
  }

  .panel-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .value-chip {
    min-width: 4rem;
    text-align: center;
    padding: 0.55rem 0.8rem;
    border-radius: 999px;
    background: rgba(59, 130, 246, 0.16);
    color: #bfdbfe;
    font-size: 0.9rem;
    font-weight: 600;
  }

  .theme-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.55rem;
  }

  .theme-chip {
    padding: 0.65rem 0.5rem;
    min-height: 2.6rem;
    border: 1px solid rgba(148, 163, 184, 0.24);
    border-radius: 0.9rem;
    background: rgba(15, 23, 42, 0.92);
    color: inherit;
    font: inherit;
    touch-action: manipulation;
    font-weight: 600;
  }

  .theme-chip.active-theme {
    border-color: rgba(59, 130, 246, 0.8);
    background: rgba(37, 99, 235, 0.18);
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

  @media (max-width: 899px) {
    .theme-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
