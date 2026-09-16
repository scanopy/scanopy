import { describe, it, expect } from 'vitest';
import { collectEdgeHandles, mergeEdgeHandles } from '$lib/features/topology/interactions';

/**
 * Nodes render only the handles in `edgeHandles` (`reactive-stores.svelte.ts`): the real edges'
 * handles with the dependency preview's merged over them. SvelteFlow drops any edge naming a handle
 * its node never rendered. A new dependency's preview picks its sides by geometry, not from a real
 * edge, so it often names a handle the real edges never needed.
 *
 * The invariant: every handle a drawn edge names, real or preview, is in its node's rendered set,
 * and a node the preview does not touch renders exactly what it did without one.
 */

type FlowEdge = {
	id: string;
	source: string;
	target: string;
	sourceHandle: string;
	targetHandle: string;
};

function edge(
	id: string,
	source: string,
	target: string,
	sourceHandle: string,
	targetHandle: string
): FlowEdge {
	return { id, source, target, sourceHandle, targetHandle };
}

/** The rendered map, built the way `edgeHandles` builds it. */
function rendered(realHandles: Map<string, Set<string>>, preview: FlowEdge[]) {
	return mergeEdgeHandles(realHandles, collectEdgeHandles(preview));
}

function missingHandles(map: Map<string, Set<string>>, edges: FlowEdge[]): string[] {
	const missing: string[] = [];
	for (const e of edges) {
		if (!map.get(e.source)?.has(`source:${e.sourceHandle}`))
			missing.push(`${e.id}: ${e.source} source:${e.sourceHandle}`);
		if (!map.get(e.target)?.has(`target:${e.targetHandle}`))
			missing.push(`${e.id}: ${e.target} target:${e.targetHandle}`);
	}
	return missing;
}

const REAL = [edge('real-0', 'a', 'b', 'Right', 'Left'), edge('real-1', 'b', 'c', 'Right', 'Left')];

describe('dependency preview handles', () => {
	it('renders handles for a new dependency between two nodes with no edges', () => {
		const realHandles = collectEdgeHandles(REAL);
		const preview = [edge('preview-0', 'x', 'y', 'Bottom', 'Top')];

		expect(missingHandles(rendered(realHandles, preview), [...REAL, ...preview])).toEqual([]);
	});

	it('renders a side the node’s real edges do not use, without dropping theirs', () => {
		const realHandles = collectEdgeHandles(REAL);
		// `c` is only ever a target, so it has no source handle at all until the preview adds one.
		const preview = [edge('preview-0', 'c', 'a', 'Bottom', 'Top')];

		expect(missingHandles(rendered(realHandles, preview), [...REAL, ...preview])).toEqual([]);
	});

	it('previews an existing dependency with the real edge’s handles and adds none', () => {
		const realHandles = collectEdgeHandles(REAL);
		const preview = [edge('preview-0', 'a', 'b', 'Right', 'Left')];
		const map = rendered(realHandles, preview);

		expect(missingHandles(map, [...REAL, ...preview])).toEqual([]);
		for (const [nodeId, handles] of realHandles) {
			expect(map.get(nodeId)).toEqual(handles);
		}
	});

	it('leaves nodes the preview does not touch exactly as they were', () => {
		const realHandles = collectEdgeHandles(REAL);
		const map = rendered(realHandles, [edge('preview-0', 'a', 'x', 'Bottom', 'Top')]);

		expect(map.get('b')).toBe(realHandles.get('b'));
		expect(map.get('c')).toBe(realHandles.get('c'));
		expect(rendered(realHandles, [])).toBe(realHandles);
	});
});
