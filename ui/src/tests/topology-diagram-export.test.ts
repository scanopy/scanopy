import { describe, it, expect } from 'vitest';
import { unzipSync, strFromU8 } from 'fflate';
import type { Edge } from '@xyflow/svelte';
import {
	buildDiagramModel,
	type DiagramDescriber,
	type DiagramModel,
	type DiagramPart,
	type DiagramSourceEdge,
	type DiagramSourceNode
} from '#lib/features/topology/diagram-export/model.ts';
import { serializeDrawio, toDrawioCells } from '#lib/features/topology/diagram-export/drawio.ts';
import { fieldKeys, hexColor } from '#lib/features/topology/diagram-export/xml.ts';
import { buildVsdxParts, zipVsdx } from '#lib/features/topology/diagram-export/vsdx.ts';
import {
	createDescriber,
	type DescriberInput
} from '#lib/features/topology/diagram-export/describe.ts';
import { base64DataUri } from '#lib/features/topology/diagram-export/collect.ts';
import type { RenderableTopology, TopologyNode } from '#lib/features/topology/types/base.ts';
import { activeView } from '#lib/features/topology/queries.ts';
import { expandedInlineGroups, inlineGroupKey } from '#lib/features/topology/collapse.ts';
import { elementMarks } from '#lib/features/topology/element-marks.ts';
import { queryClient, queryKeys } from '#lib/api/query-client.ts';
import { createColorHelper } from '#lib/shared/utils/styling.ts';
import type { Site } from '#lib/features/sites/types.ts';
import { topologyLookup } from '#lib/features/topology/resolvers.ts';
import { entityTooltip } from '#lib/shared/entity-tooltip.ts';
import { subnetTypes } from '#lib/shared/stores/metadata.ts';

const SUBNET = {
	id: 'lan',
	cidr: '10.0.0.0/24',
	subnet_type: subnetTypes.getItems()[0].id,
	vlan_ids: ['v10']
};

/**
 * A subnet holding a host card and a nested subcontainer (holding another card), plus a second
 * subnet with one card. Positions are absolute flow coordinates, as `collectSourceNodes` emits.
 */
const NODES: DiagramSourceNode[] = [
	// Child listed before its parent: the model must still order parents first.
	{ id: 'web', type: 'Element', parentId: 'lan', x: 140, y: 160, width: 250, height: 80 },
	{ id: 'lan', type: 'Container', x: 100, y: 100, width: 700, height: 400 },
	{ id: 'stack', type: 'Container', parentId: 'lan', x: 420, y: 140, width: 340, height: 200 },
	{ id: 'db', type: 'Element', parentId: 'stack', x: 460, y: 200, width: 250, height: 80 },
	{ id: 'dmz', type: 'Container', x: 900, y: 100, width: 320, height: 200 },
	{ id: 'proxy', type: 'Element', parentId: 'dmz', x: 930, y: 150, width: 250, height: 80 }
];

const EDGES: DiagramSourceEdge[] = [
	{ id: 'e-web-db', source: 'web', target: 'db', sourceHandle: 'Right', targetHandle: 'Left' },
	{ id: 'e-proxy-web', source: 'proxy', target: 'web', sourceHandle: 'Left', targetHandle: 'Top' },
	{ id: 'e-hidden', source: 'proxy', target: 'db' },
	// Endpoint inside a collapsed container: its node is not in the store.
	{ id: 'e-dangling', source: 'proxy', target: 'collapsed-child' }
];

const plainDescriber = (hiddenEdgeIds: Set<string> = new Set(['e-hidden'])): DiagramDescriber => ({
	node: (n) => ({
		box: { outline: 'rect', radius: 8, fill: '#ffffff', stroke: '#94a3b8', strokeWidth: 1 },
		label: {
			top: 8,
			lines: [
				{ value: n.id, color: '#1f2937', size: 12 },
				...(n.type === 'Element' ? [{ value: '10.0.0.1', color: '#64748b', size: 12 }] : [])
			]
		},
		parts:
			n.id === 'web'
				? [
						{ role: 'tile', outline: 'rect', x: 10, y: 10, width: 48, height: 48, fill: '#e5e7eb' },
						{
							role: 'glyph',
							outline: 'rect',
							x: 20,
							y: 20,
							width: 28,
							height: 28,
							iconKey: 'icon-web'
						},
						{
							role: 'status',
							outline: 'ellipse',
							x: 5,
							y: 5,
							width: 10,
							height: 10,
							fill: '#22c55e'
						}
					]
				: [],
		// Two labels that reduce to the same key, and one that collides with a draw.io attribute.
		data:
			n.type === 'Element'
				? [
						{ label: 'IP Address', value: '10.0.0.1' },
						{ label: 'IP-Address', value: '10.0.0.2' },
						{ label: 'label', value: 'x & <y>' }
					]
				: []
	}),
	edge: (e) => (hiddenEdgeIds.has(e.id) ? null : { color: '#3b82f6', width: 2, dash: 'solid' })
});

function sampleModel(): DiagramModel {
	return buildDiagramModel({
		name: 'HQ · L3 Logical',
		nodes: NODES,
		edges: EDGES,
		describe: plainDescriber(),
		background: '#f8fafc',
		header: { title: 'HQ · L3 Logical', subtitle: 'now', color: '#000', subtitleColor: '#666' },
		icons: new Map([
			[
				'icon-web',
				{ dataUri: 'data:image/svg+xml;base64,PHN2Zy8+', png: new Uint8Array([137, 80, 78, 71]) }
			]
		])
	});
}

const encloses = (
	outer: { x: number; y: number; width: number; height: number },
	inner: { x: number; y: number; width: number; height: number }
) =>
	inner.x >= outer.x &&
	inner.y >= outer.y &&
	inner.x + inner.width <= outer.x + outer.width &&
	inner.y + inner.height <= outer.y + outer.height;

describe('diagram model', () => {
	it('keeps each shape inside the container that holds it, parents first', () => {
		const model = sampleModel();
		const byId = new Map(model.shapes.map((s) => [s.id, s]));
		const seen = new Set<string>();
		for (const shape of model.shapes) {
			if (shape.parentId) {
				expect(seen.has(shape.parentId)).toBe(true);
				const parent = byId.get(shape.parentId)!;
				expect(parent.type).toBe('container');
				expect(encloses(parent, shape)).toBe(true);
			}
			seen.add(shape.id);
		}
		expect(byId.get('db')?.parentId).toBe('stack');
		expect(byId.get('stack')?.parentId).toBe('lan');
	});

	it('only connects shapes that exist, and leaves out undrawn and dangling edges', () => {
		const model = sampleModel();
		const ids = new Set(model.shapes.map((s) => s.id));
		for (const c of model.connectors) {
			expect(ids.has(c.sourceId)).toBe(true);
			expect(ids.has(c.targetId)).toBe(true);
		}
		expect(model.connectors.map((c) => c.id).sort()).toEqual(['e-proxy-web', 'e-web-db']);
	});

	it('places all content inside the page, below the header when there is one', () => {
		const model = sampleModel();
		for (const s of model.shapes) {
			expect(encloses({ x: 0, y: 0, width: model.width, height: model.height }, s)).toBe(true);
		}
		const top = Math.min(...model.shapes.map((s) => s.y));
		const withoutHeader = buildDiagramModel({
			name: 'x',
			nodes: NODES,
			edges: EDGES,
			describe: plainDescriber(),
			background: '#fff'
		});
		expect(top).toBeGreaterThan(Math.min(...withoutHeader.shapes.map((s) => s.y)));
	});

	it('treats a node whose parent is not drawn as a root', () => {
		const model = buildDiagramModel({
			name: 'x',
			nodes: [
				{ id: 'orphan', type: 'Element', parentId: 'gone', x: 0, y: 0, width: 10, height: 10 }
			],
			edges: [],
			describe: plainDescriber(),
			background: '#fff'
		});
		expect(model.shapes[0].parentId).toBeNull();
	});
});

describe('diagram describer', () => {
	const topology = {
		dependencies: [],
		options: { request: { element_rules: [] } },
		hosts: [],
		ip_addresses: [],
		services: [],
		ports: [],
		bindings: [],
		entity_tags: [],
		neighbours: [],
		nodes: [],
		edges: [],
		interfaces: [
			{ id: 'i1', display_name: 'Gi0/1' },
			{ id: 'i2', display_name: 'eth0' }
		],
		vlans: [{ id: 'v10', vlan_number: 10, name: 'users' }],
		subnets: [SUBNET]
	} as unknown as RenderableTopology;
	const flowEdge = (id: string, data: Record<string, unknown>): Edge =>
		({ id, source: 'a', target: 'b', data }) as unknown as Edge;
	const activeLink = { type: 'active', stroke: 'solid' };

	function describerFor(
		edges: Edge[],
		hiddenEdgeTypes: string[],
		nodeData: Map<string, TopologyNode> = new Map()
	) {
		return createDescriber({
			topology,
			nodeData,
			edges: new Map(edges.map((e) => [e.id, e])),
			hiddenEdgeTypes,
			theme: 'light',
			glyph: () => undefined
		});
	}

	it('carries the facts of the entity each shape stands for, read through the registry', () => {
		const link = flowEdge('link', {
			edge_type: 'PhysicalLink',
			view_config: activeLink,
			source_entity_id: 'i1',
			target_entity_id: 'i2',
			protocol: 'Lldp'
		});
		const vlanBox = {
			node_type: 'Container',
			container_type: 'VLAN',
			entity_id: 'v10',
			header: 'VLAN 10 (users)'
		} as unknown as TopologyNode;
		const describer = describerFor([link], [], new Map([['vlan', vlanBox]]));

		const edgeValues = describer
			.edge({ id: 'link', source: 'a', target: 'b' })
			?.data?.map((d) => d.value);
		expect(edgeValues).toEqual(expect.arrayContaining(['Gi0/1', 'eth0']));

		const boxValues = describer
			.node({ id: 'vlan', type: 'Container', x: 0, y: 0, width: 10, height: 10 })
			.data?.map((d) => d.value);
		expect(boxValues).toEqual(expect.arrayContaining(['10', 'users']));
	});

	it('exports the values a tooltip draws as tags and entities as their plain text', () => {
		const subnetBox = {
			node_type: 'Container',
			container_type: 'Subnet',
			entity_id: SUBNET.id,
			header: SUBNET.cidr
		} as unknown as TopologyNode;
		const describer = describerFor([], [], new Map([['lan', subnetBox]]));
		const shapeValues = describer
			.node({ id: 'lan', type: 'Container', x: 0, y: 0, width: 10, height: 10 })
			.data?.map((d) => d.value);

		const content = entityTooltip('Subnet', SUBNET, { lookup: topologyLookup(topology) });
		const rows = content.sections.flatMap((s) => s.rows ?? []);
		const chipRow = (type: string) => rows.find((r) => r.chips?.some((c) => c.type === type));
		expect(chipRow('Tag')?.value).toBe(subnetTypes.getName(SUBNET.subnet_type));
		expect(chipRow('Entity')?.value).toBeTruthy();
		expect(shapeValues).toEqual(
			expect.arrayContaining([chipRow('Tag')?.value, chipRow('Entity')?.value])
		);
	});

	it('leaves out edges whose type is hidden and dependency preview edges', () => {
		const edges = [
			flowEdge('shown', { edge_type: 'PhysicalLink', view_config: activeLink }),
			flowEdge('hidden-type', { edge_type: 'NeighborLink', view_config: activeLink }),
			flowEdge('preview', { edge_type: 'RequestPath', view_config: activeLink, is_preview: true })
		];
		const describer = describerFor(edges, ['NeighborLink']);
		const drawn = edges.filter((e) =>
			describer.edge({ id: e.id, source: e.source, target: e.target })
		);
		expect(drawn.map((e) => e.id)).toEqual(['shown']);
		const shown = describer.edge({ id: 'shown', source: 'a', target: 'b' });
		expect(shown?.data?.map((d) => d.value)).toEqual([expect.any(String)]);
	});
});

describe('draw.io cells', () => {
	it('references only cells that exist, with child geometry relative to its parent', () => {
		const model = sampleModel();
		const cells = toDrawioCells(model);
		const byId = new Map(cells.map((c) => [c.id, c]));
		expect(byId.size).toBe(cells.length);

		for (const cell of cells) {
			if (cell.parent) expect(byId.has(cell.parent)).toBe(true);
			if (cell.edge) {
				expect(byId.get(cell.source!)?.vertex).toBe(true);
				expect(byId.get(cell.target!)?.vertex).toBe(true);
			}
		}

		// Walking up the parent chain reconstructs each shape's absolute position.
		const absolute = (id: string): { x: number; y: number } => {
			const cell = byId.get(id)!;
			const parent = byId.get(cell.parent);
			const base = parent?.geometry ? absolute(parent.id) : { x: 0, y: 0 };
			return { x: base.x + cell.geometry!.x, y: base.y + cell.geometry!.y };
		};
		for (const shape of model.shapes) {
			expect(absolute(`n-${shape.id}`)).toEqual({ x: shape.x, y: shape.y });
		}
	});

	it('marks containers as containers so they carry their children', () => {
		const model = sampleModel();
		const cells = toDrawioCells(model);
		const holders = new Set(model.shapes.map((s) => s.parentId).filter(Boolean));
		expect(holders.size).toBeGreaterThan(0);
		for (const id of holders) {
			expect(cells.find((c) => c.id === `n-${id}`)?.style).toContain('container=1');
		}
	});
});

/**
 * The exports redraw what the canvas draws: the same parts, in both formats, from the render data
 * the canvas components read.
 */
describe('canvas styling in the export', () => {
	const SITE = { id: 'site', effective_stale_after_hours: 24 } as unknown as Site;
	const NOW = new Date().toISOString();
	const LONG_AGO = new Date(Date.now() - 30 * 24 * 3600 * 1000).toISOString();
	const base = { site_id: SITE.id, tags: [], last_seen_at: NOW, source: { type: 'Discovery' } };
	const element = (fields: Record<string, unknown>) =>
		({ node_type: 'Element', ...fields }) as unknown as TopologyNode;
	const box = (fields: Record<string, unknown>) =>
		({ node_type: 'Container', ...fields }) as unknown as TopologyNode;

	const nodes = {
		up: element({
			element_type: 'Interface',
			host_id: 'switch',
			entity_id: 'if-up',
			header: 'Gi1'
		}),
		down: element({
			element_type: 'Interface',
			host_id: 'switch',
			entity_id: 'if-down',
			header: 'Gi2'
		}),
		address: element({
			element_type: 'IPAddress',
			host_id: 'guest',
			entity_id: 'ip',
			ip_address_id: 'ip',
			subnet_id: 'lan'
		}),
		guest: element({
			element_type: 'Host',
			host_id: 'guest',
			entity_id: 'guest',
			inline_groups: [
				{ entity_id: 'docker', entity_type: 'Service', group_id: 'docker', role: 'Header' },
				{ entity_id: 'nginx', entity_type: 'Service', group_id: 'docker', role: 'Member' }
			]
		}),
		subnet: box({ container_type: 'Subnet', entity_id: 'lan', header: '10.0.0.0/24' }),
		oldSwitch: box({ container_type: 'Host', entity_id: 'old-switch', header: 'old-switch' }),
		vlan: box({ container_type: 'VLAN', entity_id: 'vlan-10', header: 'VLAN 10' }),
		runtime: box({
			container_type: 'ContainerRuntime',
			header: 'Docker',
			associated_service_definition: 'Docker'
		})
	};

	const topology = {
		id: 'topo',
		site_id: SITE.id,
		options: { request: { element_rules: [] } },
		hosts: [
			{ ...base, id: 'guest', name: 'guest', display_name: 'guest' },
			{ ...base, id: 'switch', name: 'switch', display_name: 'switch' },
			{ ...base, id: 'old-switch', name: 'old-switch', last_seen_at: LONG_AGO }
		],
		subnets: [{ ...base, id: 'lan', cidr: '10.0.0.0/24', vlan_ids: ['vlan-10'] }],
		ip_addresses: [
			{ ...base, id: 'ip', host_id: 'guest', subnet_id: 'lan', ip_address: '10.0.0.5' }
		],
		services: [
			{
				...base,
				id: 'web',
				host_id: 'guest',
				name: 'web',
				service_definition: 'Nginx',
				bindings: [{ type: 'Port', port_id: 'p80', ip_address_id: 'ip' }]
			},
			{
				...base,
				id: 'docker',
				host_id: 'guest',
				name: 'Docker Daemon',
				service_definition: 'Docker',
				bindings: []
			},
			{
				...base,
				id: 'nginx',
				host_id: 'guest',
				name: 'nginx',
				service_definition: 'Nginx',
				bindings: []
			}
		],
		ports: [{ id: 'p80', number: 80, protocol: 'Tcp' }],
		bindings: [],
		interfaces: [
			{
				...base,
				id: 'if-up',
				host_id: 'switch',
				display_name: 'Gi1',
				oper_status: 'Up',
				filter_values: { LinkState: 'Linked', OperStatus: 'Up' }
			},
			{
				...base,
				id: 'if-down',
				host_id: 'switch',
				display_name: 'Gi2',
				oper_status: 'Down',
				last_seen_at: LONG_AGO,
				filter_values: { LinkState: 'Unlinked', OperStatus: 'Down' }
			}
		],
		dependencies: [],
		vlans: [{ ...base, id: 'vlan-10', vlan_number: 10, subnet_ids: ['lan'] }],
		entity_tags: [],
		neighbours: [],
		nodes: Object.entries(nodes).map(([id, node]) => ({ ...node, id })),
		edges: []
	} as unknown as RenderableTopology;

	// What `cardGlyph` reads off the app's icon components: Docker's logo comes as a light and a
	// dark file, Nginx's sits on its own background colour, everything else is a glyph in the text
	// colour.
	const NGINX_FILL = '#009639';
	const glyph: DescriberInput['glyph'] = (key) =>
		key === 'service:Docker'
			? {
					type: 'image',
					key,
					src: '/logos/docker.svg',
					darkSrc: '/logos/docker-dark.svg',
					fill: null,
					round: false,
					css: '',
					darkCss: ''
				}
			: key === 'service:Nginx'
				? {
						type: 'image',
						key,
						src: '/logos/nginx.svg',
						darkSrc: null,
						fill: NGINX_FILL,
						round: false,
						css: '',
						darkCss: null
					}
				: { type: 'mask', key, svg: '<svg stroke="currentColor"/>', css: '' };

	const CARD = { x: 0, y: 0, width: 120, height: 110 };
	const BOX = { x: 0, y: 0, width: 400, height: 300 };

	function describerFor(
		view: 'L2Physical' | 'L3Logical' | 'Workloads',
		theme: 'light' | 'dark' = 'light',
		edges: Edge[] = []
	) {
		activeView.set(view);
		queryClient.setQueryData(queryKeys.sites.all, [SITE]);
		return createDescriber({
			topology,
			nodeData: new Map(
				Object.entries(nodes).map(([id, node]) => [id, { ...node, id } as TopologyNode])
			),
			edges: new Map(edges.map((e) => [e.id, e])),
			hiddenEdgeTypes: [],
			theme,
			glyph
		});
	}
	const card = (describer: ReturnType<typeof describerFor>, id: keyof typeof nodes) =>
		describer.node({ id, type: 'Element', ...CARD });
	const container = (describer: ReturnType<typeof describerFor>, id: keyof typeof nodes) =>
		describer.node({ id, type: 'Container', ...BOX });
	const withRole = (description: { parts: DiagramPart[] }, role: DiagramPart['role']) =>
		description.parts.filter((p) => p.role === role);

	it('draws an icon card as a centred tile with its label below it', () => {
		const port = card(describerFor('L2Physical'), 'up');
		const [tile] = withRole(port, 'tile');
		expect(tile.x + tile.width / 2).toBe(CARD.width / 2);
		expect(port.label!.top).toBeGreaterThanOrEqual(tile.y + tile.height);
		expect(port.label!.lines[0].value).toBe('Gi1');
		// The glyph sits inside the tile.
		expect(encloses(tile, withRole(port, 'glyph')[0])).toBe(true);
	});

	it("draws a port's link state as a dot in the colour of the view's Status mark", () => {
		const describer = describerFor('L2Physical');
		const dot = (id: 'up' | 'down') =>
			withRole(card(describer, id), 'status').find((p) => p.fill !== undefined && !p.stroke);
		const marks = (id: 'up' | 'down') =>
			elementMarks('L2Physical', { ...nodes[id], id } as TopologyNode, [SITE], topology);
		expect(dot('up')?.fill).toBe(createColorHelper(marks('up').Status!.color).rgb);
		expect(dot('down')?.fill).toBe(createColorHelper(marks('down').Status!.color).rgb);
		expect(dot('up')?.fill).not.toBe(dot('down')?.fill);
	});

	it('draws one badge for each service the card stacks on its tile', () => {
		const describer = describerFor('L3Logical');
		const address = card(describer, 'address');
		const badges = withRole(address, 'badge');
		expect(badges).toHaveLength(1);
		expect(badges[0].outline).toBe('ellipse');
		// A logo with a background colour fills its frame with it.
		expect(badges[0].fill).toBe(NGINX_FILL);
	});

	it("draws a guest's containers as rows, a device square and software round", () => {
		expandedInlineGroups.set(new Set([inlineGroupKey('guest', 'docker')]));
		const guest = describer_workloads();
		expect(withRole(guest, 'tile')[0].outline).toBe('rect');
		expect(withRole(guest, 'group')).toHaveLength(1);
		const rows = withRole(guest, 'row');
		expect(rows).toHaveLength(1);
		expect(rows[0].outline).toBe('ellipse');
		expect(guest.parts.some((p) => p.text?.value === 'nginx')).toBe(true);
		expandedInlineGroups.set(new Set());

		function describer_workloads() {
			return describerFor('Workloads').node({ id: 'guest', type: 'Element', ...CARD, height: 180 });
		}
	});

	it("titles a root container with its entity's pill above the box, a subcontainer inside it", () => {
		const describer = describerFor('L2Physical');
		const subnet = container(describer, 'subnet');
		const [pill] = withRole(subnet, 'title');
		expect(pill.text?.value).toBe('10.0.0.0/24');
		expect(pill.fill).toBeDefined();
		expect(pill.y + pill.height).toBeLessThanOrEqual(0);
		expect(subnet.box.dashed).toBeFalsy();

		const vlan = container(describer, 'vlan');
		const [inline] = withRole(vlan, 'title');
		expect(inline.text?.value).toBe('VLAN 10');
		expect(encloses(BOX, inline)).toBe(true);
		expect(vlan.box.dashed).toBe(true);
	});

	it('marks a stale card and a stale container with the status badge the canvas draws', () => {
		const describer = describerFor('L2Physical');
		expect(withRole(card(describer, 'down'), 'freshness')).toHaveLength(1);
		expect(withRole(card(describer, 'up'), 'freshness')).toHaveLength(0);
		expect(withRole(container(describer, 'oldSwitch'), 'freshness')).toHaveLength(1);
		expect(withRole(container(describer, 'subnet'), 'freshness')).toHaveLength(0);
	});

	it("gives a container header the logo file for the export's theme", () => {
		for (const [theme, src] of [
			['light', '/logos/docker.svg'],
			['dark', '/logos/docker-dark.svg']
		] as const) {
			const describer = describerFor('Workloads', theme);
			const runtime = container(describer, 'runtime');
			expect(withRole(runtime, 'glyph').map((p) => p.iconKey)).toEqual([src]);
			expect([...describer.icons.keys()]).toEqual([src]);
		}
	});

	it('leaves a bundle unlabelled, and colours every other edge label for the theme', () => {
		const data = { edge_type: 'PhysicalLink', view_config: { type: 'active', stroke: 'solid' } };
		const edges = [
			{ id: 'link', source: 'up', target: 'down', label: 'Gi1 ↔ Gi2', data },
			{
				id: 'bundle',
				source: 'up',
				target: 'down',
				label: 'Gi1 ↔ Gi2',
				data: { ...data, isBundle: true }
			}
		] as unknown as Edge[];
		const describer = describerFor('L2Physical', 'dark', edges);
		expect(describer.edge(edges[1])?.label).toBeUndefined();

		const model = buildDiagramModel({
			name: 'x',
			nodes: [
				{ id: 'up', type: 'Element', ...CARD },
				{ id: 'down', type: 'Element', ...CARD, x: 300 }
			],
			edges,
			describe: describer,
			background: '#15131e'
		});
		const link = toDrawioCells(model).find((c) => c.id === 'e-link')!;
		const color = describer.edge(edges[0])!.labelStyle!.color;
		expect(link.value).toBe('Gi1 ↔ Gi2');
		expect(link.style).toContain(`fontColor=${hexColor(color)}`);
		expect(hexColor(color)).not.toBe('#000000');
	});

	it('draws the same parts in both formats', () => {
		const describer = describerFor('L2Physical');
		const model = buildDiagramModel({
			name: 'x',
			nodes: [
				{ id: 'subnet', type: 'Container', ...BOX },
				{
					id: 'vlan',
					type: 'Container',
					parentId: 'subnet',
					x: 20,
					y: 60,
					width: 300,
					height: 200
				},
				{ id: 'up', type: 'Element', parentId: 'vlan', ...CARD, x: 40, y: 100 },
				{ id: 'down', type: 'Element', parentId: 'vlan', ...CARD, x: 180, y: 100 }
			],
			edges: [],
			describe: describer,
			background: '#f8fafc'
		});
		// The descriptions name their icons as the model is built, so the images come after it.
		model.icons = new Map(
			[...describer.icons.keys()].map((key) => [
				key,
				{ dataUri: 'data:image/svg+xml;base64,PHN2Zy8+', png: new Uint8Array([137, 80, 78, 71]) }
			])
		);
		const parts = model.shapes.flatMap((s) => s.parts);
		expect(parts.length).toBeGreaterThan(model.shapes.length);

		const cells = toDrawioCells(model).filter((c) => c.id.startsWith('p-'));
		expect(cells.map((c) => c.geometry)).toEqual(
			parts.map(({ x, y, width, height }) => ({ x, y, width, height }))
		);

		const page = strFromU8(unzipSync(zipVsdx(buildVsdxParts(model)))['visio/pages/page1.xml']);
		const drawn = (name: string) => [...page.matchAll(new RegExp(`NameU="${name}\\.\\d+"`, 'g'))];
		for (const role of new Set(parts.filter((p) => !p.iconKey).map((p) => p.role))) {
			expect(drawn(role)).toHaveLength(parts.filter((p) => !p.iconKey && p.role === role).length);
		}
		expect(drawn('Icon')).toHaveLength(parts.filter((p) => p.iconKey).length);
	});

	it('keeps an image usable in a draw.io style, whatever form its data URI takes', () => {
		const svg = '<svg xmlns="http://www.w3.org/2000/svg"><path d="M0 0;L1 1"/></svg>';
		const uri = base64DataUri(`data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`);
		expect(uri.replace(';base64,', ',')).not.toContain(';');
		expect(atob(uri.slice(uri.indexOf(',') + 1))).toBe(svg);
	});
});

describe('VSDX package', () => {
	const unzip = () => unzipSync(zipVsdx(buildVsdxParts(sampleModel(), new Date(0))));
	const partNames = (files: Record<string, Uint8Array>) => new Set(Object.keys(files));

	it('contains the parts Visio requires, and every declared part exists', () => {
		const files = unzip();
		const names = partNames(files);
		for (const required of [
			'[Content_Types].xml',
			'_rels/.rels',
			'visio/document.xml',
			'visio/_rels/document.xml.rels',
			'visio/pages/pages.xml',
			'visio/pages/_rels/pages.xml.rels',
			'visio/pages/page1.xml'
		]) {
			expect(names.has(required)).toBe(true);
		}

		const contentTypes = strFromU8(files['[Content_Types].xml']);
		for (const [, part] of contentTypes.matchAll(/PartName="\/([^"]+)"/g)) {
			expect(names.has(part)).toBe(true);
		}
		for (const name of names) {
			const extension = name.split('.').pop();
			const declared =
				contentTypes.includes(`PartName="/${name}"`) ||
				contentTypes.includes(`Extension="${extension}"`);
			expect(declared).toBe(true);
		}
	});

	it('resolves every relationship target to a part in the package', () => {
		const files = unzip();
		const names = partNames(files);
		for (const relsPath of [...names].filter((n) => n.endsWith('.rels'))) {
			// `a/_rels/b.xml.rels` describes `a/b.xml`; targets are relative to `a/`.
			const base = relsPath.replace(/_rels\/[^/]+$/, '');
			for (const [, target] of strFromU8(files[relsPath]).matchAll(/Target="([^"]+)"/g)) {
				const segments = `${base}${target}`.split('/');
				const resolved: string[] = [];
				for (const s of segments) {
					if (s === '..') resolved.pop();
					else if (s) resolved.push(s);
				}
				expect(names.has(resolved.join('/'))).toBe(true);
			}
		}
	});

	// Regression: groups drawn in front of their members hid every card inside a subnet in Visio.
	it('draws every group behind its members, so cards inside containers stay visible', () => {
		const page = strFromU8(unzip()['visio/pages/page1.xml']);
		const groups = [...page.matchAll(/<Shape ID="\d+"[^>]*Type="Group"[^>]*>(.*?)<Section/g)];
		expect(groups.length).toBeGreaterThan(0);
		for (const [, cells] of groups) {
			expect(cells).toContain('<Cell N="DisplayMode" V="1"/>');
		}
	});

	it('glues both ends of every connector to a shape on the page', () => {
		const files = unzip();
		const page = strFromU8(files['visio/pages/page1.xml']);
		const shapeIds = new Set([...page.matchAll(/<Shape ID="(\d+)"/g)].map((m) => m[1]));
		const connectorIds = [...page.matchAll(/<Shape ID="(\d+)" NameU="Dynamic connector/g)].map(
			(m) => m[1]
		);
		expect(connectorIds).toHaveLength(sampleModel().connectors.length);
		expect(new Set(shapeIds).size).toBe([...page.matchAll(/<Shape ID="/g)].length);

		const connects = [
			...page.matchAll(/<Connect FromSheet="(\d+)" FromCell="(\w+)"[^>]*ToSheet="(\d+)"/g)
		];
		for (const id of connectorIds) {
			const ends = connects.filter((m) => m[1] === id).map((m) => m[2]);
			expect(ends.sort()).toEqual(['BeginX', 'EndX']);
		}
		for (const [, , , toSheet] of connects) {
			expect(shapeIds.has(toSheet)).toBe(true);
			expect(connectorIds).not.toContain(toSheet);
		}
	});
});

describe('shape data', () => {
	const XML_NAME = /^[A-Za-z_][A-Za-z0-9_]*$/;

	it('gives every field a distinct key that is a valid XML name, whatever the label', () => {
		const labels = ['IP Address', 'IP-Address', 'label', 'id', '1st hop', '', 'Ünïcode'];
		const keys = fieldKeys(labels);
		expect(new Set(keys).size).toBe(labels.length);
		for (const key of keys) expect(key).toMatch(XML_NAME);
		expect(keys).not.toContain('label');
		expect(keys).not.toContain('id');
	});

	it('carries each shape’s fields into its draw.io cell, and keeps cell ids unique in the file', () => {
		const model = sampleModel();
		const cells = toDrawioCells(model);
		for (const shape of model.shapes) {
			const cell = cells.find((c) => c.id === `n-${shape.id}`)!;
			expect(cell.data?.map((d) => d.value) ?? []).toEqual(shape.data.map((f) => f.value));
		}
		const xml = serializeDrawio(model);
		for (const cell of cells) {
			const occurrences = xml.split(` id="${cell.id}"`).length - 1;
			expect(occurrences).toBe(1);
		}
	});

	it('writes one Visio Shape Data row per field, with unique row names', () => {
		const model = sampleModel();
		const page = strFromU8(unzipSync(zipVsdx(buildVsdxParts(model)))['visio/pages/page1.xml']);
		const sections = [...page.matchAll(/<Section N="Property">(.*?)<\/Section>/g)].map((m) => m[1]);
		const shapesWithData = model.shapes.filter((s) => s.data.length > 0);
		expect(sections).toHaveLength(shapesWithData.length);
		for (const [i, section] of sections.entries()) {
			const names = [...section.matchAll(/<Row N="([^"]+)">/g)].map((m) => m[1]);
			expect(names).toHaveLength(shapesWithData[i].data.length);
			expect(new Set(names).size).toBe(names.length);
			for (const name of names) expect(name).toMatch(XML_NAME);
		}
	});
});
