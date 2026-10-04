import { describe, it, expect } from 'vitest';
import { changedCardIds } from '$lib/features/topology/pipeline/execute-layout';
import { inlineGroupKey } from '$lib/features/topology/collapse';

/**
 * Opening or closing a manager box changes its card's height. The layout finds the cards whose
 * cached size to drop, before ELK re-runs, from the keys that changed.
 */
describe('changedCardIds', () => {
	it('maps a toggled manager box back to its card', () => {
		const before = new Set(['ip-card']);
		const after = new Set(['ip-card', inlineGroupKey('guest', 'identities')]);

		expect(changedCardIds(before, after)).toEqual(new Set(['guest']));
	});

	it('re-measures a card once when two of its boxes toggle together, and when a box expands', () => {
		const before = new Set([inlineGroupKey('guest', 'docker')]);
		const after = new Set([inlineGroupKey('guest', 'identities')]);

		expect(changedCardIds(before, after)).toEqual(new Set(['guest']));
		expect(changedCardIds(after, new Set())).toEqual(new Set(['guest']));
	});

	it('reports nothing when the keys are unchanged', () => {
		const keys = new Set(['ip-card', inlineGroupKey('guest', 'identities')]);

		expect(changedCardIds(keys, new Set(keys))).toEqual(new Set());
	});
});
