import { get, type Writable } from 'svelte/store';
import type { Node, Edge } from '@xyflow/svelte';
import type { TopologyNode } from './types/base';
import {
	selectedNode as globalSelectedNode,
	selectedEdge as globalSelectedEdge,
	selectedNodes as globalSelectedNodes,
	editingDependencyId
} from './queries';
import { clearSearch } from './interactions';

/**
 * A user-initiated selection change that leaves the multi-selection abandons dep-edit mode:
 * picking a single node or edge, or clearing. Adding or removing a member (modifier click,
 * box select) while two or more nodes stay selected is how an edit changes members, so it
 * keeps the mode; cancelling there showed "Create Dependency" and restored the saved edge.
 * startEditing() writes selectedNodes/selectedEdge directly via their raw
 * setters — it does not route through these wrappers — so clearing here is
 * safe and won't cancel the edit flow as it starts up.
 */
function cancelDependencyEdit(): void {
	editingDependencyId.set(null);
}

/**
 * Store references for selection state.
 * Defaults to global stores; override with context stores for share/embed isolation.
 */
export interface SelectionStores {
	selectedNode: Writable<Node | null>;
	selectedEdge: Writable<Edge | null>;
	selectedNodes: Writable<Node[]>;
}

const defaultStores: SelectionStores = {
	selectedNode: globalSelectedNode,
	selectedEdge: globalSelectedEdge,
	selectedNodes: globalSelectedNodes
};

export function selectNode(node: Node, stores: SelectionStores = defaultStores) {
	cancelDependencyEdit();
	clearSearch();
	stores.selectedNode.set(node);
	stores.selectedEdge.set(null);
	stores.selectedNodes.set([]);
}

export function selectEdge(edge: Edge, stores: SelectionStores = defaultStores) {
	cancelDependencyEdit();
	clearSearch();
	stores.selectedEdge.set(edge);
	stores.selectedNode.set(null);
}

export function clearSelection(stores: SelectionStores = defaultStores) {
	cancelDependencyEdit();
	stores.selectedNode.set(null);
	stores.selectedEdge.set(null);
	stores.selectedNodes.set([]);
	clearSearch();
}

export function handleModifierNodeClick(node: Node, stores: SelectionStores = defaultStores) {
	const nodeData = node.data as TopologyNode;
	if (nodeData.node_type !== 'Element') return;

	const current = get(stores.selectedNodes);
	const currentSingle = get(stores.selectedNode);
	const idx = current.findIndex((n) => n.id === node.id);

	if (idx !== -1) {
		// Deselect: remove from multi-selection
		const remaining = current.filter((_, i) => i !== idx);
		if (remaining.length < 2) {
			cancelDependencyEdit();
			stores.selectedNodes.set([]);
			if (remaining.length === 1) {
				stores.selectedNode.set(remaining[0]);
			}
		} else {
			stores.selectedNodes.set(remaining);
		}
	} else {
		// Add to multi-selection
		if (current.length === 0 && currentSingle) {
			const singleData = currentSingle.data as TopologyNode;
			if (singleData.node_type === 'Element') {
				stores.selectedNodes.set([currentSingle, node]);
			} else {
				stores.selectedNodes.set([node]);
			}
		} else if (current.length > 0) {
			stores.selectedNodes.set([...current, node]);
		} else {
			stores.selectedNodes.set([node]);
		}
		stores.selectedNode.set(null);
		stores.selectedEdge.set(null);
	}
}

export function handleBoxSelect(newNodes: Node[], stores: SelectionStores = defaultStores) {
	const interfaceNodes = newNodes.filter((n) => {
		const nodeData = n.data as TopologyNode;
		return nodeData.node_type === 'Element';
	});

	if (interfaceNodes.length >= 2) {
		const current = get(stores.selectedNodes);
		const currentIds = new Set(current.map((n) => n.id));
		const newIds = new Set(interfaceNodes.map((n) => n.id));
		const kept = current.filter((n) => newIds.has(n.id));
		const added = interfaceNodes.filter((n) => !currentIds.has(n.id));

		stores.selectedNodes.set([...kept, ...added]);
		stores.selectedNode.set(null);
		stores.selectedEdge.set(null);
	} else if (newNodes.length > 0) {
		cancelDependencyEdit();
		stores.selectedNodes.set([]);
	}
}
