<script lang="ts">
  import type { SessionPhase } from "$lib/terminal-ui";

  let {
    ticket,
    shell,
    status,
    sessionPhase,
    busy,
    onTicketChange,
    onShellChange,
    onConnect,
  }: {
    ticket: string;
    shell: string;
    status: string;
    sessionPhase: SessionPhase;
    busy: boolean;
    onTicketChange: (value: string) => void;
    onShellChange: (value: string) => void;
    onConnect: () => void;
  } = $props();

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
</script>

<section class="home-shell">
  <div class="hero-card">
    <div class="hero-copy">
      <span class="eyebrow">dumbpipex</span>
      <h1>P2P remote terminal</h1>
      <p>先连接远程 agent，再进入专门的会话工作区操作终端。</p>
    </div>
    <div class="status-pill" data-phase={sessionPhase}>
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
          rows="5"
          placeholder="粘贴本地 dumbpipex-cli 输出的 ticket"
          value={ticket}
          autocapitalize="off"
          autocomplete="off"
          spellcheck={false}
          enterkeyhint="done"
          aria-invalid={showTicketHint}
          aria-describedby="ticket-hint"
          oninput={(event) => onTicketChange((event.currentTarget as HTMLTextAreaElement).value)}
          disabled={busy}
        ></textarea>
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
          Connect
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
  }

  .hero-copy p {
    overflow-wrap: break-word;
    word-break: break-word;
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

  .field-hint.error {
    color: #fca5a5;
  }

  textarea[aria-invalid="true"] {
    border-color: rgba(239, 68, 68, 0.5);
  }

  .meta {
    color: #94a3b8;
    font-size: 0.92rem;
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
</style>
