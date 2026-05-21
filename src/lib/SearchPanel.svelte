<script lang="ts">
  let {
    searchQuery = $bindable(),
    onSearch,
  }: {
    searchQuery: string;
    onSearch: (query: string, direction: "next" | "previous") => void;
  } = $props();
</script>

<div class="panel-card search-panel">
  <input
    bind:value={searchQuery}
    class="search-input"
    placeholder="Search active terminal"
    onkeydown={(event) => event.key === "Enter" && onSearch(searchQuery, "next")}
  />
  <div class="panel-actions">
    <button class="toolbar-button" onclick={() => onSearch(searchQuery, "previous")} disabled={!searchQuery.trim()}>
      Prev
    </button>
    <button class="toolbar-button" onclick={() => onSearch(searchQuery, "next")} disabled={!searchQuery.trim()}>
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

  .search-input {
    width: 100%;
    padding: 0.85rem 0.9rem;
    border: 1px solid rgba(148, 163, 184, 0.24);
    border-radius: 0.9rem;
    background: rgba(15, 23, 42, 0.92);
    color: inherit;
    font: inherit;
  }

  .panel-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    align-items: center;
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
    .search-panel {
      grid-template-columns: 1fr;
    }

    .search-input {
      padding: 0.72rem 0.82rem;
    }
  }
</style>
