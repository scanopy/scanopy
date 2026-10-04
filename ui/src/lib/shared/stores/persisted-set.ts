import { writable, type Writable } from 'svelte/store';
import { browser } from '$app/environment';

/**
 * A `Set<string>` store mirrored to localStorage under `key`, for per-viewer UI state such as
 * which boxes or sections the viewer collapsed.
 *
 * Storage can be missing or throw (private windows, blocked site data), so every read and write
 * is guarded and the store falls back to an empty set: losing the remembered state is acceptable,
 * breaking the page is not.
 */
export function persistedSet(key: string): Writable<Set<string>> {
	const store = writable<Set<string>>(load(key));
	if (browser) {
		let initialized = false;
		store.subscribe((value) => {
			// The first call replays the loaded value; writing it back would be a no-op.
			if (initialized) save(key, value);
			initialized = true;
		});
	}
	return store;
}

/** Add `id` if absent, remove it if present. */
export function toggleInSet(store: Writable<Set<string>>, id: string): void {
	store.update((set) => {
		const next = new Set(set);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		return next;
	});
}

function load(key: string): Set<string> {
	if (!browser) return new Set();
	try {
		const parsed: unknown = JSON.parse(localStorage.getItem(key) ?? '[]');
		return Array.isArray(parsed)
			? new Set(parsed.filter((v): v is string => typeof v === 'string'))
			: new Set();
	} catch {
		return new Set();
	}
}

function save(key: string, value: Set<string>): void {
	try {
		localStorage.setItem(key, JSON.stringify([...value]));
	} catch {
		// Storage unavailable; the state lives for this session only.
	}
}
