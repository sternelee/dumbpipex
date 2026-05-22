<script lang="ts">
  let {
    title,
    subtitle,
    onClose,
    children,
  }: {
    title: string;
    subtitle: string;
    onClose: () => void;
    children?: import('svelte').Snippet;
  } = $props();
</script>

<button class="mobile-sheet-backdrop" aria-label="关闭移动面板" onclick={onClose}></button>
<div class="mobile-sheet" role="dialog" aria-modal="true" aria-label={title}>
  <div class="mobile-sheet-handle"></div>
  <div class="mobile-sheet-header">
    <div class="mobile-sheet-copy">
      <strong>{title}</strong>
      <small>{subtitle}</small>
    </div>
    <button class="quick-shortcut-action" onclick={onClose}>完成</button>
  </div>
  <div class="mobile-sheet-panel">
    {@render children?.()}
  </div>
</div>

<style>
  .mobile-sheet-backdrop {
    position: fixed;
    inset: 0;
    z-index: 20;
    border: 0;
    border-radius: 0;
    background: rgba(2, 6, 23, 0.52);
  }

  .mobile-sheet {
    position: fixed;
    left: max(0.75rem, env(safe-area-inset-left));
    right: max(0.75rem, env(safe-area-inset-right));
    bottom: max(0.75rem, env(safe-area-inset-bottom));
    z-index: 21;
    display: grid;
    gap: 0.9rem;
    max-height: min(68vh, 34rem);
    overflow: hidden;
    padding: 0.9rem;
    border: 1px solid rgba(148, 163, 184, 0.22);
    border-radius: 1.2rem;
    background: rgba(2, 6, 23, 0.98);
    box-shadow: 0 18px 40px rgba(15, 23, 42, 0.52);
    backdrop-filter: blur(22px);
  }

  .mobile-sheet-handle {
    width: 2.8rem;
    height: 0.3rem;
    margin: 0 auto;
    border-radius: 999px;
    background: rgba(148, 163, 184, 0.45);
  }

  .mobile-sheet-header {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    justify-content: space-between;
  }

  .mobile-sheet-copy {
    display: grid;
    gap: 0.15rem;
  }

  .mobile-sheet-copy strong {
    font-size: 0.95rem;
  }

  .mobile-sheet-copy small {
    color: #94a3b8;
    font-size: 0.76rem;
  }

  .mobile-sheet-panel {
    display: grid;
    gap: 0.75rem;
    min-height: 0;
    overflow-y: auto;
    padding-right: 0.1rem;
  }

  .quick-shortcut-action {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    line-height: 1.05;
    padding-inline: 0.56rem;
    transition:
      transform 140ms ease,
      border-color 140ms ease,
      background-color 140ms ease;
    border: 1px solid rgba(148, 163, 184, 0.24);
    border-radius: 0.72rem;
    background: rgba(15, 23, 42, 0.82);
    color: inherit;
    font: inherit;
    touch-action: manipulation;
    font-weight: 600;
    flex: 0 0 auto;
    min-height: 2.16rem;
  }

  .quick-shortcut-action:active {
    transform: scale(0.97);
  }

  @media (max-width: 899px) {
    .mobile-sheet {
      gap: 0.75rem;
      padding: 0.8rem;
      border-radius: 1rem;
    }

    .mobile-sheet-copy strong {
      font-size: 0.82rem;
    }

    .mobile-sheet-copy small {
      font-size: 0.68rem;
    }
  }
</style>
