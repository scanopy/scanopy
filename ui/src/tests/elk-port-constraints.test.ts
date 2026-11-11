import { describe, it, expect } from 'vitest';
import {
	computeElkLayout,
	portConstraintFor,
	type ElkLayoutInput
} from '#lib/features/topology/layout/elk-layout.ts';
import { buildTopologyParentIndex } from '#lib/features/topology/topology-parent-index.ts';
import type { components } from '#lib/api/schema';

type TopologyNode = components['schemas']['Node'];
type TopologyEdge = components['schemas']['Edge'];

/**
 * The L2 Physical collapse levels 2 and 3, which rendered nothing.
 *
 * `PortOpStatus` subcontainers are `is_subcontainer` *and* `collapsed_by_default`, so in
 * L2 Physical they are closed at every level except 4 while their host containers stay open. That
 * makes levels 2 and 3 the only *mixed* states: some elements are laid out and have pass-1
 * positions, and some are inside a closed subcontainer and have none — yet both still emit ports,
 * because edge endpoints deliberately resolve through the collapsed ancestor.
 *
 * Level 1 (everything collapsed, no positions at all) and level 4 (nothing collapsed, every
 * position present) are uniform and always rendered, which is exactly the reported signature.
 */

function hostContainer(id: string): TopologyNode {
	return {
		id,
		node_type: 'Container',
		container_type: 'Host',
		position: { x: 0, y: 0 },
		size: { x: 400, y: 300 }
	} as TopologyNode;
}

function portStatusSubcontainer(id: string, parent: string): TopologyNode {
	return {
		id,
		node_type: 'Container',
		container_type: 'PortOpStatus',
		parent_container_id: parent,
		position: { x: 0, y: 0 },
		size: { x: 250, y: 40 }
	} as TopologyNode;
}

function interfaceElement(id: string, containerId: string, hostId: string): TopologyNode {
	return {
		id,
		node_type: 'Element',
		element_type: 'Interface',
		host_id: hostId,
		container_id: containerId,
		position: { x: 0, y: 0 },
		size: { x: 250, y: 54 }
	} as TopologyNode;
}

function physicalLink(source: string, target: string): TopologyEdge {
	return {
		edge_type: 'PhysicalLink',
		source,
		target,
		source_handle: 'Bottom',
		target_handle: 'Top',
		view_config: {
			type: 'active' as const,
			affects_layout: true,
			default_visibility: 'visible',
			stroke: 'solid',
			highlight_behavior: 'when_visible',
			will_target_container: false
		}
	} as TopologyEdge;
}

/**
 * Three hosts. `open-host` shows its interface directly, so pass 1 positions it; `closed-host-a`
 * and `closed-host-b` hold theirs inside a collapsed `PortOpStatus` subcontainer, so pass 1 never
 * sees them. Every host is joined to `open-host`, which is what makes all three emit ports.
 */
function mixedCollapseInput(): ElkLayoutInput {
	const nodes: TopologyNode[] = [
		hostContainer('open-host'),
		interfaceElement('open-if', 'open-host', 'open-host'),
		hostContainer('closed-host-a'),
		portStatusSubcontainer('closed-sub-a', 'closed-host-a'),
		interfaceElement('closed-if-a', 'closed-sub-a', 'closed-host-a'),
		hostContainer('closed-host-b'),
		portStatusSubcontainer('closed-sub-b', 'closed-host-b'),
		interfaceElement('closed-if-b', 'closed-sub-b', 'closed-host-b')
	];
	const edges: TopologyEdge[] = [
		physicalLink('closed-if-a', 'open-if'),
		physicalLink('closed-if-b', 'open-if')
	];

	return {
		nodes,
		edges,
		view: 'L2Physical',
		collapsedContainers: new Set(['closed-sub-a', 'closed-sub-b']),
		parentIndex: buildTopologyParentIndex(nodes),
		elementNodeSizes: new Map([
			['open-if', { x: 250, y: 54 }],
			['closed-if-a', { x: 250, y: 54 }],
			['closed-if-b', { x: 250, y: 54 }],
			['closed-sub-a', { x: 250, y: 40 }],
			['closed-sub-b', { x: 250, y: 40 }]
		]),
		topology: {
			id: 'topology',
			nodes,
			edges,
			subnets: [],
			hosts: [],
			ip_addresses: [],
			interfaces: [],
			services: [],
			dependencies: [],
			entity_tags: [],
			ports: [],
			bindings: [],
			tags: [],
			options: {
				local: { hide_edge_types: {}, no_fade_edges: false, hide_resize_handles: false },
				request: {
					hide_ports: false,
					hide_service_categories: [],
					container_rules: [],
					element_rules: []
				}
			}
		} as unknown as ElkLayoutInput['topology']
	};
}

describe('L2 Physical at a mixed collapse level', () => {
	it('gives every rendered container a real size', async () => {
		const result = await computeElkLayout(mixedCollapseInput());

		for (const id of ['open-host', 'closed-host-a', 'closed-host-b']) {
			const size = result.containerSizes.get(id);
			expect(size, `${id} has no size`).toBeDefined();
			expect(size!.width, `${id} width`).toBeGreaterThan(0);
			expect(size!.height, `${id} height`).toBeGreaterThan(0);
		}
	});

	/**
	 * The failure this reproduces is not "no output" but "all output at the origin": ports declared
	 * under `FIXED_POS` with no coordinates are read as (0,0), so every container carrying one is
	 * stacked there and the canvas reads as blank.
	 */
	it('does not stack the containers holding unpositioned ports at the origin', async () => {
		const result = await computeElkLayout(mixedCollapseInput());

		const positions = ['open-host', 'closed-host-a', 'closed-host-b'].map((id) => {
			const pos = result.nodePositions.get(id);
			expect(pos, `${id} has no position`).toBeDefined();
			return `${pos!.x},${pos!.y}`;
		});

		expect(new Set(positions).size, `containers overlap: ${positions.join(' | ')}`).toBe(
			positions.length
		);
	});

	/** The two uniform levels, which always rendered, must keep doing so. */
	it('still lays out the uniform levels either side of it', async () => {
		const everythingClosed = mixedCollapseInput();
		everythingClosed.collapsedContainers = new Set([
			'open-host',
			'closed-host-a',
			'closed-host-b',
			'closed-sub-a',
			'closed-sub-b'
		]);
		const nothingClosed = mixedCollapseInput();
		nothingClosed.collapsedContainers = new Set();

		for (const input of [everythingClosed, nothingClosed]) {
			const result = await computeElkLayout(input);
			for (const id of ['closed-host-a', 'closed-host-b']) {
				const size = result.containerSizes.get(id);
				expect(size?.width ?? 0).toBeGreaterThan(0);
				expect(size?.height ?? 0).toBeGreaterThan(0);
			}
		}
	});
});

/**
 * A ring of three switches, each linked to the next, next to a separate component. Laid out RIGHT
 * with POLYLINE routing and connected-component compaction, ELK returned NaN for the x of every
 * root container and for the root's width, so every L2 node and edge rendered at x = NaN.
 */
function physicalRingInput(): ElkLayoutInput {
	const input = mixedCollapseInput();
	const nodes: TopologyNode[] = [];
	for (const host of ['ring-a', 'ring-b', 'ring-c', 'hub', 'leaf-a', 'leaf-b']) {
		nodes.push(hostContainer(host), interfaceElement(`${host}-if`, host, host));
	}
	const edges: TopologyEdge[] = [
		physicalLink('ring-a-if', 'ring-b-if'),
		physicalLink('ring-b-if', 'ring-c-if'),
		physicalLink('ring-c-if', 'ring-a-if'),
		physicalLink('hub-if', 'leaf-a-if'),
		physicalLink('hub-if', 'leaf-b-if')
	];
	return {
		...input,
		nodes,
		edges,
		collapsedContainers: new Set(),
		parentIndex: buildTopologyParentIndex(nodes),
		elementNodeSizes: new Map(
			nodes.filter((n) => n.node_type === 'Element').map((n) => [n.id, { x: 250, y: 54 }])
		),
		topology: { ...input.topology, nodes, edges }
	};
}

describe('L2 Physical with a physical-link ring', () => {
	it('places every node at a finite position', async () => {
		const result = await computeElkLayout(physicalRingInput());

		const nonFinite = [...result.nodePositions]
			.filter(([, pos]) => !Number.isFinite(pos.x) || !Number.isFinite(pos.y))
			.map(([id, pos]) => `${id}: ${pos.x},${pos.y}`);
		expect(nonFinite).toEqual([]);
		for (const [id, size] of result.containerSizes) {
			expect(Number.isFinite(size.width) && Number.isFinite(size.height), id).toBe(true);
		}
	});
});

/**
 * One switch with a VLAN group whose ports run TrueNAS, HV01, HV02 uplink top to bottom, linked to
 * three hosts given in the order HV02, TrueNAS, HV01. Host ids sort the switch's ports into that
 * order. Laid out in model order, every edge crosses another; ordering the hosts by their port
 * removes every crossing.
 */
function switchUplinksInput(): ElkLayoutInput {
	const input = mixedCollapseInput();
	const hosts = ['h-c-hv02', 'h-a-truenas', 'h-b-hv01'];
	const nodes: TopologyNode[] = [
		hostContainer('switch'),
		portStatusSubcontainer('vlan-20', 'switch'),
		...hosts.map((host) => interfaceElement(`switch-${host}`, 'vlan-20', 'switch'))
	];
	const edges: TopologyEdge[] = [];
	for (const host of hosts) {
		nodes.push(hostContainer(host), interfaceElement(`${host}-if`, host, host));
		edges.push(physicalLink(`switch-${host}`, `${host}-if`));
	}
	return {
		...input,
		nodes,
		edges,
		collapsedContainers: new Set(),
		parentIndex: buildTopologyParentIndex(nodes),
		elementNodeSizes: new Map(
			nodes.filter((n) => n.node_type === 'Element').map((n) => [n.id, { x: 120, y: 90 }])
		),
		topology: { ...input.topology, nodes, edges }
	};
}

/** Edge pairs whose endpoints are in opposite vertical order at each end. */
function chordCrossings(input: ElkLayoutInput, positions: Map<string, { x: number; y: number }>) {
	const byId = new Map(input.nodes.map((n) => [n.id, n]));
	const sizeOf = (id: string) => input.elementNodeSizes?.get(id)?.y ?? 0;
	const absoluteY = (id: string): number => {
		const node = byId.get(id) as Record<string, unknown> | undefined;
		const parent = (node?.container_id ?? node?.parent_container_id) as string | undefined;
		return (positions.get(id)?.y ?? 0) + (parent ? absoluteY(parent) : 0);
	};
	const ends = input.edges.map((e) => [
		absoluteY(e.source) + sizeOf(e.source) / 2,
		absoluteY(e.target) + sizeOf(e.target) / 2
	]);
	let crossings = 0;
	for (let i = 0; i < ends.length; i++) {
		for (let j = i + 1; j < ends.length; j++) {
			if ((ends[i][0] - ends[j][0]) * (ends[i][1] - ends[j][1]) < 0) crossings++;
		}
	}
	return crossings;
}

/**
 * Collapse level 2: every port group is closed. The switch's Trunk group sits above its VLAN 20
 * group; `docker` links to VLAN 20 and comes first in the model, `ap` and `fw` link to Trunk.
 */
function collapsedGroupsInput(): ElkLayoutInput {
	const input = mixedCollapseInput();
	const nodes: TopologyNode[] = [
		hostContainer('switch'),
		portStatusSubcontainer('switch-a-trunk', 'switch'),
		portStatusSubcontainer('switch-b-vlan', 'switch'),
		interfaceElement('switch-ap', 'switch-a-trunk', 'switch'),
		interfaceElement('switch-fw', 'switch-a-trunk', 'switch'),
		interfaceElement('switch-docker', 'switch-b-vlan', 'switch')
	];
	const edges: TopologyEdge[] = [];
	for (const host of ['docker', 'ap', 'fw']) {
		nodes.push(
			hostContainer(host),
			portStatusSubcontainer(`${host}-sub`, host),
			interfaceElement(`${host}-if`, `${host}-sub`, host)
		);
		edges.push(physicalLink(`${host}-if`, `switch-${host}`));
	}
	// The switch's other VLAN 20 ports, linked out to hosts on its far side.
	for (const host of ['hv01', 'hv02', 'nas']) {
		nodes.push(
			interfaceElement(`switch-${host}`, 'switch-b-vlan', 'switch'),
			hostContainer(host),
			portStatusSubcontainer(`${host}-sub`, host),
			interfaceElement(`${host}-if`, `${host}-sub`, host)
		);
		edges.push(physicalLink(`switch-${host}`, `${host}-if`));
	}
	const subs = nodes.filter((n) => (n as Record<string, unknown>).parent_container_id);
	return {
		...input,
		nodes,
		edges,
		collapsedContainers: new Set(subs.map((n) => n.id)),
		parentIndex: buildTopologyParentIndex(nodes),
		elementNodeSizes: new Map(subs.map((n) => [n.id, { x: 250, y: 40 }])),
		topology: { ...input.topology, nodes, edges }
	};
}

describe('L2 Physical edge crossings', () => {
	it('orders hosts by the switch ports they link to', async () => {
		const input = switchUplinksInput();
		const result = await computeElkLayout(input);
		expect(chordCrossings(input, result.nodePositions)).toBe(0);
	});

	it('orders hosts by the closed port group they link to', async () => {
		const input = collapsedGroupsInput();
		const result = await computeElkLayout(input);
		const y = (id: string) => result.nodePositions.get(id)!.y;
		// Trunk is above VLAN 20 inside the switch, so docker's host goes below both Trunk hosts.
		expect(y('switch-a-trunk')).toBeLessThan(y('switch-b-vlan'));
		expect(y('docker')).toBeGreaterThan(Math.max(y('ap'), y('fw')));
	});
});

describe('an open container holding one card', () => {
	it.each(['L2Physical', 'L3Logical'] as const)(
		'is as wide as the card and its padding in %s',
		async (view) => {
			const nodes: TopologyNode[] = [
				hostContainer('host'),
				portStatusSubcontainer('group', 'host'),
				interfaceElement('card', 'group', 'host')
			];
			const input: ElkLayoutInput = {
				...mixedCollapseInput(),
				nodes,
				edges: [],
				view,
				collapsedContainers: new Set(),
				parentIndex: buildTopologyParentIndex(nodes),
				elementNodeSizes: new Map([['card', { x: 120, y: 90 }]])
			};
			const result = await computeElkLayout(input);
			// 25px padding either side, from the container types' metadata.
			expect(result.containerSizes.get('group')?.width).toBe(170);
			expect(result.containerSizes.get('host')?.width).toBe(220);
		}
	);
});

describe('portConstraintFor', () => {
	it('fixes positions only when every port on the container has them', () => {
		expect(
			portConstraintFor([
				{ x: 10, y: 20 },
				{ x: 0, y: 40 }
			])
		).toBe('FIXED_POS');
	});

	/**
	 * The mixed collapse state in one assertion: one element inside a collapsed container has no
	 * pass-1 position, so its port has no coordinates, and the container it shares with positioned
	 * siblings must fall back rather than let ELK read the gap as the origin.
	 */
	it('falls back to the side constraint when any single port is unpositioned', () => {
		expect(portConstraintFor([{ x: 10, y: 20 }, {}])).toBe('FIXED_SIDE');
		expect(portConstraintFor([{}, {}])).toBe('FIXED_SIDE');
	});

	/** A half-known coordinate is not a coordinate — ELK would read the missing axis as zero. */
	it('does not accept a port with only one axis', () => {
		expect(portConstraintFor([{ x: 10 }])).toBe('FIXED_SIDE');
		expect(portConstraintFor([{ y: 10 }])).toBe('FIXED_SIDE');
	});
});
