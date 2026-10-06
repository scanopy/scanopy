/**
 * Keyboard shortcuts as key chips show them. Apple platforms take ⌘ and spell keys as glyphs;
 * everywhere else takes Ctrl and spells keys out.
 */

/** This browser's platform string; empty outside a browser. */
export const currentPlatform = typeof navigator === 'undefined' ? '' : navigator.platform;

export function isApplePlatform(platform: string): boolean {
	return /mac|iphone|ipad/i.test(platform);
}

/**
 * One key as its chip shows it. `Mod` is the shortcut modifier: ⌘ on Apple platforms, Ctrl
 * elsewhere. Any other key passes through unchanged.
 */
export function keyLabel(key: string, platform = currentPlatform): string {
	const apple = isApplePlatform(platform);
	switch (key) {
		case 'Mod':
			return apple ? '⌘' : 'Ctrl';
		case 'Shift':
			return apple ? '⇧' : 'Shift';
		case 'Escape':
			return 'Esc';
		default:
			return key;
	}
}

/** A Mod shortcut as one key chip shows it: `⌘F` on Apple platforms, `Ctrl F` elsewhere. */
export function shortcutLabel(key: string, platform = currentPlatform): string {
	const mod = keyLabel('Mod', platform);
	return isApplePlatform(platform) ? `${mod}${key}` : `${mod} ${key}`;
}
