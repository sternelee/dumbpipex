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
</script>

<section class="home-shell">
  <div class="hero-card">
    <div class="hero-copy">
      <span class="eyebrow">dumbpipex</span>
      <h1>P2P remote terminal</h1>
      <p>先连接远程 agent，再进入专门的会话工作区操作终端。</p>
    </div>
    <div class="status-pill">{phaseLabel(sessionPhase)}</div>
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
          oninput={(event) => onTicketChange((event.currentTarget as HTMLTextAreaElement).value)}
          disabled={busy}
        ></textarea>
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
          onkeydown={(event) => event.key === "Enter" && !busy && ticket.trim() && onConnect()}
          disabled={busy}
        />
      </label>

      <button class="primary" onclick={onConnect} disabled={busy || !ticket.trim()}>
        {sessionPhase === "connecting" ? "Connecting..." : "Connect"}
      </button>

      <div class="meta">
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
    padding: 0.35rem 0.7rem;
    border-radius: 999px;
    background: rgba(148, 163, 184, 0.16);
    font-size: 0.875rem;
    white-space: nowrap;
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
  }

  button:disabled {
    opacity: 0.5;
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
