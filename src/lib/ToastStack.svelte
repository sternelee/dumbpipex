<script lang="ts" module>
  export type ToastKind = "info" | "success" | "warning" | "error";

  export type Toast = {
    id: number;
    kind: ToastKind;
    message: string;
    /** ms until auto-dismiss. 0 means sticky. */
    durationMs: number;
  };

  let nextId = 1;
  let toasts = $state<Toast[]>([]);
  const timers = new Map<number, ReturnType<typeof setTimeout>>();

  function dismiss(id: number) {
    toasts = toasts.filter((t) => t.id !== id);
    const timer = timers.get(id);
    if (timer) {
      clearTimeout(timer);
      timers.delete(id);
    }
  }

  export function toast(message: string, kind: ToastKind = "info", durationMs = 3200) {
    const id = nextId++;
    toasts = [...toasts, { id, kind, message, durationMs }];
    if (durationMs > 0) {
      timers.set(
        id,
        setTimeout(() => dismiss(id), durationMs),
      );
    }
    return id;
  }

  export function dismissToast(id: number) {
    dismiss(id);
  }
</script>

<script lang="ts">
  function iconFor(kind: ToastKind): string {
    switch (kind) {
      case "success":
        return "✓";
      case "warning":
        return "!";
      case "error":
        return "✕";
      default:
        return "i";
    }
  }
</script>

<div
  class="toast-stack"
  role="region"
  aria-live="polite"
  aria-label="通知"
>
  {#each toasts as t (t.id)}
    <div
      class="toast"
      data-kind={t.kind}
      role={t.kind === "error" ? "alert" : "status"}
      aria-label={t.kind === "error"
        ? `错误: ${t.message}`
        : t.kind === "warning"
          ? `警告: ${t.message}`
          : t.kind === "success"
            ? `成功: ${t.message}`
            : `通知: ${t.message}`}
    >
      <span class="toast-icon" aria-hidden="true">{iconFor(t.kind)}</span>
      <span class="toast-message">{t.message}</span>
      <button
        class="toast-close"
        type="button"
        aria-label="关闭通知"
        onclick={() => dismiss(t.id)}
      >×</button>
    </div>
  {/each}
</div>

<style>
  .toast-stack {
    position: fixed;
    top: calc(env(safe-area-inset-top) + 0.5rem);
    left: 50%;
    transform: translateX(-50%);
    z-index: 200;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    max-width: min(90vw, 30rem);
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.65rem 0.85rem;
    border-radius: 0.85rem;
    background: rgba(15, 23, 42, 0.96);
    border: 1px solid rgba(148, 163, 184, 0.25);
    box-shadow: 0 14px 36px rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(16px);
    color: #e2e8f0;
    font-size: 0.85rem;
    line-height: 1.3;
    pointer-events: auto;
    animation: toast-in 180ms ease both;
  }

  .toast[data-kind="success"] {
    border-color: rgba(34, 197, 94, 0.45);
    background: rgba(6, 78, 59, 0.92);
  }

  .toast[data-kind="warning"] {
    border-color: rgba(234, 179, 8, 0.5);
    background: rgba(113, 63, 18, 0.92);
  }

  .toast[data-kind="error"] {
    border-color: rgba(239, 68, 68, 0.55);
    background: rgba(127, 29, 29, 0.92);
  }

  .toast-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.4rem;
    height: 1.4rem;
    flex-shrink: 0;
    border-radius: 999px;
    font-size: 0.78rem;
    font-weight: 700;
    background: rgba(255, 255, 255, 0.1);
  }

  .toast[data-kind="success"] .toast-icon {
    background: rgba(34, 197, 94, 0.25);
    color: #bbf7d0;
  }

  .toast[data-kind="error"] .toast-icon {
    background: rgba(239, 68, 68, 0.25);
    color: #fecaca;
  }

  .toast[data-kind="warning"] .toast-icon {
    background: rgba(234, 179, 8, 0.25);
    color: #fde68a;
  }

  .toast-message {
    flex: 1 1 auto;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .toast-close {
    flex-shrink: 0;
    border: none;
    background: transparent;
    color: inherit;
    opacity: 0.6;
    font-size: 1rem;
    line-height: 1;
    padding: 0.25rem 0.4rem;
    border-radius: 0.4rem;
    cursor: pointer;
    transition: opacity 120ms ease, background-color 120ms ease;
    min-width: 2rem;  /* 32px */
    min-height: 2rem;
  }

  .toast-close:hover {
    opacity: 1;
    background: rgba(255, 255, 255, 0.1);
  }

  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(-8px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .toast {
      animation: none;
    }
  }

  @media (max-width: 680px) {
    /* Close button bump from 32px to 44px on phones; the toast itself
       is centered and the close × is in the corner of the thumb zone,
       where a 32px target is hard to land reliably. */
    .toast-close {
      min-width: 2.75rem; /* 44px */
      min-height: 2.75rem;
      font-size: 1.1rem;
    }
  }
</style>
