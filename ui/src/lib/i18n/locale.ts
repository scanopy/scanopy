import { getLocale, overwriteGetLocale } from '#lib/paraglide/runtime.js';

/**
 * Resolve the locale once and serve every later `getLocale()` from that value.
 *
 * Paraglide's own `getLocale()` reads `document.cookie` and compiles a RegExp on each call, and
 * every message function calls it. A 3,500-node topology spent 0.9s of its first draw there.
 * `setLocale()` reloads the page by default, and nothing here calls it with `reload: false`, so
 * the value resolved at boot holds for the page's lifetime.
 */
export function cacheLocale(): void {
	const locale = getLocale();
	overwriteGetLocale(() => locale);
}
