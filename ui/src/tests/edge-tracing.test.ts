import { describe, it, expect } from 'vitest';
import type { Edge } from '@xyflow/svelte';
import { linksOfHost } from '#lib/features/topology/edge-tracing.ts';
import { outermostNode } from '#lib/features/topology/viewport-fit.ts';
import type { RenderableTopology } from '#lib/features/topology/types/base.ts';

type Topo = Pick<RenderableTopology, 'interfaces' | 'hosts'>;

/** Hosts sw1, sw2, sw3 and plc, each with ports p1 and p2 (`<host>/p1`). */
const topology = {
	hosts: ['sw1', 'sw2', 'sw3', 'plc'].map((id) => ({ id, display_name: id.toUpperCase() })),
	interfaces: ['sw1', 'sw2', 'sw3', 'plc'].flatMap((host) =>
		['p1', 'p2'].map((p) => ({ id: `${host}/${p}`, host_id: host, display_name: p }))
	)
} as unknown as Topo;

function physical(id: string, source: string, target: string): Edge {
	return {
		id,
		source: `node:${source}`,
		target: `node:${target}`,
		data: {
			edge_type: 'PhysicalLink',
			protocol: 'Lldp',
			source_entity_id: source,
			target_entity_id: target
		}
	} as Edge;
}

const edges: Edge[] = [
	physical('sw1-sw2', 'sw1/p2', 'sw2/p1'),
	physical('sw2-sw3', 'sw2/p2', 'sw3/p1'),
	physical('plc-sw2', 'plc/p1', 'sw2/p1'),
	{
		id: 'sw3-plc',
		source: 'node:sw3',
		target: 'node:plc',
		data: {
			edge_type: 'NeighborLink',
			protocol: 'Lldp',
			source_host_id: 'sw3',
			target_host_id: 'plc'
		}
	} as Edge,
	{
		id: 'bundle',
		source: 'a',
		target: 'b',
		data: { ...physical('x', 'sw2/p1', 'sw1/p1').data, isAggregated: true }
	} as Edge,
	{ id: 'other', source: 'a', target: 'b', data: { edge_type: 'SameHost', host_id: 'sw2' } } as Edge
];

describe('linksOfHost', () => {
	it('lists the other links on a host with the far host and both ports, whichever end it is', () => {
		const links = linksOfHost('sw2', edges, topology, 'sw1-sw2');
		expect(
			links.map((l) => [l.edge.id, l.localInterfaceId, l.farHostId, l.farInterfaceId])
		).toEqual([
			['plc-sw2', 'sw2/p1', 'plc', 'plc/p1'],
			['sw2-sw3', 'sw2/p2', 'sw3', 'sw3/p1']
		]);
	});

	it('includes device-level links, with no ports', () => {
		const links = linksOfHost('sw3', edges, topology, 'sw2-sw3');
		expect(links).toHaveLength(1);
		expect(links[0]).toMatchObject({ farHostId: 'plc', localInterfaceId: undefined });
	});

	it('walks a path: the far host of each next link leads on to the following one', () => {
		const first = linksOfHost('sw2', edges, topology, 'sw1-sw2').find(
			(l) => l.farHostId === 'sw3'
		)!;
		const next = linksOfHost(first.farHostId, edges, topology, first.edge.id);
		expect(next.map((l) => l.farHostId)).toEqual(['plc']);
	});
});

describe('outermostNode', () => {
	it('climbs parents to the outermost container', () => {
		const parents: Record<string, string | undefined> = { port: 'group', group: 'host' };
		const lookup = (id: string) =>
			id in parents || id === 'host' ? { parentId: parents[id] } : undefined;
		expect(outermostNode('port', lookup)).toBe('host');
		expect(outermostNode('host', lookup)).toBe('host');
		expect(outermostNode('unknown', lookup)).toBe('unknown');
	});
});
