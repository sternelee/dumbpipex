<script lang="ts">
  import { onMount } from "svelte";

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

  let sheetEl = $state<HTMLDivElement | null>(null);
  let open = $state(false);

  onMount(() => {
    requestAnimationFrame(() => {
      open = true;
    });

    // If a text input OUTSIDE the sheet is focused, the virtual
    // keyboard may be appearing. iOS in particular animates the
    // keyboard in 200–400ms *after* focus, and during that window the
    // sheet sits on top of where the keyboard is going to be, leaving
    // the user unable to reach the "完成" close button. Inputs INSIDE
    // the sheet are fine (search panel needs its own input) and are
    // not handled here — SessionWorkspace's !keyboardOpen guard
    // already prevents the sheet from rendering while a textbox has
    // focus.
    const handleFocusIn = (event: FocusEvent) => {
      const target = event.target as Element | null;
      if (!target) return;
      const tag = target.tagName;
      const isText = tag === "INPUT" || tag === "TEXTAREA" || (target as HTMLElement).isContentEditable;
      if (!isText) return;
      if (sheetEl && sheetEl.contains(target)) return;
      requestAnimationFrame(() => onClose());
    };
    document.addEventListener("focusin", handleFocusIn);
    return () => {
      document.removeEventListener("focusin", handleFocusIn);
    };
  });

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) onClose();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="mobile-sheet-backdrop"
  class:open
  onclick={handleBackdropClick}
>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    bind:this={sheetEl}
    class="mobile-sheet"
    class:open
    role="dialog"
    aria-modal="true"
    aria-label={title}
    tabindex="-1"
    onclick={(event) => event.stopPropagation()}
  >
    <div class="mobile-sheet-handle"></div>
    <div class="mobile-sheet-header">
      <div class="mobile-sheet-copy">
        <strong>{title}</strong>
        <small>{subtitle}</small>
      </div>
      <button class="quick-shortcut-action" type="button" onclick={onClose}>完成</button>
    </div>
    <div class="mobile-sheet-panel">
      {@render children?.()}
    </div>
  </div>
</div>

<style>
  .mobile-sheet-backdrop {
    position: fixed;
    inset: 0;
    z-index: 20;
    background: rgba(2, 6, 23, 0);
    transition: background-color 200ms ease;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    pointer-events: auto;
  }

  .mobile-sheet-backdrop.open {
    background: rgba(2, 6, 23, 0.52);
  }

  .mobile-sheet {
    position: relative;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 21;
    display: grid;
    gap: 0.9rem;
    width: min(100%, 30rem);
    max-height: min(70vh, 36rem);
    overflow: hidden;
    margin: 0 max(0.75rem, env(safe-area-inset-right)) max(0.75rem, env(safe-area-inset-bottom))
      max(0.75rem, env(safe-area-inset-left));
    padding: 0.9rem;
    border: 1px solid rgba(148, 163, 184, 0.22);
    border-radius: 1.2rem;
    background: rgba(2, 6, 23, 0.98);
    box-shadow: 0 18px 40px rgba(15, 23, 42, 0.52);
    backdrop-filter: blur(22px);
    transform: translateY(110%);
    opacity: 0;
    transition:
      transform 280ms cubic-bezier(0.2, 0.7, 0.2, 1),
      opacity 200ms ease;
  }

  .mobile-sheet.open {
    transform: translateY(0);
    opacity: 1;
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
    min-width: 0;
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

  @media (prefers-reduced-motion: reduce) {
    .mobile-sheet,
    .mobile-sheet-backdrop {
      transition: none;
    }
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

