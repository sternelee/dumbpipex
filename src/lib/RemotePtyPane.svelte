<script lang="ts">
  import type { RemotePtyApi, RemotePtyTheme } from "$lib/remote-pty-types";
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
    flexBasisPct,
    ondata,
    onnotice,
    onregisterApi,
    onresize,
  }: {
    active?: boolean;
    fontSize?: number;
    theme: RemotePtyTheme;
    flexBasisPct?: number;
    ondata?: (data: string) => void;
    onnotice?: (message: string) => void;
    onregisterApi?: (api: RemotePtyApi | null) => void;
    onresize?: (size: { cols: number; rows: number }) => void;
  } = $props();

  let host = $state<HTMLDivElement>(null as any);
  let mobileInput = $state<HTMLTextAreaElement>(null as any);

  const decoder = new TextDecoder();
  let term = $state<Terminal | null>(null);
  let fitAddon: FitAddon | null = null;
  let searchAddon: SearchAddon | null = null;
  let longPressTimer: number | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let focused = $state(false);

  /* ── detect if we're on mobile WebView ── */
  function isMobileWebView(): boolean {
    if (typeof navigator === "undefined") return false;
    const ua = navigator.userAgent.toLowerCase();
    return /iphone|ipad|android/.test(ua);
  }

  /* ── mobile input bridge ── */

  function handleMobileInput() {
    const el = mobileInput;
    if (!el || !term) return;
    const text = el.value;
    if (!text) return;
    // Use Array.from to iterate over Unicode code points (handles emoji, CJK)
    for (const char of Array.from(text)) {
      term.input(char);
    }
    el.value = "";
  }


  function handleMobileKeyDown(e: KeyboardEvent) {
    if (!term) return;
    if (e.key === "Enter" || e.key === "Backspace" || e.key === "Tab" ||
        e.key === "Escape" || e.key.startsWith("Arrow") ||
        e.ctrlKey || e.metaKey || e.altKey) {
      e.preventDefault();
      (term as any)._core._keyDown(e);
    }
  }

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
    if (!active || !term) return;
    fitAddon?.fit();
    if (term) {
      onresize?.({ cols: term.cols, rows: term.rows });
    }
  }

  function focus() {
    if (isMobileWebView()) {
      mobileInput?.focus();
    } else {
      term?.focus();
    }
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
    focus();
    if (event.pointerType === "mouse") return;
    cancelLongPress();
    longPressTimer = window.setTimeout(() => {
      void copySelection();
    }, 520);
  }

  $effect(() => {
    const t = term;
    if (!t) return;
    t.options.fontSize = fontSize;
    t.options.theme = { ...theme };
    if (t.rows > 0) t.refresh(0, t.rows - 1);
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
      cursorStyle: "bar",
      cursorWidth: 2,
      fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
      fontSize,
      lineHeight: 1.2,
      theme,
      scrollback: 5000,
      smoothScrollDuration: 60,
      allowProposedApi: true,
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

    if ("ResizeObserver" in window) {
      resizeObserver = new ResizeObserver(() => {
        if (active) fit();
      });
      resizeObserver.observe(host);
    }

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
      resizeObserver?.disconnect();
      disposeData.dispose();
      term?.dispose();
    };
  });

  onDestroy(() => {
    onregisterApi?.(null);
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class:active
  class="pane"
  class:focused={focused}
  style:display={active ? "flex" : "none"}
  style:flex-basis={flexBasisPct != null ? `${flexBasisPct}%` : null}
  onpointerdown={(e) => {
    startLongPress(e);
    if (!focused) {
      focused = true;
      focus();
    }
  }}
  onpointerup={cancelLongPress}
  onpointercancel={cancelLongPress}
  onpointerleave={cancelLongPress}
  onclick={() => {
    if (!focused) {
      focused = true;
      focus();
    }
  }}
  role="application"
  aria-label="remote terminal"
  tabindex="0"
>
  <div bind:this={host} class="terminal-host"></div>
  <!-- Hidden textarea bridge for mobile keyboard input.
     xterm.js already has its own hidden textarea for input; this
     extra bridge exists so we can call .focus() on it from JS
     (some WebView keyboards ignore the xterm internal textarea's
     blur/focus sequence). It MUST stay pointer-events: none and
     unfocusable so touches fall through to the xterm canvas and
     long-press text selection keeps working. -->
  <textarea
    bind:this={mobileInput}
    class="mobile-input-bridge"
    tabindex="-1"
    aria-hidden="true"
    autocomplete="off"
    autocapitalize="off"
    spellcheck="false"
    oninput={handleMobileInput}
    onkeydown={handleMobileKeyDown}
  ></textarea>
</div>

<style>
  .pane {
    flex: 1 1 auto;
    min-height: 0;
    min-width: 0;
    display: none;
    position: relative;
    touch-action: manipulation;
    -webkit-tap-highlight-color: transparent;
    overflow: hidden;
    border-radius: 0.75rem;
    outline: 2px solid transparent;
    outline-offset: -2px;
    transition: outline-color 180ms ease, box-shadow 180ms ease;
  }

  .pane.active {
    display: flex;
  }

  .pane.focused {
    box-shadow: 0 0 0 1px rgba(59, 130, 246, 0.4);
  }

  /* On a phone the active pane is always the one filling the
     viewport, so a "focused" outline carries no information and just
     clips the terminal by 1px. Desktop keeps the ring for mouse
     keyboard navigation. */
  @media (max-width: 680px) {
    .pane.focused {
      box-shadow: none;
    }
  }

  .terminal-host {
    flex: 1 1 auto;
    min-height: 0;
    min-width: 0;
    width: 100%;
    border-radius: 0.75rem;
    overflow: hidden;
    overscroll-behavior: contain;
  }

  /* Hidden textarea bridge for mobile keyboard input.
     xterm.js has its own hidden textarea; this is a *second*
     bridge we only focus() to force the virtual keyboard open
     in tricky WebView situations. It must NOT intercept
     pointer events — that would block all touches to the
     xterm canvas and break long-press text selection. */
  .mobile-input-bridge {
    position: absolute;
    left: 0;
    top: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
    background: transparent;
    border: none;
    outline: none;
    resize: none;
    font-size: 16px;
    color: transparent;
    caret-color: transparent;
    -webkit-user-select: none;
    user-select: none;
    pointer-events: none;
    z-index: -1;
    overscroll-behavior: contain;
  }
</style>
