import { describe, it, expect, beforeEach } from 'vitest';
import { get, writable } from 'svelte/store';
import type { Node, Edge } from '@xyflow/svelte';
import { editingDependencyId } from '$lib/features/topology/queries';
import {
	handleBoxSelect,
	handleModifierNodeClick,
	selectNode,
	type SelectionStores
} from '$lib/features/topology/selection';

/**
 * Editing a dependency selects its members, and adding a member is a modifier
 * click. That click used to end edit mode, so the pane switched to "Create
 * Dependency" and the canvas went back to drawing the saved edge without the
 * new member.
 */

function element(id: string): Node {
	return { id, position: { x: 0, y: 0 }, data: { id, node_type: 'Element' } } as Node;
}

function stores(selected: Node[]): SelectionStores {
	return {
		selectedNode: writable<Node | null>(null),
		selectedEdge: writable<Edge | null>(null),
		selectedNodes: writable<Node[]>(selected)
	};
}

describe('dependency edit mode across selection changes', () => {
	beforeEach(() => editingDependencyId.set('dep-1'));

	it('keeps edit mode when a modifier click adds a member', () => {
		const s = stores([element('a'), element('b')]);

		handleModifierNodeClick(element('c'), s);

		expect(get(editingDependencyId)).toBe('dep-1');
		expect(get(s.selectedNodes).map((n) => n.id)).toEqual(['a', 'b', 'c']);
	});

	it('keeps edit mode when a modifier click removes a member and two remain', () => {
		const s = stores([element('a'), element('b'), element('c')]);

		handleModifierNodeClick(element('c'), s);

		expect(get(editingDependencyId)).toBe('dep-1');
	});

	it('ends edit mode when fewer than two members remain', () => {
		const s = stores([element('a'), element('b')]);

		handleModifierNodeClick(element('b'), s);

		expect(get(editingDependencyId)).toBe(null);
	});

	it('keeps edit mode when a box select still holds two or more nodes', () => {
		const s = stores([element('a'), element('b')]);

		handleBoxSelect([element('a'), element('b'), element('c')], s);

		expect(get(editingDependencyId)).toBe('dep-1');
	});

	it('ends edit mode when a single node is selected', () => {
		selectNode(element('a'), stores([element('a'), element('b')]));

		expect(get(editingDependencyId)).toBe(null);
	});
});
