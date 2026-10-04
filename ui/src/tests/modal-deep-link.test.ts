import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { get } from 'svelte/store';
import {
	closeModal,
	modalState,
	navigateToEntity,
	resolveModalDeepLink
} from '$lib/shared/stores/modal-registry';
import type { ModalState } from '$lib/shared/stores/modal-registry';

interface TestEntity {
	id: string;
	name: string;
}

const entities: TestEntity[] = [
	{ id: 'aaa', name: 'Alpha' },
	{ id: 'bbb', name: 'Bravo' },
	{ id: 'ccc', name: 'Charlie' }
];

function state(
	name: string | null,
	id: string | null = null,
	entityData?: Record<string, unknown>
): ModalState {
	return { name, id, tab: null, subEntityId: null, returnUrl: null, returnTitle: null, entityData };
}

describe('resolveModalDeepLink', () => {
	it('returns undefined when modal name does not match', () => {
		const result = resolveModalDeepLink(
			state('other-modal', 'aaa'),
			'my-modal',
			entities,
			false,
			null
		);
		expect(result).toBeUndefined();
	});

	it('returns undefined when state name is null', () => {
		const result = resolveModalDeepLink(state(null), 'my-modal', entities, false, null);
		expect(result).toBeUndefined();
	});

	it('returns null for create mode (no id, modal closed)', () => {
		const result = resolveModalDeepLink(state('my-modal'), 'my-modal', entities, false, null);
		expect(result).toBeNull();
	});

	it('returns entity when id matches and modal is closed', () => {
		const result = resolveModalDeepLink(
			state('my-modal', 'bbb'),
			'my-modal',
			entities,
			false,
			null
		);
		expect(result).toEqual({ id: 'bbb', name: 'Bravo' });
	});

	it('returns undefined when id not found in data (data not loaded yet)', () => {
		const result = resolveModalDeepLink(
			state('my-modal', 'zzz'),
			'my-modal',
			entities,
			false,
			null
		);
		expect(result).toBeUndefined();
	});

	it('returns undefined when id not found in empty data array', () => {
		const result = resolveModalDeepLink(state('my-modal', 'aaa'), 'my-modal', [], false, null);
		expect(result).toBeUndefined();
	});

	it('returns entity for entity switch (modal open, different id)', () => {
		const result = resolveModalDeepLink(
			state('my-modal', 'ccc'),
			'my-modal',
			entities,
			true,
			'aaa'
		);
		expect(result).toEqual({ id: 'ccc', name: 'Charlie' });
	});

	it('returns undefined when already editing the same entity', () => {
		const result = resolveModalDeepLink(
			state('my-modal', 'aaa'),
			'my-modal',
			entities,
			true,
			'aaa'
		);
		expect(result).toBeUndefined();
	});

	it('returns undefined when modal is open with no id (create mode already open)', () => {
		const result = resolveModalDeepLink(state('my-modal'), 'my-modal', entities, true, null);
		expect(result).toBeUndefined();
	});

	describe('validate callback', () => {
		const alwaysFail = () => false;
		const alwaysPass = () => true;

		it('returns entity when validate passes', () => {
			const result = resolveModalDeepLink(
				state('my-modal', 'aaa'),
				'my-modal',
				entities,
				false,
				null,
				alwaysPass
			);
			expect(result).toEqual({ id: 'aaa', name: 'Alpha' });
		});

		it('returns undefined when validate fails (modal closed)', () => {
			const result = resolveModalDeepLink(
				state('my-modal', 'aaa'),
				'my-modal',
				entities,
				false,
				null,
				alwaysFail
			);
			expect(result).toBeUndefined();
		});

		it('returns undefined when validate fails during entity switch', () => {
			const result = resolveModalDeepLink(
				state('my-modal', 'bbb'),
				'my-modal',
				entities,
				true,
				'aaa',
				alwaysFail
			);
			expect(result).toBeUndefined();
		});

		it('returns entity when validate passes during entity switch', () => {
			const result = resolveModalDeepLink(
				state('my-modal', 'bbb'),
				'my-modal',
				entities,
				true,
				'aaa',
				alwaysPass
			);
			expect(result).toEqual({ id: 'bbb', name: 'Bravo' });
		});

		it('does not call validate for create mode', () => {
			const result = resolveModalDeepLink(
				state('my-modal'),
				'my-modal',
				entities,
				false,
				null,
				alwaysFail
			);
			expect(result).toBeNull();
		});
	});

	describe('entityData fallback', () => {
		const fallbackEntity = { id: 'zzz', name: 'Zulu' };

		it('uses entityData when entity not in data array', () => {
			const result = resolveModalDeepLink(
				state('my-modal', 'zzz', fallbackEntity),
				'my-modal',
				entities,
				false,
				null
			);
			expect(result).toEqual(fallbackEntity);
		});

		it('data array takes precedence over entityData', () => {
			const result = resolveModalDeepLink(
				state('my-modal', 'aaa', { id: 'aaa', name: 'Stale Alpha' }),
				'my-modal',
				entities,
				false,
				null
			);
			expect(result).toEqual({ id: 'aaa', name: 'Alpha' });
		});

		it('ignores entityData when its id does not match state.id', () => {
			const result = resolveModalDeepLink(
				state('my-modal', 'zzz', { id: 'other', name: 'Wrong' }),
				'my-modal',
				entities,
				false,
				null
			);
			expect(result).toBeUndefined();
		});

		it('applies validate callback to entityData fallback', () => {
			const result = resolveModalDeepLink(
				state('my-modal', 'zzz', fallbackEntity),
				'my-modal',
				entities,
				false,
				null,
				() => false
			);
			expect(result).toBeUndefined();
		});

		it('uses entityData fallback during entity switch', () => {
			const result = resolveModalDeepLink(
				state('my-modal', 'zzz', fallbackEntity),
				'my-modal',
				entities,
				true,
				'aaa'
			);
			expect(result).toEqual(fallbackEntity);
		});
	});
});

/**
 * Opening one modal from inside another.
 *
 * The registry is a single slot, so opening B while A is on screen makes A close — and a close
 * handler that calls `closeModal()` unconditionally then clears B as well. That is the ordinary
 * shape of a close handler and it is right when the user dismissed A, so the repair belongs at the
 * point that knows A is being *superseded* rather than dismissed: `GenericModal`'s effect captures
 * the incoming state and puts it back if the handler cleared it.
 *
 * Without that, a chip or an action that navigates from inside one modal to another entity's
 * editor lands on the destination tab with nothing open — the deep-link effect that would have
 * opened it finds an empty registry, which `resolveModalDeepLink` correctly declines to act on.
 */
describe('a modal opened from inside another modal', () => {
	it('is not resolvable once a close handler has cleared the registry', () => {
		// What the destination tab's deep-link effect saw before the fix.
		const cleared = state(null);
		expect(
			resolveModalDeepLink(cleared, 'credential-editor', entities, false, null)
		).toBeUndefined();
	});

	it('resolves once the superseding state is restored', () => {
		// What it sees now: the state that was captured before the handler ran, put back intact.
		const superseding = state('credential-editor', 'bbb');
		const restored: ModalState = { ...superseding };

		expect(resolveModalDeepLink(restored, 'credential-editor', entities, false, null)).toEqual(
			entities[1]
		);
	});

	it('still resolves from entityData when the destination list has not loaded', () => {
		// The other half of the same click: the reader arrives from another tab, so the
		// destination's own query may not have settled. Carrying the entity makes the open
		// independent of that timing.
		const carried = { id: 'zzz', name: 'Zulu' };
		const restored = state('credential-editor', 'zzz', carried);

		expect(resolveModalDeepLink(restored, 'credential-editor', entities, false, null)).toEqual(
			carried
		);
	});
});

describe('navigateToEntity', () => {
	beforeEach(() => {
		vi.stubGlobal('window', {
			location: { href: 'http://localhost/#hosts', hash: '#hosts', search: '' },
			history: { replaceState: () => {} }
		});
	});
	afterEach(() => {
		vi.unstubAllGlobals();
		closeModal();
	});

	// A host's "Last found by" links a run record; the scans tab's editor only edits configurations.
	it('opens a discovery run in the history tab detail', () => {
		const run = { id: 'run', run_type: { type: 'Historical', results: {} } };
		navigateToEntity('Discovery', run.id, run);

		expect(window.location.hash).toBe('discovery-history');
		expect(get(modalState)).toMatchObject({
			name: 'discovery-history-detail',
			id: 'run',
			entityData: run
		});
	});

	it('opens a scan configuration in the scans tab editor', () => {
		const scan = { id: 'scan', run_type: { type: 'Scheduled' } };
		navigateToEntity('Discovery', scan.id, scan);

		expect(window.location.hash).toBe('discovery-scans');
		expect(get(modalState)).toMatchObject({ name: 'discovery-editor', id: 'scan' });
	});
});
