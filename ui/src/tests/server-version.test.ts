import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { writable } from 'svelte/store';

const modalState = writable<{ name: string | null }>({ name: null });

vi.mock('#lib/version', () => ({ VERSION: '0.19.3' }));
vi.mock('#lib/shared/stores/modal-registry.ts', () => ({ modalState }));

const reload = vi.fn();
let storage: Map<string, string>;

/** A fresh module per test: the pending flag and the modal wait are module state. */
async function load() {
	vi.resetModules();
	return import('#lib/api/server-version.ts');
}

describe('checkServerVersion', () => {
	beforeEach(() => {
		reload.mockClear();
		modalState.set({ name: null });
		storage = new Map();
		vi.stubGlobal('window', { location: { reload } });
		vi.stubGlobal('sessionStorage', {
			getItem: (k: string) => storage.get(k) ?? null,
			setItem: (k: string, v: string) => storage.set(k, v),
			removeItem: (k: string) => storage.delete(k)
		});
	});

	afterEach(() => {
		vi.unstubAllGlobals();
	});

	it('does nothing while the server runs the bundle’s version', async () => {
		const { checkServerVersion } = await load();
		checkServerVersion('0.19.3');
		expect(reload).not.toHaveBeenCalled();
	});

	it('reloads at once when the server changed and no modal is open', async () => {
		const { checkServerVersion } = await load();
		checkServerVersion('0.20.0');
		checkServerVersion('0.20.0');
		expect(reload).toHaveBeenCalledTimes(1);
	});

	it('waits for an open modal to close before reloading', async () => {
		const { checkServerVersion } = await load();
		modalState.set({ name: 'host-editor' });

		checkServerVersion('0.20.0');
		expect(reload).not.toHaveBeenCalled();

		modalState.set({ name: 'subnet-editor' });
		expect(reload).not.toHaveBeenCalled();

		modalState.set({ name: null });
		expect(reload).toHaveBeenCalledTimes(1);

		modalState.set({ name: 'host-editor' });
		modalState.set({ name: null });
		expect(reload).toHaveBeenCalledTimes(1);
	});

	it('stops when a reload did not bring the new version', async () => {
		const first = await load();
		first.checkServerVersion('0.20.0');
		expect(reload).toHaveBeenCalledTimes(1);

		// The reloaded page still carries the old bundle.
		const afterReload = await load();
		afterReload.checkServerVersion('0.20.0');
		expect(reload).toHaveBeenCalledTimes(1);

		// A later release gets its own reload.
		const later = await load();
		later.checkServerVersion('0.21.0');
		expect(reload).toHaveBeenCalledTimes(2);
	});
});
