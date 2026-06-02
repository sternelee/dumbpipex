<script lang="ts">
  import { onMount, tick } from "svelte";

  let {
    searchQuery = $bindable(),
    onSearch,
    onClose,
  }: {
    searchQuery: string;
    onSearch: (query: string, direction: "next" | "previous") => boolean;
    onClose?: () => void;
  } = $props();

  let inputEl = $state<HTMLInputElement | null>(null);
  let status = $state<"idle" | "found" | "empty">("idle");
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    const q = searchQuery;
    if (debounceTimer) clearTimeout(debounceTimer);
    if (!q.trim()) {
      status = "idle";
      return;
    }
    debounceTimer = setTimeout(() => {
      const trimmed = q.trim();
      if (!trimmed) return;
      const ok = onSearch(trimmed, "next");
      status = ok ? "found" : "empty";
    }, 180);
  });

  onMount(async () => {
    await tick();
    inputEl?.focus();
    inputEl?.select();
  });

  function handleKey(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      const q = searchQuery.trim();
      if (!q) return;
      const direction: "next" | "previous" = event.shiftKey ? "previous" : "next";
      const ok = onSearch(q, direction);
      status = ok ? "found" : "empty";
    } else if (event.key === "Escape") {
      event.preventDefault();
      onClose?.();
    }
  }

  function clearQuery() {
    searchQuery = "";
    status = "idle";
    inputEl?.focus();
  }
</script>

<div class="panel-card search-panel" data-status={status}>
  <div class="search-input-wrap">
    <span class="search-icon" aria-hidden="true">⌕</span>
    <input
      bind:this={inputEl}
      bind:value={searchQuery}
      class="search-input"
      type="text"
      placeholder="搜索活动终端（Enter 下一个 · Shift+Enter 上一个）"
      aria-label="搜索活动终端"
      onkeydown={handleKey}
    />
    {#if searchQuery}
      <button
        type="button"
        class="search-clear"
        aria-label="清空搜索"
        title="清空"
        onclick={clearQuery}
      >×</button>
    {/if}
  </div>
  <div class="panel-actions">
    <span class="search-status" data-status={status} aria-live="polite">
      {#if status === "empty"}
        无匹配
      {:else if status === "found"}
        匹配
      {:else}
        &nbsp;
      {/if}
    </span>
    <button
      class="toolbar-button"
      type="button"
      onclick={() => {
        const q = searchQuery.trim();
        if (!q) return;
        const ok = onSearch(q, "previous");
        status = ok ? "found" : "empty";
      }}
      disabled={!searchQuery.trim()}
      title="上一个 (Shift+Enter)"
    >
      Prev
    </button>
    <button
      class="toolbar-button"
      type="button"
      onclick={() => {
        const q = searchQuery.trim();
        if (!q) return;
        const ok = onSearch(q, "next");
        status = ok ? "found" : "empty";
      }}
      disabled={!searchQuery.trim()}
      title="下一个 (Enter)"
    >
      Next
    </button>
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

  .search-panel {
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
  }

  .search-panel[data-status="empty"] {
    border-color: rgba(239, 68, 68, 0.4);
  }

  .search-input-wrap {
    position: relative;
    display: flex;
    align-items: center;
    min-width: 0;
  }

  .search-icon {
    position: absolute;
    left: 0.85rem;
    color: #94a3b8;
    font-size: 1rem;
    pointer-events: none;
    line-height: 1;
  }

  .search-input {
    width: 100%;
    padding: 0.85rem 2.6rem 0.85rem 2.5rem;
    border: 1px solid rgba(148, 163, 184, 0.24);
    border-radius: 0.9rem;
    background: rgba(15, 23, 42, 0.92);
    color: inherit;
    font: inherit;
    transition: border-color 160ms ease, box-shadow 160ms ease;
  }

  .search-input:focus-visible {
    outline: none;
    border-color: rgba(59, 130, 246, 0.65);
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.2);
  }

  .search-clear {
    position: absolute;
    right: 0.4rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.7rem;
    height: 1.7rem;
    border: none;
    border-radius: 999px;
    background: transparent;
    color: rgba(148, 163, 184, 0.85);
    font-size: 0.95rem;
    line-height: 1;
    cursor: pointer;
    transition: background-color 120ms ease, color 120ms ease;
  }

  .search-clear:hover,
  .search-clear:focus-visible {
    background: rgba(148, 163, 184, 0.18);
    color: #e2e8f0;
    outline: none;
  }

  .panel-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    align-items: center;
  }

  .search-status {
    min-width: 3rem;
    text-align: center;
    font-size: 0.78rem;
    font-weight: 600;
    color: #64748b;
    padding: 0 0.4rem;
    transition: color 160ms ease;
  }

  .search-status[data-status="found"] {
    color: #34d399;
  }

  .search-status[data-status="empty"] {
    color: #fca5a5;
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
    transition: border-color 140ms ease, background-color 140ms ease, color 140ms ease;
  }

  .toolbar-button:hover:not(:disabled) {
    border-color: rgba(59, 130, 246, 0.55);
    background: rgba(37, 99, 235, 0.15);
    color: #dbeafe;
  }

  .toolbar-button:focus-visible {
    outline: none;
    border-color: rgba(59, 130, 246, 0.65);
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.2);
  }

  .toolbar-button:disabled {
    opacity: 0.5;
  }

  @media (max-width: 899px) {
    .search-panel {
      grid-template-columns: 1fr;
    }

    .search-input {
      padding: 0.72rem 2.95rem 0.72rem 2.3rem;
      font-size: 16px; /* prevent iOS zoom */
      min-height: 2.75rem; /* 44px tap target */
    }

    .search-clear {
      width: 2.75rem; /* 44px tap target */
      height: 2.75rem;
      right: 0.25rem;
    }

    .toolbar-button {
      min-height: 2.75rem; /* 44px */
    }
  }
</style>

