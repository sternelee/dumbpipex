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
