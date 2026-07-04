const STORAGE_KEY = 'scanopy-theme';

export type ThemeMode = 'system' | 'light' | 'dark';
export type ResolvedTheme = 'light' | 'dark';

let themeMode = $state<ThemeMode>('system');
let systemPrefersDark = $state(
	typeof window !== 'undefined' ? window.matchMedia('(prefers-color-scheme: dark)').matches : true
);

const resolvedTheme = $derived<ResolvedTheme>(
	themeMode === 'system' ? (systemPrefersDark ? 'dark' : 'light') : themeMode
);

// Initialize from URL / localStorage and set up listeners (browser only)
if (typeof window !== 'undefined') {
	// n9e 嵌入:iframe 的 ?theme=light|dark 优先(跟随 n9e 明暗)。
	// 嵌入但没带 theme 参数时,默认浅色(n9e 默认浅色)——不跟 OS,避免嵌入区意外变黑。
	// 非嵌入(原生)才按 localStorage / system。
	const urlTheme = new URLSearchParams(window.location.search).get('theme');
	const embed = !!(window as unknown as { __SCANOPY_EMBED__?: boolean }).__SCANOPY_EMBED__;
	const stored = localStorage.getItem(STORAGE_KEY) as ThemeMode | null;
	if (urlTheme === 'light' || urlTheme === 'dark') {
		themeMode = urlTheme;
	} else if (embed) {
		themeMode = 'light';
	} else if (stored === 'light' || stored === 'dark' || stored === 'system') {
		themeMode = stored;
	}

	// Update reactive state when OS preference changes
	window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
		systemPrefersDark = e.matches;
	});

	// Apply theme to DOM whenever resolvedTheme changes
	$effect.root(() => {
		$effect(() => {
			document.documentElement.classList.toggle('dark', resolvedTheme === 'dark');
			document.documentElement.style.colorScheme = resolvedTheme;
		});
	});
}

export function setTheme(mode: ThemeMode) {
	themeMode = mode;
	localStorage.setItem(STORAGE_KEY, mode);
}

export function getThemeMode(): ThemeMode {
	return themeMode;
}

export function getResolvedTheme(): ResolvedTheme {
	return resolvedTheme;
}

export const themeStore = {
	get themeMode() {
		return themeMode;
	},
	get resolvedTheme() {
		return resolvedTheme;
	},
	setTheme
};
