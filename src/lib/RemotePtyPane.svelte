<script lang="ts" module>
  export type RemotePtyTheme = {
    background: string;
    foreground: string;
    cursor: string;
    selectionBackground?: string;
    selectionInactiveBackground?: string;
  };

  export type RemotePtyApi = {
    writeBase64Url: (data: string) => void;
    writeText: (data: string) => void;
    clear: () => void;
    fit: () => void;
    focus: () => void;
    finish: () => void;
    findNext: (query: string) => boolean;
    findPrevious: (query: string) => boolean;
    copySelection: () => Promise<boolean>;
  };
</script>

<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { FitAddon } from "@xterm/addon-fit";
  import { SearchAddon } from "@xterm/addon-search";
  import { WebLinksAddon } from "@xterm/addon-web-links";
  import { Terminal } from "@xterm/xterm";
  import "@xterm/xterm/css/xterm.css";

  let {
    active = false,
    fontSize = 15,
    theme,
    ondata,
    onnotice,
    onregisterApi,
    onresize,
  }: {
    active?: boolean;
    fontSize?: number;
    theme: RemotePtyTheme;
    ondata?: (data: string) => void;
    onnotice?: (message: string) => void;
    onregisterApi?: (api: RemotePtyApi | null) => void;
    onresize?: (size: { cols: number; rows: number }) => void;
  } = $props();

  let host = $state<HTMLDivElement>(null as any);

  const decoder = new TextDecoder();
  let term: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let searchAddon: SearchAddon | null = null;
  let longPressTimer: number | null = null;

  function base64UrlToBytes(data: string): Uint8Array {
    const normalized = data.replace(/-/g, "+").replace(/_/g, "/");
    const padded = normalized.padEnd(normalized.length + ((4 - (normalized.length % 4)) % 4), "=");
    const binary = atob(padded);
    return Uint8Array.from(binary, (char) => char.charCodeAt(0));
  }

  function getCurrentLineText() {
    if (!term) return "";
    const buffer = term.buffer.active;
    const line = buffer.getLine(buffer.baseY + buffer.cursorY);
    return line?.translateToString(true).trim() ?? "";
  }

  async function copySelection() {
    const text = term?.hasSelection() ? term.getSelection() : getCurrentLineText();
    if (!text) {
      onnotice?.("没有可复制的文本");
      return false;
    }

    try {
      await navigator.clipboard.writeText(text);
      onnotice?.("已复制终端文本");
      return true;
    } catch {
      onnotice?.("复制失败");
      return false;
    }
  }

  function fit() {
    fitAddon?.fit();
    if (term) {
      onresize?.({ cols: term.cols, rows: term.rows });
    }
  }

  function focus() {
    term?.focus();
  }

  function finish() {
    const trailing = decoder.decode();
    if (trailing) term?.write(trailing);
  }

  function findNext(query: string) {
    return query.trim() ? (searchAddon?.findNext(query) ?? false) : false;
  }

  function findPrevious(query: string) {
    return query.trim() ? (searchAddon?.findPrevious(query) ?? false) : false;
  }

  function writeBase64Url(data: string) {
    term?.write(decoder.decode(base64UrlToBytes(data), { stream: true }));
  }

  function writeText(data: string) {
    term?.write(data);
  }

  function clear() {
    term?.clear();
  }

  const api: RemotePtyApi = {
    writeBase64Url,
    writeText,
    clear,
    fit,
    focus,
    finish,
    findNext,
    findPrevious,
    copySelection,
  };

  function cancelLongPress() {
    if (longPressTimer) {
      window.clearTimeout(longPressTimer);
      longPressTimer = null;
    }
  }

  function startLongPress(event: PointerEvent) {
    if (event.pointerType === "mouse") return;
    cancelLongPress();
    longPressTimer = window.setTimeout(() => {
      void copySelection();
    }, 520);
  }

  $effect(() => {
    if (!term) return;
    term.options.fontSize = fontSize;
    term.options.theme = theme;
    queueMicrotask(() => fit());
  });

  $effect(() => {
    if (!active || !term) return;
    queueMicrotask(() => {
      fit();
      focus();
    });
  });

  onMount(() => {
    term = new Terminal({
      convertEol: true,
      cursorBlink: true,
      fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
      fontSize,
      theme,
      scrollback: 5000,
    });

    fitAddon = new FitAddon();
    searchAddon = new SearchAddon();
    term.loadAddon(fitAddon);
    term.loadAddon(searchAddon);
    term.loadAddon(
      new WebLinksAddon((_event, uri) => {
        window.open(uri, "_blank", "noopener,noreferrer");
      }),
    );
    term.open(host);
    fit();
    onregisterApi?.(api);

    const disposeData = term.onData((data) => ondata?.(data));
    const handleResize = () => {
      if (active) fit();
    };

    window.addEventListener("resize", handleResize);
    window.visualViewport?.addEventListener("resize", handleResize);

    if (active) focus();

    return () => {
      cancelLongPress();
      window.removeEventListener("resize", handleResize);
      window.visualViewport?.removeEventListener("resize", handleResize);
      disposeData.dispose();
      term?.dispose();
    };
  });

  onDestroy(() => {
    onregisterApi?.(null);
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class:active
  class="pane"
  style:display={active ? "block" : "none"}
  onpointerdown={startLongPress}
  onpointerup={cancelLongPress}
  onpointercancel={cancelLongPress}
  onpointerleave={cancelLongPress}
>
  <div bind:this={host} class="terminal-host"></div>
</div>

<style>
  .pane {
    height: 100%;
  }

  .pane.active {
    display: block;
  }

  .terminal-host {
    height: 100%;
    min-height: 48dvh;
    border-radius: 0.75rem;
    overflow: hidden;
  }
</style>
