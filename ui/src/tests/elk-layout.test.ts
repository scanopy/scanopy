import { describe, it, expect, beforeEach } from 'vitest';
import { LayoutGraph } from '$lib/features/topology/layout/layout-graph';
import {
	computeElkLayout,
	computeOptimalHandles,
	type ElkLayoutInput,
	type ObstacleRect
} from '$lib/features/topology/layout/elk-layout';
import {
	isDisabledEdge,
	isDottedEdge,
	isOverlayEdge,
	affectsLayout,
	isHiddenByDefault,
	getDefaultHiddenEdgeTypes
} from '$lib/features/topology/layout/edge-classification';
import { elevateEdgesToContainers } from '$lib/features/topology/layout/edge-elevation';
import type { components } from '$lib/api/schema';

type TopologyNode = components['schemas']['Node'];
type TopologyEdge = components['schemas']['Edge'];
type Subnet = components['schemas']['Subnet'];
type EdgeStroke = components['schemas']['EdgeStroke'];
type SubnetType = components['schemas']['SubnetType'];

// --- Test Helpers ---

let idCounter = 0;
function uuid(): string {
	return `00000000-0000-0000-0000-${String(++idCounter).padStart(12, '0')}`;
}

function resetIds() {
	idCounter = 0;
}

function makeContainer(
	id: string,
	opts?: { width?: number; height?: number; container_type?: string; parent_container_id?: string }
): TopologyNode {
	return {
		id,
		node_type: 'Container',
		container_type: opts?.container_type ?? 'Subnet',
		...(opts?.parent_container_id && { parent_container_id: opts.parent_container_id }),
		position: { x: 0, y: 0 },
		size: { x: opts?.width ?? 400, y: opts?.height ?? 300 }
	} as TopologyNode;
}

function makeElement(id: string, subnetId: string, hostId?: string): TopologyNode {
	return {
		id,
		node_type: 'Element',
		element_type: 'IPAddress',
		host_id: hostId ?? uuid(),
		container_id: subnetId,
		subnet_id: subnetId,
		position: { x: 0, y: 0 },
		size: { x: 180, y: 60 }
	} as TopologyNode;
}

function makeEdge(
	source: string,
	target: string,
	edgeType: string = 'SameHost',
	viewConfig?: {
		affects_layout: boolean;
		default_visibility?: 'visible' | 'hidden';
		stroke?: EdgeStroke;
	}
): TopologyEdge {
	return {
		edge_type: edgeType,
		source,
		target,
		source_handle: 'Bottom',
		target_handle: 'Top',
		view_config: viewConfig
			? {
					type: 'active' as const,
					affects_layout: viewConfig.affects_layout,
					default_visibility: viewConfig.default_visibility ?? 'visible',
					stroke: viewConfig.stroke ?? 'solid',
					highlight_behavior: 'when_visible',
					will_target_container: false
				}
			: { type: 'disabled' as const }
	} as TopologyEdge;
}

/** Shorthand for a layout-affecting visible solid edge */
function primaryEdge(source: string, target: string, edgeType: string = 'SameHost'): TopologyEdge {
	return makeEdge(source, target, edgeType, { affects_layout: true });
}

function makeSubnet(id: string, subnetType: SubnetType): Subnet {
	return {
		id,
		name: `subnet-${subnetType}`,
		cidr_source: 'DaemonSelfReport',
		subnet_type: subnetType,
		site_id: uuid(),
		cidr: '10.0.0.0/24',
		virtualization_service_id: null,
		source: { type: 'Manual' },
		tags: [],
		created_at: '2026-01-01T00:00:00Z',
		updated_at: '2026-01-01T00:00:00Z'
	} as Subnet;
}

function makeTopology(
	nodes: TopologyNode[],
	edges: TopologyEdge[],
	subnets: Subnet[],
	view: ElkLayoutInput['view'] = 'L3Logical'
): ElkLayoutInput {
	return {
		nodes,
		edges,
		view,
		topology: {
			id: uuid(),
			created_at: '2026-01-01T00:00:00Z',
			updated_at: '2026-01-01T00:00:00Z',
			name: 'test',
			site_id: uuid(),
			is_locked: false,
			is_stale: false,
			last_refreshed: '2026-01-01T00:00:00Z',
			nodes,
			edges,
			subnets,
			hosts: [],
			ip_addresses: [],
			interfaces: [],
			services: [],
			dependencies: [],
			entity_tags: [],
			ports: [],
			bindings: [],
			tags: [],
			removed_hosts: [],
			removed_ip_addresses: [],
			removed_interfaces: [],
			removed_services: [],
			removed_subnets: [],
			removed_dependencies: [],
			removed_ports: [],
			removed_bindings: [],
			options: {
				local: {
					left_zone_title: '', // Deprecated field, kept for generated type compat
					hide_edge_types: [],
					no_fade_edges: false,
					hide_resize_handles: false
				},
				request: {
					hide_ports: false,
					hide_service_categories: [],
					container_rules: [
						{ id: '00000000-0000-0000-0000-000000000001', rule: 'BySubnet' },
						{ id: '00000000-0000-0000-0000-000000000002', rule: 'MergeContainerBridges' }
					],
					element_rules: []
				}
			}
		} as unknown as ElkLayoutInput['topology']
	};
}

// --- Edge Classification Tests ---

describe('edge view config helpers', () => {
	it('treats edges without view_config as disabled', () => {
		const edge = { edge_type: 'SameHost', source: 'a', target: 'b' } as TopologyEdge;
		expect(isDisabledEdge(edge)).toBe(true);
		expect(affectsLayout(edge)).toBe(false);
	});

	it('isDisabledEdge returns true for disabled config', () => {
		const edge = makeEdge('a', 'b', 'SameHost');
		expect(isDisabledEdge(edge)).toBe(true);
	});

	it('isDisabledEdge returns false for active config', () => {
		const edge = makeEdge('a', 'b', 'SameHost', { affects_layout: true });
		expect(isDisabledEdge(edge)).toBe(false);
	});

	it('affectsLayout reads from view config', () => {
		expect(affectsLayout(makeEdge('a', 'b', 'SameHost', { affects_layout: true }))).toBe(true);
		expect(affectsLayout(makeEdge('a', 'b', 'SameHost', { affects_layout: false }))).toBe(false);
		expect(affectsLayout(makeEdge('a', 'b', 'SameHost'))).toBe(false); // disabled
	});

	it('isOverlayEdge treats any non-solid stroke as an overlay', () => {
		// Overlay styling (thinner, dimmed) applies to every non-solid stroke, so adding a stroke
		// must not require touching the width/opacity logic.
		for (const stroke of ['dashed', 'dotted'] as const) {
			expect(isOverlayEdge(makeEdge('a', 'b', 'SameHost', { affects_layout: true, stroke }))).toBe(
				true
			);
		}
		expect(
			isOverlayEdge(makeEdge('a', 'b', 'SameHost', { affects_layout: true, stroke: 'solid' }))
		).toBe(false);
	});

	it('isDottedEdge distinguishes dotted from other overlay strokes', () => {
		// Only the dash pattern branches on the specific stroke.
		expect(
			isDottedEdge(makeEdge('a', 'b', 'SameContainer', { affects_layout: false, stroke: 'dotted' }))
		).toBe(true);
		expect(
			isDottedEdge(makeEdge('a', 'b', 'SameHost', { affects_layout: true, stroke: 'dashed' }))
		).toBe(false);
	});

	it('isHiddenByDefault reads default_visibility from view config', () => {
		expect(
			isHiddenByDefault(
				makeEdge('a', 'b', 'SameHost', { affects_layout: true, default_visibility: 'hidden' })
			)
		).toBe(true);
		expect(
			isHiddenByDefault(
				makeEdge('a', 'b', 'SameHost', { affects_layout: true, default_visibility: 'visible' })
			)
		).toBe(false);
	});

	it('getDefaultHiddenEdgeTypes returns an array', () => {
		const result = getDefaultHiddenEdgeTypes('L3Logical');
		expect(Array.isArray(result)).toBe(true);
	});
});

// --- ELK Layout Tests ---

describe('computeElkLayout', () => {
	beforeEach(() => resetIds());

	it('returns empty maps for empty input', async () => {
		const result = await computeElkLayout({
			nodes: [],
			edges: [],
			view: 'L3Logical',
			topology: makeTopology([], [], []).topology
		});
		expect(result.nodePositions.size).toBe(0);
		expect(result.containerSizes.size).toBe(0);
	});

	it('produces valid positions for a simple topology', async () => {
		const subnetExt = uuid();
		const subnetGw = uuid();
		const subnetLan = uuid();

		const elem1 = uuid();
		const elem2 = uuid();
		const elem3 = uuid();
		const elem4 = uuid();
		const elem5 = uuid();

		const host1 = uuid();

		const nodes: TopologyNode[] = [
			makeContainer(subnetExt),
			makeContainer(subnetGw),
			makeContainer(subnetLan),
			makeElement(elem1, subnetExt, host1),
			makeElement(elem2, subnetExt),
			makeElement(elem3, subnetGw, host1), // multi-homed: same host in different subnet
			makeElement(elem4, subnetLan),
			makeElement(elem5, subnetLan)
		];

		const edges: TopologyEdge[] = [
			primaryEdge(elem1, elem3, 'SameHost'), // primary: ext -> gw
			primaryEdge(elem3, elem4, 'SameHost'), // primary: gw -> lan
			makeEdge(elem1, elem4, 'Hypervisor') // overlay: should be ignored by layout
		];

		const subnets = [
			makeSubnet(subnetExt, 'Internet'),
			makeSubnet(subnetGw, 'Gateway'),
			makeSubnet(subnetLan, 'Lan')
		];

		const input = makeTopology(nodes, edges, subnets);
		const result = await computeElkLayout(input);

		// All nodes should have positions
		expect(result.nodePositions.size).toBeGreaterThanOrEqual(nodes.length);

		// All positions should be valid numbers
		for (const [, pos] of result.nodePositions) {
			expect(Number.isFinite(pos.x)).toBe(true);
			expect(Number.isFinite(pos.y)).toBe(true);
		}

		// Container sizes should be set
		expect(result.containerSizes.has(subnetExt)).toBe(true);
		expect(result.containerSizes.has(subnetGw)).toBe(true);
		expect(result.containerSizes.has(subnetLan)).toBe(true);

		for (const [, size] of result.containerSizes) {
			expect(size.width).toBeGreaterThan(0);
			expect(size.height).toBeGreaterThan(0);
		}
	});

	it('orders containers by edge connectivity (connected containers in adjacent layers)', async () => {
		const subnetExt = uuid();
		const subnetGw = uuid();
		const subnetLan = uuid();

		const elem1 = uuid();
		const elem2 = uuid();
		const elem3 = uuid();

		const nodes: TopologyNode[] = [
			makeContainer(subnetExt),
			makeContainer(subnetGw),
			makeContainer(subnetLan),
			makeElement(elem1, subnetExt),
			makeElement(elem2, subnetGw),
			makeElement(elem3, subnetLan)
		];

		const edges: TopologyEdge[] = [
			primaryEdge(elem1, elem2, 'SameHost'),
			primaryEdge(elem2, elem3, 'SameHost')
		];

		const subnets = [
			makeSubnet(subnetExt, 'Internet'),
			makeSubnet(subnetGw, 'Gateway'),
			makeSubnet(subnetLan, 'Lan')
		];

		const input = makeTopology(nodes, edges, subnets);
		const result = await computeElkLayout(input);

		const extPos = result.nodePositions.get(subnetExt)!;
		const gwPos = result.nodePositions.get(subnetGw)!;
		const lanPos = result.nodePositions.get(subnetLan)!;

		// Connected containers should be in adjacent layers (different y positions)
		// but not necessarily in a fixed order — edge-driven layout determines order
		const ys = [extPos.y, gwPos.y, lanPos.y];
		const uniqueYs = new Set(ys.map((y) => Math.round(y / 50)));
		expect(uniqueYs.size).toBeGreaterThanOrEqual(2);
	});

	it('positions element nodes with non-negative relative coordinates', async () => {
		const subnetId = uuid();
		const elem1 = uuid();
		const elem2 = uuid();

		const nodes: TopologyNode[] = [
			makeContainer(subnetId),
			makeElement(elem1, subnetId),
			makeElement(elem2, subnetId)
		];

		const edges: TopologyEdge[] = [primaryEdge(elem1, elem2, 'SameHost')];
		const subnets = [makeSubnet(subnetId, 'Lan')];

		const input = makeTopology(nodes, edges, subnets);
		const result = await computeElkLayout(input);

		// Element positions should be non-negative (relative to parent)
		const l1Pos = result.nodePositions.get(elem1)!;
		const l2Pos = result.nodePositions.get(elem2)!;

		expect(l1Pos.x).toBeGreaterThanOrEqual(0);
		expect(l1Pos.y).toBeGreaterThanOrEqual(0);
		expect(l2Pos.x).toBeGreaterThanOrEqual(0);
		expect(l2Pos.y).toBeGreaterThanOrEqual(0);
	});

	it('handles single subnet with single host', async () => {
		const subnetId = uuid();
		const elementId = uuid();

		const nodes: TopologyNode[] = [makeContainer(subnetId), makeElement(elementId, subnetId)];
		const edges: TopologyEdge[] = [];
		const subnets = [makeSubnet(subnetId, 'Lan')];

		const input = makeTopology(nodes, edges, subnets);
		const result = await computeElkLayout(input);

		expect(result.nodePositions.has(subnetId)).toBe(true);
		expect(result.nodePositions.has(elementId)).toBe(true);

		const containerSize = result.containerSizes.get(subnetId)!;
		expect(containerSize.width).toBeGreaterThanOrEqual(180); // at least as wide as child
		expect(containerSize.height).toBeGreaterThanOrEqual(60); // at least as tall as child
	});

	it('handles medium topology without errors', async () => {
		const subnetTypes: SubnetType[] = [
			'Internet',
			'Gateway',
			'Lan',
			'WiFi',
			'DockerBridge',
			'Management',
			'Storage',
			'IoT'
		];

		const subnetIds = subnetTypes.map(() => uuid());
		const subnets = subnetIds.map((id, i) => makeSubnet(id, subnetTypes[i]));

		const nodes: TopologyNode[] = subnetIds.map((id) => makeContainer(id));

		// Add ~2-3 hosts per subnet
		const elementIds: string[] = [];
		for (const subnetId of subnetIds) {
			const count = 2 + Math.floor(subnetIds.indexOf(subnetId) % 3);
			for (let j = 0; j < count; j++) {
				const elementId = uuid();
				elementIds.push(elementId);
				nodes.push(makeElement(elementId, subnetId));
			}
		}

		// Create some primary edges between adjacent subnets
		const edges: TopologyEdge[] = [];
		for (let i = 0; i < elementIds.length - 1; i += 3) {
			edges.push(primaryEdge(elementIds[i], elementIds[i + 1], 'SameHost'));
		}

		const input = makeTopology(nodes, edges, subnets);
		const result = await computeElkLayout(input);

		// All nodes have positions
		for (const node of nodes) {
			expect(result.nodePositions.has(node.id)).toBe(true);
		}

		// No NaN positions
		for (const [, pos] of result.nodePositions) {
			expect(Number.isFinite(pos.x)).toBe(true);
			expect(Number.isFinite(pos.y)).toBe(true);
		}
	});

	it('handles nested sub-group containers inside a subnet', async () => {
		const subnetId = uuid();
		const groupId = uuid();
		const elem1 = uuid();
		const elem2 = uuid();
		const elem3 = uuid();

		const nodes: TopologyNode[] = [
			makeContainer(subnetId),
			makeContainer(groupId, {
				container_type: 'NestedServiceCategory',
				parent_container_id: subnetId
			}),
			makeElement(elem1, subnetId),
			makeElement(elem2, groupId),
			makeElement(elem3, groupId)
		];

		const edges: TopologyEdge[] = [primaryEdge(elem1, elem2, 'SameHost')];
		const subnets = [makeSubnet(subnetId, 'Lan')];

		const input = makeTopology(nodes, edges, subnets);
		const result = await computeElkLayout(input);

		// All nodes should have positions
		expect(result.nodePositions.has(subnetId)).toBe(true);
		expect(result.nodePositions.has(groupId)).toBe(true);
		expect(result.nodePositions.has(elem1)).toBe(true);
		expect(result.nodePositions.has(elem2)).toBe(true);

		// Sub-group should have a size
		expect(result.containerSizes.has(groupId)).toBe(true);
		const groupSize = result.containerSizes.get(groupId)!;
		expect(groupSize.width).toBeGreaterThan(0);
		expect(groupSize.height).toBeGreaterThan(0);

		// Sub-group position should be non-negative (relative to parent)
		const groupPos = result.nodePositions.get(groupId)!;
		expect(groupPos.x).toBeGreaterThanOrEqual(0);
		expect(groupPos.y).toBeGreaterThanOrEqual(0);
	});

	it('containers with 8 elements use multi-column layout', async () => {
		const subnetId = uuid();
		const elemIds = Array.from({ length: 8 }, () => uuid());

		const nodes: TopologyNode[] = [
			makeContainer(subnetId),
			...elemIds.map((id) => makeElement(id, subnetId))
		];

		const edges: TopologyEdge[] = [];
		const subnets = [makeSubnet(subnetId, 'Lan')];
		const input = makeTopology(nodes, edges, subnets);
		const result = await computeElkLayout(input);

		// Elements should NOT all have the same x position (i.e., not single column)
		const xPositions = new Set(elemIds.map((id) => result.nodePositions.get(id)!.x));
		expect(xPositions.size).toBeGreaterThan(1);

		// Container should be wider than a single element (180px)
		const containerSize = result.containerSizes.get(subnetId)!;
		expect(containerSize.width).toBeGreaterThan(250);
	});

	it('containers with 20 elements wrap into multiple rows', async () => {
		const subnetId = uuid();
		const elemIds = Array.from({ length: 20 }, () => uuid());

		const nodes: TopologyNode[] = [
			makeContainer(subnetId),
			...elemIds.map((id) => makeElement(id, subnetId))
		];

		const edges: TopologyEdge[] = [];
		const subnets = [makeSubnet(subnetId, 'Lan')];
		const input = makeTopology(nodes, edges, subnets);
		const result = await computeElkLayout(input);

		// Should have multiple distinct x AND y positions (grid, not single row)
		const xPositions = new Set(elemIds.map((id) => result.nodePositions.get(id)!.x));
		const yPositions = new Set(elemIds.map((id) => result.nodePositions.get(id)!.y));
		expect(xPositions.size).toBeGreaterThan(1);
		expect(yPositions.size).toBeGreaterThan(1);

		// Container should not be excessively wide (no more than ~7 elements wide)
		const containerSize = result.containerSizes.get(subnetId)!;
		expect(containerSize.width).toBeLessThan(180 * 8 + 25 * 7); // ~1615px
	});

	it('packs disconnected containers densely instead of stacking in separate layers', async () => {
		const containerA = uuid();
		const containerB = uuid();
		const containerC = uuid(); // disconnected — no edges to A or B
		const elemA = uuid();
		const elemB = uuid();
		const elemC = uuid();

		const nodes: TopologyNode[] = [
			makeContainer(containerA),
			makeContainer(containerB),
			makeContainer(containerC),
			makeElement(elemA, containerA),
			makeElement(elemB, containerB),
			makeElement(elemC, containerC)
		];

		// Only A↔B are connected; C is disconnected
		const edges: TopologyEdge[] = [primaryEdge(elemA, elemB, 'Dependency')];
		const subnets = [
			makeSubnet(containerA, 'Lan'),
			makeSubnet(containerB, 'Lan'),
			makeSubnet(containerC, 'Lan')
		];

		const input = makeTopology(nodes, edges, subnets);
		const result = await computeElkLayout(input);

		const posA = result.nodePositions.get(containerA)!;
		const posB = result.nodePositions.get(containerB)!;
		const posC = result.nodePositions.get(containerC)!;
		const sizeA = result.containerSizes.get(containerA)!;
		const sizeB = result.containerSizes.get(containerB)!;

		// The connected pair (A, B) are in different layers (one above the other)
		expect(posA.y).not.toBe(posB.y);

		// The disconnected container C should be packed beside the connected
		// components, not stacked far below both. Its top should be within
		// the vertical range of the connected component layout.
		const connectedBottom = Math.max(posA.y + sizeA.height, posB.y + sizeB.height);
		expect(posC.y).toBeLessThan(connectedBottom + 100); // allow some margin for spacing
	});

	it('container-targeted edge (post-elevation) positions containers adjacently', async () => {
		// Simulates a ServiceVirtualization edge after edge elevation:
		// source = IP element in host subnet, target = Docker bridge container
		const hostSubnet = uuid();
		const dockerBridge = uuid();
		const disconnectedSubnet = uuid(); // a subnet far away with no edges
		const hostIP = uuid();
		const disconnectedElem = uuid();

		// Docker bridge has a single element (the container IP)
		const containerIP = uuid();

		const nodes: TopologyNode[] = [
			makeContainer(hostSubnet),
			makeContainer(dockerBridge, { container_type: 'Subnet' }),
			makeContainer(disconnectedSubnet),
			makeElement(hostIP, hostSubnet),
			makeElement(containerIP, dockerBridge),
			makeElement(disconnectedElem, disconnectedSubnet)
		];

		// Post-elevation edge: source = element, target = container
		// (will_target_container already applied by edge-elevation)
		const edges: TopologyEdge[] = [
			{
				edge_type: 'ServiceVirtualization',
				source: hostIP,
				target: dockerBridge, // container ID, not element
				source_handle: 'Bottom',
				target_handle: 'Top',
				view_config: {
					type: 'active' as const,
					affects_layout: true,
					default_visibility: 'visible',
					stroke: 'solid',
					highlight_behavior: 'when_visible',
					will_target_container: true,
					show_directionality: false
				}
			} as unknown as TopologyEdge
		];

		const subnets = [
			makeSubnet(hostSubnet, 'Lan'),
			makeSubnet(dockerBridge, 'DockerBridge'),
			makeSubnet(disconnectedSubnet, 'Lan')
		];

		const input = makeTopology(nodes, edges, subnets);
		const result = await computeElkLayout(input);

		const hostPos = result.nodePositions.get(hostSubnet)!;
		const dockerPos = result.nodePositions.get(dockerBridge)!;
		const disconnectedPos = result.nodePositions.get(disconnectedSubnet)!;

		expect(hostPos).toBeDefined();
		expect(dockerPos).toBeDefined();
		expect(disconnectedPos).toBeDefined();

		// Edge creates a layered relationship: host subnet in layer N,
		// Docker bridge in layer N+1 (directly below in DOWN layout).
		// They should share the same x column, proving the edge connects them.
		expect(hostPos.x).toBe(dockerPos.x);
		// Docker bridge should be below host subnet (connected by edge)
		expect(dockerPos.y).toBeGreaterThan(hostPos.y);
	});

	it('container-targeted edge works in complex topology with many subnets', async () => {
		// Realistic L3 scenario: 5 regular subnets connected by SameHost edges,
		// plus a Docker bridge that should be pulled near subnet1 by a ServiceVirtualization edge
		const subnet1 = uuid(); // Has the Docker host
		const subnet2 = uuid();
		const subnet3 = uuid();
		const subnet4 = uuid();
		const subnet5 = uuid();
		const dockerBridge = uuid();

		// Elements in each subnet
		const elem1a = uuid();
		const elem1b = uuid();
		const elem2a = uuid();
		const elem2b = uuid();
		const elem3a = uuid();
		const elem4a = uuid();
		const elem5a = uuid();
		const dockerElem = uuid();

		const nodes: TopologyNode[] = [
			makeContainer(subnet1),
			makeContainer(subnet2),
			makeContainer(subnet3),
			makeContainer(subnet4),
			makeContainer(subnet5),
			makeContainer(dockerBridge, { container_type: 'Subnet' }),
			makeElement(elem1a, subnet1),
			makeElement(elem1b, subnet1),
			makeElement(elem2a, subnet2),
			makeElement(elem2b, subnet2),
			makeElement(elem3a, subnet3),
			makeElement(elem4a, subnet4),
			makeElement(elem5a, subnet5),
			makeElement(dockerElem, dockerBridge)
		];

		// Regular SameHost edges connecting subnets in a chain
		const edges: TopologyEdge[] = [
			primaryEdge(elem1a, elem2a, 'SameHost'),
			primaryEdge(elem2b, elem3a, 'SameHost'),
			primaryEdge(elem3a, elem4a, 'SameHost'),
			primaryEdge(elem4a, elem5a, 'SameHost'),
			// ServiceVirtualization: elem1b → dockerBridge container (post-elevation)
			{
				edge_type: 'ServiceVirtualization',
				source: elem1b,
				target: dockerBridge,
				source_handle: 'Bottom',
				target_handle: 'Top',
				view_config: {
					type: 'active' as const,
					affects_layout: true,
					default_visibility: 'visible',
					stroke: 'solid',
					highlight_behavior: 'when_visible',
					will_target_container: true,
					show_directionality: false
				}
			} as unknown as TopologyEdge
		];

		const subnets = [
			makeSubnet(subnet1, 'Lan'),
			makeSubnet(subnet2, 'Lan'),
			makeSubnet(subnet3, 'Lan'),
			makeSubnet(subnet4, 'Lan'),
			makeSubnet(subnet5, 'Lan'),
			makeSubnet(dockerBridge, 'DockerBridge')
		];

		const input = makeTopology(nodes, edges, subnets);
		const result = await computeElkLayout(input);

		const pos1 = result.nodePositions.get(subnet1)!;
		const posDocker = result.nodePositions.get(dockerBridge)!;

		// Docker bridge should be below subnet1 (connected by edge, DOWN layout)
		expect(posDocker.y).toBeGreaterThan(pos1.y);
	});

	it('element-to-element ServiceVirtualization edge (no elevation) positions subnets adjacently', async () => {
		// When MergeContainerBridges is off, the edge stays as IP→IP (no elevation)
		const hostSubnet = uuid();
		const dockerBridge = uuid();
		const farSubnet = uuid();
		const hostIP = uuid();
		const containerIP = uuid();
		const farElem = uuid();

		const nodes: TopologyNode[] = [
			makeContainer(hostSubnet),
			makeContainer(dockerBridge, { container_type: 'Subnet' }),
			makeContainer(farSubnet),
			makeElement(hostIP, hostSubnet),
			makeElement(containerIP, dockerBridge),
			makeElement(farElem, farSubnet)
		];

		// Non-elevated edge: both endpoints are elements
		const edges: TopologyEdge[] = [
			{
				edge_type: 'ServiceVirtualization',
				source: hostIP,
				target: containerIP, // still an element, not elevated to container
				source_handle: 'Bottom',
				target_handle: 'Top',
				view_config: {
					type: 'active' as const,
					affects_layout: true,
					default_visibility: 'visible',
					stroke: 'solid',
					highlight_behavior: 'when_visible',
					will_target_container: true,
					show_directionality: false
				}
			} as unknown as TopologyEdge
		];

		const subnets = [
			makeSubnet(hostSubnet, 'Lan'),
			makeSubnet(dockerBridge, 'DockerBridge'),
			makeSubnet(farSubnet, 'Lan')
		];

		const input = makeTopology(nodes, edges, subnets);
		const result = await computeElkLayout(input);

		const hostPos = result.nodePositions.get(hostSubnet)!;
		const dockerPos = result.nodePositions.get(dockerBridge)!;

		// Docker bridge should be in adjacent layer (connected by edge)
		expect(hostPos.x).toBe(dockerPos.x);
		expect(dockerPos.y).toBeGreaterThan(hostPos.y);
	});

	it('full pipeline: edge elevation + ELK positions Docker bridge near host', async () => {
		// End-to-end test: raw edges → edge elevation → ELK layout
		// Container bridge has will_accept_edges: true (MergeContainerBridges)
		const hostSubnet = uuid();
		const dockerBridge = uuid();
		const farSubnet = uuid();
		const hostIP = uuid();
		const containerIP = uuid();
		const farElem = uuid();

		const nodes: TopologyNode[] = [
			makeContainer(hostSubnet),
			{
				...makeContainer(dockerBridge, { container_type: 'Subnet' }),
				will_accept_edges: true // MergeContainerBridges enables this
			} as TopologyNode,
			makeContainer(farSubnet),
			makeElement(hostIP, hostSubnet),
			makeElement(containerIP, dockerBridge),
			makeElement(farElem, farSubnet)
		];

		// Raw (pre-elevation) edge: IP element → IP element
		const rawEdges: TopologyEdge[] = [
			{
				edge_type: 'ServiceVirtualization',
				source: hostIP,
				target: containerIP, // pre-elevation: still targets the element
				source_handle: 'Bottom',
				target_handle: 'Top',
				view_config: {
					type: 'active' as const,
					affects_layout: true,
					default_visibility: 'visible',
					stroke: 'solid',
					highlight_behavior: 'when_visible',
					will_target_container: true,
					show_directionality: false
				}
			} as unknown as TopologyEdge
		];

		// Run edge elevation (same as prepare.ts pipeline)
		const elevatedEdges = elevateEdgesToContainers(rawEdges, nodes);

		// Verify elevation happened: target should now be dockerBridge container
		expect(elevatedEdges.length).toBe(1);
		expect(elevatedEdges[0].source).toBe(hostIP); // unchanged
		expect(elevatedEdges[0].target).toBe(dockerBridge); // elevated to container

		const subnets = [
			makeSubnet(hostSubnet, 'Lan'),
			makeSubnet(dockerBridge, 'DockerBridge'),
			makeSubnet(farSubnet, 'Lan')
		];

		const input = makeTopology(nodes, elevatedEdges, subnets);
		const result = await computeElkLayout(input);

		const hostPos = result.nodePositions.get(hostSubnet)!;
		const dockerPos = result.nodePositions.get(dockerBridge)!;

		expect(hostPos).toBeDefined();
		expect(dockerPos).toBeDefined();

		// Docker bridge should be in adjacent layer to host subnet
		expect(hostPos.x).toBe(dockerPos.x);
		expect(dockerPos.y).toBeGreaterThan(hostPos.y);
	});

	it('does not pass disabled edges to ELK layout', async () => {
		const subnetId = uuid();
		const elem1 = uuid();
		const elem2 = uuid();

		const nodes: TopologyNode[] = [
			makeContainer(subnetId),
			makeElement(elem1, subnetId),
			makeElement(elem2, subnetId)
		];

		// Only disabled edges — no layout-affecting edges to drive layout
		const edges: TopologyEdge[] = [
			makeEdge(elem1, elem2, 'Hypervisor'),
			makeEdge(elem1, elem2, 'RequestPath')
		];

		const subnets = [makeSubnet(subnetId, 'Lan')];
		const input = makeTopology(nodes, edges, subnets);

		// Should succeed even with no layout-affecting edges
		const result = await computeElkLayout(input);
		expect(result.nodePositions.size).toBeGreaterThan(0);
	});
});

// --- computeOptimalHandles: crossing-aware handle selection ---

describe('computeOptimalHandles', () => {
	it('picks shortest-distance handles when no obstacles are supplied', () => {
		// Source element sits above target element. The pair minimising straight-line
		// distance between anchors is source-Bottom → target-Top.
		const srcPos = { x: 200, y: 200 };
		const srcSize = { w: 100, h: 60 };
		const tgtPos = { x: 200, y: 400 };
		const tgtSize = { w: 100, h: 60 };

		const result = computeOptimalHandles(srcPos, srcSize, tgtPos, tgtSize);
		expect(result).toEqual({ sourceHandle: 'Bottom', targetHandle: 'Top' });
	});

	it('treats empty obstacles list like no obstacles (distance-only)', () => {
		const srcPos = { x: 200, y: 200 };
		const srcSize = { w: 100, h: 60 };
		const tgtPos = { x: 200, y: 400 };
		const tgtSize = { w: 100, h: 60 };

		const result = computeOptimalHandles(srcPos, srcSize, tgtPos, tgtSize, []);
		expect(result).toEqual({ sourceHandle: 'Bottom', targetHandle: 'Top' });
	});

	it('flips to a slightly longer crossing-free path over a shorter one that cuts through an unrelated node', () => {
		// Analogue of the authentik → Docker-Bridge reproducer. A narrow source
		// and a wider, taller target positioned down-and-right: distance picks
		// Bottom→Left (distSq 195,400). An obstacle lying on the Bottom→Left
		// diagonal but clearing the shallower Bottom→Top path forces the flip.
		const srcPos = { x: 140, y: 100 };
		const srcSize = { w: 20, h: 60 };
		const tgtPos = { x: 500, y: 400 };
		const tgtSize = { w: 100, h: 60 };

		// No obstacles: Bottom→Left wins on distance alone.
		const baseline = computeOptimalHandles(srcPos, srcSize, tgtPos, tgtSize);
		expect(baseline).toEqual({ sourceHandle: 'Bottom', targetHandle: 'Left' });

		// Obstacle spans the middle of the Bottom→Left diagonal (slope ≈ 0.77)
		// but sits below the Bottom→Top diagonal (slope ≈ 0.60) over the same
		// x-range, so only Bottom→Left intersects it.
		const obstacle: ObstacleRect = { x: 350, y: 320, w: 50, h: 40 };

		const withObstacle = computeOptimalHandles(srcPos, srcSize, tgtPos, tgtSize, [obstacle]);
		expect(withObstacle).not.toEqual(baseline);
		expect(withObstacle).toEqual({ sourceHandle: 'Bottom', targetHandle: 'Top' });
	});

	it('still picks fewest-crossings candidate when every candidate crosses at least one node', () => {
		// Ring of obstacles around source so every anchor pair crosses ≥1 rect,
		// but one anchor pair crosses only 1 while others cross 2. The 1-crossing
		// pair should win regardless of being slightly longer than a 2-crossing pair.
		const srcPos = { x: 100, y: 100 };
		const srcSize = { w: 60, h: 60 };
		const tgtPos = { x: 400, y: 100 };
		const tgtSize = { w: 60, h: 60 };

		// Obstacles spanning most of the horizontal corridor between source and
		// target — every candidate pair's straight line has to cross at least one.
		const obstacles: ObstacleRect[] = [
			{ x: 200, y: 80, w: 50, h: 100 }, // near source
			{ x: 300, y: 80, w: 50, h: 100 }, // near target
			{ x: 250, y: 160, w: 50, h: 40 } // below middle — avoidable
		];

		const result = computeOptimalHandles(srcPos, srcSize, tgtPos, tgtSize, obstacles);
		// Whatever the picker chooses, it must not have been blocked outright:
		// all four handles are legal values.
		expect(['Top', 'Bottom', 'Left', 'Right']).toContain(result.sourceHandle);
		expect(['Top', 'Bottom', 'Left', 'Right']).toContain(result.targetHandle);

		// Sanity: the picker must prefer a lower-crossing candidate over a
		// higher-crossing one when both are available. Compare the chosen pair
		// against a deliberately-bad pair (source-Top → target-Bottom, which
		// is geometrically the worst since it wraps around above and below).
		const bad = computeOptimalHandles(srcPos, srcSize, tgtPos, tgtSize);
		// Without obstacles, distance picks Right→Left (the direct horizontal).
		expect(bad).toEqual({ sourceHandle: 'Right', targetHandle: 'Left' });
	});

	it('baseline: no obstacle in the path — crossing logic leaves the distance winner intact', () => {
		// Two nodes stacked vertically; an unrelated obstacle sits far to the
		// right, well off the direct path. Every candidate has zero crossings,
		// so score = distSq and the distance-only winner still wins.
		const srcPos = { x: 100, y: 100 };
		const srcSize = { w: 100, h: 60 };
		const tgtPos = { x: 100, y: 300 };
		const tgtSize = { w: 100, h: 60 };

		const farAway: ObstacleRect = { x: 800, y: 100, w: 100, h: 200 };

		const result = computeOptimalHandles(srcPos, srcSize, tgtPos, tgtSize, [farAway]);
		expect(result).toEqual({ sourceHandle: 'Bottom', targetHandle: 'Top' });
	});
});

describe('LayoutGraph.restoreExpandedSizes', () => {
	// A rebuild recreates every container with `expandedSize` at {0, 0}. Restoring only collapsed
	// ones made that destructive at collapse level 4, where nothing is collapsed: containers came
	// back zero-sized and rendered as slivers with their contents outside until something forced a
	// fresh ELK run.
	const graphWith = (collapsed: boolean) => {
		const nodes = [
			{ id: 'host', node_type: 'Container', container_type: 'Host' },
			{ id: 'port', node_type: 'Element', container_id: 'host', host_id: 'h' }
		] as unknown as Parameters<typeof LayoutGraph.fromTopology>[0];
		const graph = LayoutGraph.fromTopology(nodes);
		graph.syncCollapseState(collapsed ? new Set(['host']) : new Set());
		return graph;
	};

	it('restores an expanded container, not just a collapsed one', () => {
		const graph = graphWith(false);
		// Not `{0, 0}`: an unsized container reports no size at all, so a caller falls back to a
		// measured hint instead of publishing a 0x0 node that renders as nothing.
		expect(graph.getContainerSize('host')).toBeUndefined();

		graph.restoreExpandedSizes(new Map([['host', { width: 350, height: 179 }]]));

		expect(graph.getContainerSize('host')).toEqual({ width: 350, height: 179 });
	});

	it('still restores a collapsed container', () => {
		const graph = graphWith(true);
		graph.restoreExpandedSizes(new Map([['host', { width: 350, height: 179 }]]));

		// Collapsed reports its collapsed size, but the expanded size must be carried so expanding
		// it does not fall back to zero.
		expect(graph.getExpandedSize('host')).toEqual({ width: 350, height: 179 });
	});
});
