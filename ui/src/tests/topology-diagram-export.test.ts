import { describe, it, expect } from 'vitest';
import { unzipSync, strFromU8 } from 'fflate';
import type { Edge } from '@xyflow/svelte';
import {
	buildDiagramModel,
	type DiagramDescriber,
	type DiagramModel,
	type DiagramSourceEdge,
	type DiagramSourceNode
} from '#lib/features/topology/diagram-export/model.ts';
import { serializeDrawio, toDrawioCells } from '#lib/features/topology/diagram-export/drawio.ts';
import { fieldKeys } from '#lib/features/topology/diagram-export/xml.ts';
import { buildVsdxParts, zipVsdx } from '#lib/features/topology/diagram-export/vsdx.ts';
import { createDescriber } from '#lib/features/topology/diagram-export/describe.ts';
import type { RenderableTopology, TopologyNode } from '#lib/features/topology/types/base.ts';
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
		title: n.id,
		lines: n.type === 'Element' ? ['10.0.0.1'] : [],
		style: { fill: '#ffffff', stroke: '#94a3b8', text: '#1f2937' },
		iconKey: n.id === 'web' ? 'icon-web' : undefined,
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
		options: {},
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
			iconKeys: new Map()
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
		const cells = toDrawioCells(sampleModel());
		const parents = new Set(cells.filter((c) => c.vertex).map((c) => c.parent));
		for (const cell of cells.filter((c) => c.vertex && parents.has(c.id))) {
			expect(cell.style).toContain('container=1');
		}
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
