import { describe, it, expect } from 'vitest';
import { computeElkLayout, type ElkLayoutInput } from '#lib/features/topology/layout/elk-layout.ts';
import type { components } from '#lib/api/schema';

type TopologyNode = components['schemas']['Node'];
type TopologyEdge = components['schemas']['Edge'];

let idCounter = 0;
function uuid(): string {
	return `00000000-0000-0000-0000-${String(++idCounter).padStart(12, '0')}`;
}

function container(id: string, parent?: string): TopologyNode {
	return {
		id,
		node_type: 'Container',
		container_type: parent ? 'NestedTag' : 'Subnet',
		...(parent && { parent_container_id: parent }),
		position: { x: 0, y: 0 },
		size: { x: 0, y: 0 }
	} as TopologyNode;
}

function element(id: string, containerId: string): TopologyNode {
	return {
		id,
		node_type: 'Element',
		element_type: 'IPAddress',
		host_id: uuid(),
		container_id: containerId,
		subnet_id: containerId,
		position: { x: 0, y: 0 },
		size: { x: 0, y: 0 }
	} as TopologyNode;
}

function layoutEdge(source: string, target: string): TopologyEdge {
	return {
		edge_type: 'SameHost',
		source,
		target,
		source_handle: 'Bottom',
		target_handle: 'Top',
		view_config: {
			type: 'active',
			affects_layout: true,
			default_visibility: 'visible',
			stroke: 'solid',
			highlight_behavior: 'when_visible',
			will_target_container: false
		}
	} as TopologyEdge;
}

function input(
	nodes: TopologyNode[],
	edges: TopologyEdge[],
	sizes: Map<string, { x: number; y: number }>,
	collapsed: Set<string> = new Set(),
	preserveChildOrder = true
): ElkLayoutInput {
	return {
		nodes,
		edges,
		view: 'L3Logical',
		elementNodeSizes: sizes,
		collapsedContainers: collapsed,
		preserveChildOrder,
		topology: {
			nodes,
			edges,
			hosts: [],
			interfaces: [],
			ip_addresses: [],
			ports: [],
			services: [],
			subnets: []
		} as unknown as ElkLayoutInput['topology']
	};
}

/** Children of `parentId` in reading order: top to bottom by row, left to right within one. */
function readingOrder(
	positions: Map<string, { x: number; y: number }>,
	nodes: TopologyNode[],
	parentId: string
): string[] {
	const children = nodes.filter((n) =>
		n.node_type === 'Element' ? n.container_id === parentId : n.parent_container_id === parentId
	);
	return children
		.map((n) => ({ id: n.id, ...positions.get(n.id)! }))
		.sort((a, b) => a.y - b.y || a.x - b.x)
		.map((c) => c.id);
}

describe('computeElkLayout with preserveChildOrder', () => {
	it('keeps input order over id order and size order', async () => {
		const subnet = uuid();
		// Descending ids: without a pinned order the layout falls back to ascending id.
		const ids = Array.from({ length: 8 }, () => uuid()).reverse();
		const nodes = [container(subnet), ...ids.map((id) => element(id, subnet))];
		// Sizes in neither input nor id order, so the packer's size ranking cannot reproduce either.
		const scale = [3, 7, 1, 5, 0, 6, 2, 4];
		const sizes = new Map(
			ids.map((id, i) => [id, { x: 150 + scale[i] * 40, y: 50 + scale[i] * 10 }])
		);

		const result = await computeElkLayout(input(nodes, [], sizes));

		expect(readingOrder(result.nodePositions, nodes, subnet)).toEqual(ids);
	});

	it('keeps expanded and collapsed groups in their input slot, across a cross-child edge', async () => {
		const subnet = uuid();
		const expanded = uuid();
		const collapsed = uuid();
		const [e0, e1, e2, a1, a2] = Array.from({ length: 5 }, () => uuid());
		const nodes = [
			container(subnet),
			element(e0, subnet),
			container(expanded, subnet),
			element(a1, expanded),
			element(a2, expanded),
			element(e1, subnet),
			container(collapsed, subnet),
			element(e2, subnet)
		];
		const sizes = new Map([
			[e0, { x: 180, y: 60 }],
			[e1, { x: 220, y: 60 }],
			[e2, { x: 300, y: 90 }],
			[a1, { x: 240, y: 60 }],
			[a2, { x: 180, y: 60 }],
			[collapsed, { x: 250, y: 40 }]
		]);
		// An edge from a loose element into a group switches the subnet to a layered layout when
		// order is not preserved, which ranks nodes by that edge rather than by input position.
		const edges = [layoutEdge(e2, a1)];

		const result = await computeElkLayout(input(nodes, edges, sizes, new Set([collapsed])));

		expect(readingOrder(result.nodePositions, nodes, subnet)).toEqual([
			e0,
			expanded,
			e1,
			collapsed,
			e2
		]);
		expect(readingOrder(result.nodePositions, nodes, expanded)).toEqual([a1, a2]);
	});
});

/** A dependency hop: drawn in every view that shows it, but laying nothing out in L3. */
function dependencyEdge(source: string, target: string, affectsLayout: boolean): TopologyEdge {
	const edge = layoutEdge(source, target);
	return {
		...edge,
		edge_type: 'RequestPath',
		view_config: { ...edge.view_config, affects_layout: affectsLayout }
	} as TopologyEdge;
}

function centre(
	positions: Map<string, { x: number; y: number }>,
	sizes: Map<string, { width: number; height: number }>,
	id: string
): { x: number; y: number } {
	const pos = positions.get(id)!;
	const size = sizes.get(id)!;
	return { x: pos.x + size.width / 2, y: pos.y + size.height / 2 };
}

describe('computeElkLayout with inner edges', () => {
	const card = { x: 200, y: 60 };

	it('stacks a chain of cards in its edge order, each below the one before', async () => {
		const box = uuid();
		const [a, b, c, d] = Array.from({ length: 4 }, () => uuid());
		// Listed against the chain, so input order and id order both disagree with it.
		const nodes = [container(box), ...[d, b, a, c].map((id) => element(id, box))];
		const sizes = new Map([a, b, c, d].map((id) => [id, card]));
		const edges = [
			dependencyEdge(c, d, true),
			dependencyEdge(a, b, true),
			dependencyEdge(b, c, true)
		];

		const result = await computeElkLayout(input(nodes, edges, sizes, new Set(), false));

		const y = (id: string) => result.nodePositions.get(id)!.y;
		expect(y(a)).toBeLessThan(y(b));
		expect(y(b)).toBeLessThan(y(c));
		expect(y(c)).toBeLessThan(y(d));
	});

	it('puts a hub above its spokes', async () => {
		const box = uuid();
		const [hub, s1, s2] = Array.from({ length: 3 }, () => uuid());
		const nodes = [container(box), ...[s2, s1, hub].map((id) => element(id, box))];
		const sizes = new Map([hub, s1, s2].map((id) => [id, card]));
		const edges = [dependencyEdge(hub, s1, true), dependencyEdge(hub, s2, true)];

		const result = await computeElkLayout(input(nodes, edges, sizes, new Set(), false));

		const y = (id: string) => result.nodePositions.get(id)!.y;
		expect(y(hub)).toBeLessThan(y(s1));
		expect(y(hub)).toBeLessThan(y(s2));
	});
});

describe('repacking disconnected containers', () => {
	it('places a container next to the one it shares a drawn edge with', async () => {
		const [hub, runtime, linked, loose] = Array.from({ length: 4 }, () => uuid());
		const [hubA, hubB, runtimeA, linkedA, looseA] = Array.from({ length: 5 }, () => uuid());
		// Unlinked containers listed first and made tallest, so the shelf would reach them first.
		const nodes = [
			container(loose),
			element(looseA, loose),
			container(hub),
			element(hubA, hub),
			element(hubB, hub),
			container(runtime),
			element(runtimeA, runtime),
			container(linked),
			element(linkedA, linked)
		];
		const card = { x: 200, y: 60 };
		const sizes = new Map([
			[hubA, card],
			[hubB, card],
			[runtimeA, card],
			[linkedA, card],
			[looseA, { x: 200, y: 300 }]
		]);
		const edges = [layoutEdge(hubA, runtimeA), dependencyEdge(hubB, linkedA, false)];

		const result = await computeElkLayout(input(nodes, edges, sizes, new Set(), false));

		const distance = (id: string) => {
			const a = centre(result.nodePositions, result.containerSizes, hub);
			const b = centre(result.nodePositions, result.containerSizes, id);
			return Math.hypot(a.x - b.x, a.y - b.y);
		};
		expect(distance(linked)).toBeLessThan(distance(loose));
	});
});
