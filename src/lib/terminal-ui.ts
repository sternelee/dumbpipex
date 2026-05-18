import type { RemotePtyTheme } from "$lib/RemotePtyPane.svelte";

export type RemoteEvent =
  | { type: "disconnected"; reason?: string | null }
  | {
      type: "pty_created";
      pty_id: string;
      shell: string;
      cols: number;
      rows: number;
      resume_token: string;
      resumed: boolean;
    }
  | { type: "pty_output"; pty_id: string; data: string }
  | { type: "pty_exited"; pty_id: string; exit_code?: number | null }
  | { type: "error"; message: string };

export type PtyRecoveryInfo = {
  pty_id: string;
  shell: string;
  cols: number;
  rows: number;
  resume_token: string;
};

export type ConnectTicketResponse = {
  agent_name: string;
  label: string;
  sessions: PtyRecoveryInfo[];
};

export type SessionPhase = "idle" | "connecting" | "creating_pty" | "ready" | "disconnecting";

export type PtySession = {
  pty_id: string;
  shell: string;
  cols: number;
  rows: number;
  exited: boolean;
  exit_code?: number | null;
};

export type ShortcutButton = { label: string; data: string; hint?: string };
export type ShortcutSection = { title: string; rows: ShortcutButton[][] };
export type TerminalThemeOption = { id: string; label: string; theme: RemotePtyTheme };
export type SessionMode = "shell" | "vim" | "claude" | "pager" | "repl" | "monitor";

export const terminalThemes: TerminalThemeOption[] = [
  {
    id: "night",
    label: "Night",
    theme: {
      background: "#0f172a",
      foreground: "#e2e8f0",
      cursor: "#f8fafc",
      selectionBackground: "rgba(59, 130, 246, 0.35)",
      selectionInactiveBackground: "rgba(71, 85, 105, 0.35)",
    },
  },
  {
    id: "light",
    label: "Light",
    theme: {
      background: "#f8fafc",
      foreground: "#0f172a",
      cursor: "#1e293b",
      selectionBackground: "rgba(37, 99, 235, 0.25)",
      selectionInactiveBackground: "rgba(148, 163, 184, 0.25)",
    },
  },
  {
    id: "solarized",
    label: "Solarized",
    theme: {
      background: "#002b36",
      foreground: "#93a1a1",
      cursor: "#fdf6e3",
      selectionBackground: "rgba(181, 137, 0, 0.35)",
      selectionInactiveBackground: "rgba(88, 110, 117, 0.35)",
    },
  },
];

export const shortcutSections: ShortcutSection[] = [
  {
    title: "Essentials",
    rows: [
      [
        { label: "Esc", data: "\u001b" },
        { label: "Tab", data: "\t" },
        { label: "Enter", data: "\r" },
        { label: "Ctrl+C", data: "\u0003", hint: "stop" },
      ],
    ],
  },
  {
    title: "Shell",
    rows: [
      [
        { label: "Ctrl+A", data: "\u0001", hint: "line start" },
        { label: "Ctrl+E", data: "\u0005", hint: "line end" },
        { label: "Ctrl+W", data: "\u0017", hint: "delete word" },
        { label: "Ctrl+U", data: "\u0015", hint: "clear line" },
      ],
      [
        { label: "Ctrl+L", data: "\u000c", hint: "clear screen" },
        { label: "Ctrl+D", data: "\u0004", hint: "EOF" },
        { label: "Ctrl+Z", data: "\u001a", hint: "suspend" },
        { label: "Alt+B", data: "\u001bb", hint: "word back" },
      ],
      [{ label: "Alt+F", data: "\u001bf", hint: "word next" }],
    ],
  },
  {
    title: "Navigation",
    rows: [
      [
        { label: "\u2190", data: "\u001b[D" },
        { label: "\u2191", data: "\u001b[A" },
        { label: "\u2193", data: "\u001b[B" },
        { label: "\u2192", data: "\u001b[C" },
      ],
      [
        { label: "Home", data: "\u001b[H" },
        { label: "End", data: "\u001b[F" },
        { label: "PgUp", data: "\u001b[5~" },
        { label: "PgDn", data: "\u001b[6~" },
      ],
    ],
  },
  {
    title: "Vim",
    rows: [
      [
        { label: ":", data: ":" },
        { label: "/", data: "/" },
        { label: "gg", data: "gg", hint: "top" },
        { label: "G", data: "G", hint: "bottom" },
      ],
      [
        { label: "0", data: "0" },
        { label: "$", data: "$" },
        { label: "w", data: "w" },
        { label: "b", data: "b" },
      ],
    ],
  },
];

export const mobileModeShortcuts: Record<
  SessionMode,
  { label: string; data: string }[]
> = {
  shell: [
    { label: "Esc", data: "\u001b" },
    { label: "Tab", data: "\t" },
    { label: "Ctrl+C", data: "\u0003" },
    { label: "\u2191", data: "\u001b[A" },
    { label: "Enter", data: "\r" },
  ],
  vim: [
    { label: "Esc", data: "\u001b" },
    { label: ":w", data: ":w\r" },
    { label: ":q", data: ":q\r" },
    { label: ":wq", data: ":wq\r" },
    { label: "/", data: "/" },
  ],
  claude: [
    { label: "/help", data: "/help\r" },
    { label: "/clear", data: "/clear\r" },
    { label: "/compact", data: "/compact\r" },
    { label: "Ctrl+C", data: "\u0003" },
    { label: "Enter", data: "\r" },
  ],
  pager: [
    { label: "q", data: "q" },
    { label: "/", data: "/" },
    { label: "n", data: "n" },
    { label: "Space", data: " " },
    { label: "b", data: "b" },
  ],
  repl: [
    { label: "Tab", data: "\t" },
    { label: "\u2191", data: "\u001b[A" },
    { label: "Ctrl+C", data: "\u0003" },
    { label: "Ctrl+D", data: "\u0004" },
    { label: "Enter", data: "\r" },
  ],
  monitor: [
    { label: "q", data: "q" },
    { label: "k", data: "k" },
    { label: "Space", data: " " },
    { label: "Ctrl+C", data: "\u0003" },
    { label: "Enter", data: "\r" },
  ],
};

export function sessionModeLabel(mode: SessionMode) {
  switch (mode) {
    case "vim":
      return "Vim";
    case "claude":
      return "Claude CLI";
    case "pager":
      return "Pager";
    case "repl":
      return "REPL";
    case "monitor":
      return "Monitor";
    default:
      return "Shell";
  }
}
