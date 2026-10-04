import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { get } from 'svelte/store';

vi.mock('$app/environment', () => ({ browser: true }));

/**
 * The collapse state of manager boxes and inspector sections lives in localStorage. It must come
 * back after a reload, and a page whose storage is missing or throws must still work.
 */
describe('persistedSet', () => {
	let backing: Map<string, string>;

	beforeEach(() => {
		backing = new Map();
		vi.stubGlobal('localStorage', {
			getItem: (k: string) => backing.get(k) ?? null,
			setItem: (k: string, v: string) => void backing.set(k, v)
		});
		vi.resetModules();
	});
	afterEach(() => vi.unstubAllGlobals());

	it('restores what a previous page load saved', async () => {
		const first = await import('$lib/shared/stores/persisted-set');
		const store = first.persistedSet('k');
		first.toggleInSet(store, 'guest|identities');

		vi.resetModules();
		const second = await import('$lib/shared/stores/persisted-set');
		expect(get(second.persistedSet('k'))).toEqual(new Set(['guest|identities']));
	});

	it('toggling twice removes the entry', async () => {
		const { persistedSet, toggleInSet } = await import('$lib/shared/stores/persisted-set');
		const store = persistedSet('k');
		toggleInSet(store, 'Services');
		toggleInSet(store, 'Services');

		expect(get(store)).toEqual(new Set());
		expect(backing.get('k')).toBe('[]');
	});

	it('starts empty and keeps working when storage throws', async () => {
		vi.stubGlobal('localStorage', {
			getItem: () => {
				throw new Error('blocked');
			},
			setItem: () => {
				throw new Error('blocked');
			}
		});
		const { persistedSet, toggleInSet } = await import('$lib/shared/stores/persisted-set');
		const store = persistedSet('k');
		expect(get(store)).toEqual(new Set());

		toggleInSet(store, 'Services');
		expect(get(store)).toEqual(new Set(['Services']));
	});

	it('ignores a stored value of the wrong shape', async () => {
		backing.set('k', '{"not":"an array"}');
		const { persistedSet } = await import('$lib/shared/stores/persisted-set');

		expect(get(persistedSet('k'))).toEqual(new Set());
	});
});
