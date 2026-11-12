import { describe, it, expect, beforeEach } from 'vitest';
import type { Edge } from '@xyflow/svelte';
import { computeEdgeDisplayUpdates } from '#lib/features/topology/pipeline/sync-edge-display.ts';
import { setEdgeHover, clearEdgeHoverState } from '#lib/features/topology/interactions.ts';

/**
 * `computeEdgeDisplayUpdates` runs on every selection change and every edge
 * pointer-enter/leave. It must only allocate for edges whose display state
 * actually changed — otherwise hovering one edge re-renders all of them, and
 * also invalidates SvelteFlow's per-edge path-geometry memo.
 *
 * These assert the identity contract rather than the flag values: an edge whose
 * state is unchanged must come back as the *same object*, and a settled array
 * must come back as the *same array*.
 */

const NO_HIDDEN = new Set<string>();

function edge(id: string, source: string, target: string): Edge {
	return {
		id,
		source,
		target,
		type: 'custom',
		animated: false,
		// getEdgeDisplayState reads endpoints off `data`, not off the flow edge.
		data: { edge_type: 'PhysicalLink', source, target }
	};
}

/** Run once to populate the display flags, as the real pipeline does. */
function settled(edges: Edge[]): Edge[] {
	return computeEdgeDisplayUpdates(edges, null, null, NO_HIDDEN, NO_HIDDEN);
}

describe('computeEdgeDisplayUpdates identity', () => {
	beforeEach(() => {
		clearEdgeHoverState();
	});

	it('returns the same array when nothing changed', () => {
		const base = settled([edge('e1', 'a', 'b'), edge('e2', 'b', 'c')]);

		expect(computeEdgeDisplayUpdates(base, null, null, NO_HIDDEN, NO_HIDDEN)).toBe(base);
	});

	it('rebuilds only the hovered edge', () => {
		const base = settled([edge('e1', 'a', 'b'), edge('e2', 'b', 'c'), edge('e3', 'c', 'd')]);

		// This is the hot path: pointer-enter re-runs the whole computation, but
		// only the hovered edge's state actually differs.
		setEdgeHover(base[0], true, base);
		const next = computeEdgeDisplayUpdates(base, null, null, NO_HIDDEN, NO_HIDDEN);

		expect(next[0]).not.toBe(base[0]);
		expect(next[1]).toBe(base[1]);
		expect(next[2]).toBe(base[2]);
	});

	it('is a no-op when the same hover state is recomputed', () => {
		const base = settled([edge('e1', 'a', 'b'), edge('e2', 'b', 'c')]);
		setEdgeHover(base[0], true, base);

		const hovered = computeEdgeDisplayUpdates(base, null, null, NO_HIDDEN, NO_HIDDEN);
		// Repeated pointer events over the same edge must stop churning here.
		expect(computeEdgeDisplayUpdates(hovered, null, null, NO_HIDDEN, NO_HIDDEN)).toBe(hovered);
	});

	it('rebuilds every edge when selection is gained, since hasActiveSelection is global', () => {
		const base = settled([edge('e1', 'a', 'b'), edge('e2', 'b', 'c')]);

		const next = computeEdgeDisplayUpdates(base, null, base[0], NO_HIDDEN, NO_HIDDEN);

		// Documenting the real cost: `hasActiveSelection` is carried on every
		// edge, so gaining or losing a selection legitimately touches all of
		// them. Only moving *between* selections is cheap (next test).
		expect(next.every((e, i) => e !== base[i])).toBe(true);
		expect(next.find((e) => e.id === 'e1')?.data?.isSelected).toBe(true);
	});

	it('rebuilds only the two affected edges when selection moves between edges', () => {
		const base = settled([edge('e1', 'a', 'b'), edge('e2', 'b', 'c'), edge('e3', 'c', 'd')]);
		const withE1 = computeEdgeDisplayUpdates(base, null, base[0], NO_HIDDEN, NO_HIDDEN);

		const withE2 = computeEdgeDisplayUpdates(withE1, null, withE1[1], NO_HIDDEN, NO_HIDDEN);

		expect(withE2[0]).not.toBe(withE1[0]); // lost isSelected
		expect(withE2[1]).not.toBe(withE1[1]); // gained isSelected
		expect(withE2[2]).toBe(withE1[2]); // untouched
	});

	it('rebuilds an edge whose endpoint becomes search-hidden', () => {
		const base = settled([edge('e1', 'a', 'b'), edge('e2', 'c', 'd')]);

		const next = computeEdgeDisplayUpdates(base, null, null, new Set(['a']), NO_HIDDEN);

		expect(next[0]).not.toBe(base[0]);
		expect(next[0].data?.isEndpointSearchHidden).toBe(true);
		expect(next[1]).toBe(base[1]);
	});

	it('clears an animation flag left on an edge by a previous render', () => {
		const stale = { ...edge('e1', 'a', 'b'), animated: true };

		const next = computeEdgeDisplayUpdates([stale], null, null, NO_HIDDEN, NO_HIDDEN);

		expect(next[0]).not.toBe(stale);
		expect(next[0].animated).toBe(false);
	});
});
