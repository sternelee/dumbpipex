import type { RemotePtyTheme } from "$lib/remote-pty-types";

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

export type SessionPhase =
	| "idle"
	| "connecting"
	| "creating_pty"
	| "ready"
	| "disconnecting";

export type PtySession = {
	pty_id: string;
	shell: string;
	cols: number;
	rows: number;
	exited: boolean;
	exit_code?: number | null;
};

export type ShortcutButton = { label: string; data: string; hint?: string };
export type MobileShortcutButton = ShortcutButton & { repeatable?: boolean };
export type MobilePlatform = "ios" | "android" | "other";
export type StickyModifier = "ctrl" | "alt" | "esc" | "shift";
export type ShortcutSection = { title: string; rows: ShortcutButton[][] };
export type TerminalThemeOption = {
	id: string;
	label: string;
	theme: RemotePtyTheme;
};
export type SessionMode =
	| "shell"
	| "vim"
	| "claude"
	| "pager"
	| "repl"
	| "monitor";

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

export const mobileModeShortcuts: Record<SessionMode, MobileShortcutButton[]> =
	{
		shell: [
			{ label: "Esc", data: "\u001b" },
			{ label: "Tab", data: "\t", repeatable: true },
			{ label: "⌫", data: "\u007f", repeatable: true },
			{ label: "Ctrl+C", data: "\u0003" },
			{ label: "Ctrl+L", data: "\u000c" },
			{ label: "Ctrl+A", data: "\u0001" },
			{ label: "Ctrl+E", data: "\u0005" },
			{ label: "Ctrl+W", data: "\u0017" },
			{ label: "Ctrl+U", data: "\u0015" },
			{ label: "Home", data: "\u001b[H" },
			{ label: "End", data: "\u001b[F" },
			{ label: "\u2190", data: "\u001b[D", repeatable: true },
			{ label: "\u2191", data: "\u001b[A", repeatable: true },
			{ label: "\u2193", data: "\u001b[B", repeatable: true },
			{ label: "\u2192", data: "\u001b[C", repeatable: true },
			{ label: "Enter", data: "\r", repeatable: true },
		],
		vim: [
			{ label: "Esc", data: "\u001b" },
			{ label: "⌫", data: "\u007f", repeatable: true },
			{ label: "Enter", data: "\r", repeatable: true },
			{ label: "/", data: "/" },
			{ label: ":", data: ":" },
			{ label: ":w", data: ":w\r" },
			{ label: ":q", data: ":q\r" },
			{ label: "gg", data: "gg" },
			{ label: "G", data: "G" },
			{ label: "0", data: "0" },
			{ label: "$", data: "$" },
			{ label: "w", data: "w", repeatable: true },
			{ label: "b", data: "b", repeatable: true },
		],
		claude: [
			{ label: "Esc", data: "\u001b" },
			{ label: "Tab", data: "\t", repeatable: true },
			{ label: "⌫", data: "\u007f", repeatable: true },
			{ label: "/help", data: "/help\r" },
			{ label: "/clear", data: "/clear\r" },
			{ label: "/compact", data: "/compact\r" },
			{ label: "Ctrl+C", data: "\u0003" },
			{ label: "Ctrl+L", data: "\u000c" },
			{ label: "\u2191", data: "\u001b[A", repeatable: true },
			{ label: "\u2193", data: "\u001b[B", repeatable: true },
			{ label: "Enter", data: "\r", repeatable: true },
		],
		pager: [
			{ label: "q", data: "q" },
			{ label: "/", data: "/" },
			{ label: "n", data: "n", repeatable: true },
			{ label: "Space", data: " ", repeatable: true },
			{ label: "b", data: "b", repeatable: true },
			{ label: "PgUp", data: "\u001b[5~", repeatable: true },
			{ label: "PgDn", data: "\u001b[6~", repeatable: true },
			{ label: "\u2191", data: "\u001b[A", repeatable: true },
			{ label: "\u2193", data: "\u001b[B", repeatable: true },
			{ label: "Enter", data: "\r", repeatable: true },
		],
		repl: [
			{ label: "Tab", data: "\t", repeatable: true },
			{ label: "⌫", data: "\u007f", repeatable: true },
			{ label: "Ctrl+C", data: "\u0003" },
			{ label: "Ctrl+D", data: "\u0004" },
			{ label: "Ctrl+A", data: "\u0001" },
			{ label: "Ctrl+E", data: "\u0005" },
			{ label: "\u2190", data: "\u001b[D", repeatable: true },
			{ label: "\u2191", data: "\u001b[A", repeatable: true },
			{ label: "\u2193", data: "\u001b[B", repeatable: true },
			{ label: "\u2192", data: "\u001b[C", repeatable: true },
			{ label: "Enter", data: "\r", repeatable: true },
		],
		monitor: [
			{ label: "q", data: "q" },
			{ label: "k", data: "k", repeatable: true },
			{ label: "j", data: "j", repeatable: true },
			{ label: "Space", data: " ", repeatable: true },
			{ label: "Ctrl+C", data: "\u0003" },
			{ label: "PgUp", data: "\u001b[5~", repeatable: true },
			{ label: "PgDn", data: "\u001b[6~", repeatable: true },
			{ label: "\u2191", data: "\u001b[A", repeatable: true },
			{ label: "\u2193", data: "\u001b[B", repeatable: true },
			{ label: "Enter", data: "\r", repeatable: true },
		],
	};

/** Special characters always available in mobile shortcut bar */
export const specialCharGrid: MobileShortcutButton[] = [
	{ label: "|", data: "|" },
	{ label: "\\", data: "\\" },
	{ label: "?", data: "?" },
	{ label: "-", data: "-" },
	{ label: ":", data: ":" },
	{ label: ";", data: ";" },
	{ label: "~", data: "~" },
	{ label: "@", data: "@" },
	{ label: "#", data: "#" },
	{ label: "$", data: "$" },
	{ label: "%", data: "%" },
	{ label: "^", data: "^" },
	{ label: "&", data: "&" },
	{ label: "*", data: "*" },
	{ label: "(", data: "(" },
	{ label: ")", data: ")" },
	{ label: "[", data: "[" },
	{ label: "]", data: "]" },
	{ label: "{", data: "{" },
	{ label: "}", data: "}" },
	{ label: "<", data: "<" },
	{ label: ">", data: ">" },
	{ label: "/", data: "/" },
	{ label: ".", data: "." },
	{ label: ",", data: "," },
	{ label: "!", data: "!" },
	{ label: "=", data: "=" },
	{ label: "+", data: "+" },
	{ label: "'", data: "'" },
	{ label: '"', data: '"' },
	{ label: "`", data: "`" },
];

const mobileShortcutPriority: Record<MobilePlatform, string[]> = {
	ios: [
		"Esc",
		"Tab",
		"⌫",
		"←",
		"↑",
		"↓",
		"→",
		"Enter",
		"Ctrl+C",
		"Ctrl+L",
		"Ctrl+A",
		"Ctrl+E",
		"Ctrl+W",
		"Ctrl+U",
		"Home",
		"End",
		"PgUp",
		"PgDn",
		"/help",
		"/clear",
		"/compact",
		":",
		"/",
		":w",
		":q",
		"gg",
		"G",
		"0",
		"$",
		"w",
		"b",
		"q",
		"Space",
		"n",
		"Ctrl+D",
		"k",
		"j",
	],
	android: [
		"Ctrl+C",
		"Esc",
		"Tab",
		"⌫",
		"↑",
		"↓",
		"←",
		"→",
		"Enter",
		"Ctrl+L",
		"Ctrl+A",
		"Ctrl+E",
		"Ctrl+W",
		"Ctrl+U",
		"Home",
		"End",
		"PgUp",
		"PgDn",
		"/help",
		"/clear",
		"/compact",
		":",
		"/",
		":w",
		":q",
		"gg",
		"G",
		"0",
		"$",
		"w",
		"b",
		"q",
		"Space",
		"n",
		"Ctrl+D",
		"k",
		"j",
	],
	other: [],
};

export function prioritizeMobileShortcuts(
	shortcuts: MobileShortcutButton[],
	platform: MobilePlatform,
) {
	const priority = mobileShortcutPriority[platform];
	if (priority.length === 0) return shortcuts;

	const ranking = new Map(priority.map((label, index) => [label, index]));
	return [...shortcuts].sort((left, right) => {
		const leftRank = ranking.get(left.label) ?? Number.MAX_SAFE_INTEGER;
		const rightRank = ranking.get(right.label) ?? Number.MAX_SAFE_INTEGER;
		return leftRank - rightRank;
	});
}

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

export function mobilePlatformLabel(platform: MobilePlatform) {
	switch (platform) {
		case "ios":
			return "iPhone / iPad";
		case "android":
			return "Android";
		default:
			return "Mobile";
	}
}

export function mobileShortcutHint(mode: SessionMode) {
	switch (mode) {
		case "shell":
			return "长按修饰键锁定 · 可切换极简 / 展开";
		case "vim":
			return "长按修饰键锁定 · 可切换极简 / 展开";
		case "claude":
			return "长按修饰键锁定 · 可切换极简 / 展开";
		case "pager":
			return "长按修饰键锁定 · 可切换极简 / 展开";
		case "repl":
			return "长按修饰键锁定 · 可切换极简 / 展开";
		case "monitor":
			return "长按修饰键锁定 · 可切换极简 / 展开";
		default:
			return "长按修饰键锁定 · 按住方向键、⌫、Enter 可连发";
	}
}
