<script lang="ts">
  import { onMount } from "svelte";
  import type { SessionPhase } from "$lib/terminal-ui";

  let {
    ticket = "",
    shell = "",
    status = "",
    sessionPhase = "idle" as SessionPhase,
    busy = false,
    viewerMode = false,
    onTicketChange = (_value: string) => {},
    onShellChange = (_value: string) => {},
    onViewerModeChange = (_value: boolean) => {},
    onConnect = () => {},
  }: {
    ticket?: string;
    shell?: string;
    status?: string;
    sessionPhase?: SessionPhase;
    busy?: boolean;
    viewerMode?: boolean;
    onTicketChange?: (value: string) => void;
    onShellChange?: (value: string) => void;
    onViewerModeChange?: (value: boolean) => void;
    onConnect?: () => void;
  } = $props();

  let ticketEl = $state<HTMLTextAreaElement | null>(null);

  function phaseLabel(phase: SessionPhase) {
    switch (phase) {
      case "connecting":
        return "Connecting";
      case "creating_pty":
        return "Starting PTY";
      case "disconnecting":
        return "Disconnecting";
      case "ready":
        return "Connected";
      default:
        return "Idle";
    }
  }

  /* The wire format is URL-safe base64 with no padding. */
  const TICKET_PATTERN = /^[A-Za-z0-9_-]+$/;

  const trimmedTicket = $derived(ticket.trim());
  const ticketLooksValid = $derived(
    trimmedTicket.length > 16 && TICKET_PATTERN.test(trimmedTicket),
  );
  const connectDisabled = $derived(busy || !ticketLooksValid);
  const showTicketHint = $derived(trimmedTicket.length > 0 && !ticketLooksValid);
  const isConnecting = $derived(
    sessionPhase === "connecting" || sessionPhase === "creating_pty",
  );

  // `autocorrect` is a non-standard iOS Safari/WebKit attribute and
  // not in svelte-html's HTMLAttributes, so we set it imperatively on
  // the bound node. On Android/desktop WebView it's a harmless no-op.
  onMount(() => {
    if (ticketEl) ticketEl.setAttribute("autocorrect", "off");
  });

  // Paste the clipboard into the ticket field. On phones, clipboard
  // permissions are tight: navigator.clipboard.readText() may reject
  // outside a user gesture, so we wrap the call in a button click.
  // Falls back to focusing the textarea so the user can long-press →
  // paste manually.
  let pasteHint = $state<string | null>(null);
  async function pasteFromClipboard() {
    if (!navigator.clipboard?.readText) {
      pasteHint = "此设备不支持自动粘贴，请长按输入框手动粘贴";
      ticketEl?.focus();
      return;
    }
    try {
      const text = await navigator.clipboard.readText();
      if (text) {
        onTicketChange(text);
        pasteHint = null;
      } else {
        pasteHint = "剪贴板为空";
      }
    } catch {
      pasteHint = "无法读取剪贴板，请长按输入框手动粘贴";
      ticketEl?.focus();
    }
  }
</script>

<section class="home-shell">
  <div class="hero-card">
    <div class="hero-copy">
      <span class="eyebrow">dumbpipex</span>
      <h1>P2P remote terminal</h1>
      <p>先连接远程 agent，再进入专门的会话工作区操作终端。</p>
    </div>
    <div
      class="status-pill"
      data-phase={sessionPhase}
      role="status"
      aria-live="polite"
      aria-label={`连接状态: ${phaseLabel(sessionPhase)}`}
    >
      {#if isConnecting}
        <span class="status-spinner" aria-hidden="true"></span>
      {:else}
        <span class="status-dot" aria-hidden="true"></span>
      {/if}
      {phaseLabel(sessionPhase)}
    </div>
  </div>

  <div class="home-grid">
    <section class="connect-card">
      <div class="section-heading">
        <h2>连接首页</h2>
        <p>粘贴 ticket 后连接远程 agent，连接成功会自动进入会话界面。</p>
      </div>

      <label class="field">
        <span>Agent ticket</span>
        <textarea
          bind:this={ticketEl}
          rows="5"
          placeholder="粘贴本地 dumbpipex-cli 输出的 ticket"
          value={ticket}
          autocapitalize="off"
          autocomplete="off"
          inputmode="text"
          spellcheck={false}
          enterkeyhint="done"
          aria-invalid={showTicketHint}
          aria-describedby="ticket-hint"
          oninput={(event) => onTicketChange((event.currentTarget as HTMLTextAreaElement).value)}
          disabled={busy}
        ></textarea>
        <div class="ticket-actions">
          <button
            type="button"
            class="ghost-btn"
            onclick={pasteFromClipboard}
            disabled={busy}
          >从剪贴板粘贴</button>
          {#if pasteHint}
            <span class="paste-hint">{pasteHint}</span>
          {/if}
        </div>
        <small id="ticket-hint" class="field-hint" class:error={showTicketHint}>
          {#if showTicketHint}
            Ticket 格式无效，应为 URL-safe base64 编码（仅 A-Z、a-z、0-9、_、-）
          {:else}
            通常为 100+ 字符的 base64 字符串
          {/if}
        </small>
      </label>

      <label class="field">
        <span>Shell override</span>
        <input
          value={shell}
          placeholder="默认使用远程 agent 配置的 shell"
          autocapitalize="off"
          autocomplete="off"
          spellcheck={false}
          enterkeyhint="done"
          oninput={(event) => onShellChange((event.currentTarget as HTMLInputElement).value)}
          onkeydown={(event) =>
            event.key === "Enter" && !busy && ticketLooksValid && onConnect()}
          disabled={busy}
        />
      </label>

      <label class="field viewer-toggle">
        <input
          type="checkbox"
          checked={viewerMode}
          onchange={(event) => onViewerModeChange((event.currentTarget as HTMLInputElement).checked)}
          disabled={busy}
        />
        <span class="viewer-label">
          <strong>只读连接（viewer）</strong>
          <small>只能查看现有 PTY 的输出，不能发送输入、调整大小或关闭会话。</small>
        </span>
      </label>

      <button
        class="primary"
        onclick={onConnect}
        disabled={connectDisabled}
        aria-busy={isConnecting}
      >
        {#if isConnecting}
          <span class="status-spinner" aria-hidden="true"></span>
          正在连接...
        {:else}
          连接
        {/if}
      </button>

      <div class="meta" aria-live="polite">
        <span><strong>Status:</strong> {status}</span>
      </div>
    </section>

    <section class="tips-card">
      <div class="section-heading">
        <h2>使用流程</h2>
        <p>连接首页只负责建立会话，终端与快捷操作全部留在下一屏。</p>
      </div>

      <ol>
        <li>在目标机器启动 `dumbpipex-cli` 并复制 ticket。</li>
        <li>在这里粘贴 ticket，按 Connect。</li>
        <li>连接建立后自动进入会话界面并创建第一个 PTY。</li>
      </ol>
    </section>
  </div>
</section>

<style>
  .home-shell {
    height: 100%;
    max-height: 100%;
    min-width: 0;
    display: grid;
    gap: 1rem;
    padding-top: max(1.5rem, calc(1rem + env(safe-area-inset-top)));
    padding-right: calc(1rem + env(safe-area-inset-right));
    padding-bottom: max(1rem, calc(0.5rem + env(safe-area-inset-bottom)));
    padding-left: calc(1rem + env(safe-area-inset-left));
    box-sizing: border-box;
    overflow-x: hidden;
    overflow-y: auto;
    overscroll-behavior: none;
    -webkit-overflow-scrolling: touch;
  }

  .hero-card,
  .connect-card,
  .tips-card {
    background: rgba(15, 23, 42, 0.88);
    border: 1px solid rgba(148, 163, 184, 0.18);
    border-radius: 1rem;
    box-shadow: 0 10px 40px rgba(15, 23, 42, 0.45);
  }

  .hero-card {
    padding: 1.1rem;
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
    min-width: 0;
  }

  .hero-copy {
    min-width: 0;
    overflow: hidden;
  }

  .hero-copy h1 {
    overflow-wrap: break-word;
    word-break: break-word;
    font-size: 1.6rem; /* explicit override of UA h1 default */
  }

  .hero-copy p {
    overflow-wrap: break-word;
    word-break: break-word;
    font-size: 0.95rem;
    line-height: 1.4;
  }

  .hero-copy h1,
  .section-heading h2 {
    margin: 0;
  }

  .hero-copy p,
  .section-heading p,
  li {
    color: #94a3b8;
  }

  .eyebrow {
    display: inline-block;
    margin-bottom: 0.55rem;
    color: #60a5fa;
    font-size: 0.85rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .status-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    padding: 0.35rem 0.7rem;
    border-radius: 999px;
    background: rgba(148, 163, 184, 0.16);
    font-size: 0.875rem;
    white-space: nowrap;
  }

  .status-dot {
    display: inline-block;
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 999px;
    background: #94a3b8;
  }

  .status-pill[data-phase="ready"] .status-dot {
    background: #34d399;
    box-shadow: 0 0 6px rgba(52, 211, 153, 0.5);
  }

  .status-pill[data-phase="connecting"] .status-dot,
  .status-pill[data-phase="creating_pty"] .status-dot,
  .status-pill[data-phase="disconnecting"] .status-dot {
    background: #facc15;
  }

  .status-spinner {
    display: inline-block;
    width: 0.85rem;
    height: 0.85rem;
    border-radius: 999px;
    border: 2px solid rgba(255, 255, 255, 0.18);
    border-top-color: #60a5fa;
    animation: status-spin 720ms linear infinite;
  }

  @keyframes status-spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .status-spinner {
      animation-duration: 1800ms;
    }
  }

  .home-grid {
    display: grid;
    gap: 1rem;
  }

  .connect-card,
  .tips-card {
    padding: 1rem;
    display: grid;
    gap: 1rem;
  }

  .section-heading {
    display: grid;
    gap: 0.35rem;
  }

  .field {
    display: grid;
    gap: 0.45rem;
  }

  .field span {
    color: #cbd5e1;
    font-size: 0.92rem;
  }

  textarea,
  input,
  button {
    border: 1px solid rgba(148, 163, 184, 0.24);
    border-radius: 0.9rem;
    background: rgba(15, 23, 42, 0.92);
    color: inherit;
    font: inherit;
  }

  textarea,
  input {
    padding: 0.85rem 0.9rem;
    resize: vertical;
  }

  button {
    padding: 0.9rem 1rem;
    font-weight: 600;
    min-height: 3rem;
    touch-action: manipulation;
  }

  @media (max-width: 680px) {
    button {
      min-height: 3rem; /* already 48px */
      font-size: 1rem;
    }

    textarea,
    input {
      min-height: 3rem;
      font-size: 16px; /* iOS: prevent zoom; also Android legibility */
    }

    /* .field-hint is overridden to 0.85rem in the dedicated block
       below; keep this slot empty so the cascade resolves once. */

    /* Status pill is the only at-a-glance health indicator on the
       connection home; bumping it to 0.95rem on phones keeps it
       readable without inflating the pill beyond a single line. */
    .status-pill {
      font-size: 0.95rem;
      padding: 0.45rem 0.85rem;
    }

    .meta {
      font-size: 0.95rem;
    }
  }

  button.primary {
    background: linear-gradient(135deg, #2563eb, #3b82f6);
    border-color: transparent;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
  }

  button:disabled {
    opacity: 0.5;
  }

  .field-hint {
    color: #64748b;
    font-size: 0.78rem;
    line-height: 1.35;
  }

  @media (max-width: 680px) {
    .field-hint {
      font-size: 0.85rem;
    }
  }

  .field-hint.error {
    color: #fca5a5;
  }

  .ticket-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
  }

  .ghost-btn {
    align-self: flex-start;
    padding: 0.45rem 0.7rem;
    border: 1px dashed rgba(148, 163, 184, 0.35);
    border-radius: 0.65rem;
    background: transparent;
    color: #cbd5e1;
    font: inherit;
    font-size: 0.82rem;
    font-weight: 500;
    min-height: 2.4rem;
    touch-action: manipulation;
    transition: border-color 140ms ease, color 140ms ease, background-color 140ms ease;
  }

  .ghost-btn:hover:not(:disabled) {
    border-color: rgba(96, 165, 250, 0.55);
    color: #e2e8f0;
    background: rgba(30, 41, 59, 0.6);
  }

  .ghost-btn:disabled {
    opacity: 0.4;
  }

  .paste-hint {
    color: #94a3b8;
    font-size: 0.8rem;
    line-height: 1.3;
  }

  textarea[aria-invalid="true"] {
    border-color: rgba(239, 68, 68, 0.5);
  }

  .meta {
    color: #94a3b8;
    font-size: 0.92rem;
  }

  /* The status string can be a long backend error message. Clamp to
     2 lines on phones so a multi-line exception trace doesn't push
     the rest of the card down; users can tap to see more if we add
     a detail view later. */
  @media (max-width: 680px) {
    .meta {
      font-size: 0.88rem;
      display: -webkit-box;
      -webkit-line-clamp: 2;
      line-clamp: 2;
      -webkit-box-orient: vertical;
      overflow: hidden;
    }
  }

  ol {
    margin: 0;
    padding-left: 1.25rem;
    display: grid;
    gap: 0.6rem;
  }

  @media (max-width: 899px) {
    .hero-card {
      flex-direction: column;
    }

    .status-pill {
      width: fit-content;
    }

    .connect-card,
    .tips-card {
      padding-bottom: calc(1rem + env(safe-area-inset-bottom));
    }
  }

  @media (max-width: 680px) {
    /* Compact the hero on phones so the Connect button sits in the
       thumb zone (upper 2/3 of the viewport), not below the fold. */
    .hero-card {
      padding: 0.85rem 0.95rem;
      gap: 0.7rem;
    }

    .hero-copy h1 {
      font-size: 1.3rem;
      line-height: 1.25;
    }

    .hero-copy p {
      font-size: 0.88rem;
      line-height: 1.4;
    }

    .eyebrow {
      margin-bottom: 0.35rem;
      font-size: 0.78rem;
    }

    .status-pill {
      font-size: 0.85rem;
      padding: 0.32rem 0.7rem;
    }

    .home-grid {
      gap: 0.75rem;
    }

    .connect-card,
    .tips-card {
      padding: 0.85rem 0.95rem;
    }

    .field {
      gap: 0.35rem;
    }

    .field span {
      font-size: 0.88rem;
    }
  }

  @media (min-width: 900px) {
    .home-shell {
      max-width: 1080px;
      margin: 0 auto;
      align-content: center;
    }

    .home-grid {
      grid-template-columns: minmax(0, 1.15fr) minmax(280px, 0.85fr);
    }
  }

  .viewer-toggle {
    display: flex;
    align-items: flex-start;
    gap: 0.6rem;
    padding: 0.6rem 0.7rem;
    border: 1px solid rgba(148, 163, 184, 0.18);
    border-radius: 0.6rem;
    background: rgba(15, 23, 42, 0.35);
  }

  .viewer-toggle input {
    margin-top: 0.18rem;
    width: 1rem;
    height: 1rem;
    accent-color: #3b82f6;
  }

  .viewer-label {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    font-size: 0.88rem;
  }

  .viewer-label small {
    color: rgba(148, 163, 184, 0.85);
    font-size: 0.75rem;
  }
</style>
