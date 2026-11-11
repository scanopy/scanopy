/**
 * A filter removes elements; the boxes that grouped them have to go too.
 *
 * Collapsed subcontainers were exempted from this prune for a while, which mattered because
 * `PortOpStatus` is the only container type marked `collapsed_by_default` — so hiding unlinked
 * ports left a collapsed "Down" box on every host with nothing inside it. The exemption arrived
 * in a commit described as a pure refactor, so nothing flagged it; these pin the behaviour so the
 * next such change fails a test rather than shipping.
 */
import { describe, it, expect } from 'vitest';
import { pruneEmptySubcontainers } from '#lib/features/topology/pipeline/prepare.ts';
import type { RenderableTopology } from '#lib/features/topology/types/base.ts';

type Nodes = RenderableTopology['nodes'];

const container = (id: string, container_type: string) =>
	({ id, node_type: 'Container', container_type }) as unknown as Nodes[number];

const element = (id: string, container_id: string) =>
	({ id, node_type: 'Element', container_id }) as unknown as Nodes[number];

// Mirrors the shape of the real metadata: only some container types are subcontainers, and the
// collapsed-by-default one is a subcontainer.
const containerTypes = {
	getMetadata: (ct: string) =>
		({
			Host: { is_subcontainer: false },
			VLAN: { is_subcontainer: true },
			PortOpStatus: { is_subcontainer: true }
		})[ct] ?? { is_subcontainer: false }
};

const ids = (nodes: Nodes) => nodes.map((n) => n.id).sort();

describe('pruneEmptySubcontainers', () => {
	it('drops a subcontainer whose elements were all filtered out', () => {
		const nodes = [
			container('host', 'Host'),
			container('down', 'PortOpStatus'),
			container('up', 'PortOpStatus'),
			element('port-1', 'up')
		] as Nodes;

		expect(ids(pruneEmptySubcontainers(nodes, containerTypes))).toEqual(['host', 'port-1', 'up']);
	});

	it('keeps a subcontainer that still has children', () => {
		const nodes = [
			container('host', 'Host'),
			container('vlan', 'VLAN'),
			element('port-1', 'vlan')
		] as Nodes;

		expect(ids(pruneEmptySubcontainers(nodes, containerTypes))).toEqual(['host', 'port-1', 'vlan']);
	});

	it('never drops a root container, even with no children of its own', () => {
		// A host whose ports all live in subcontainers has no direct element children. Pruning it
		// would delete the device from the diagram.
		const nodes = [
			container('host', 'Host'),
			container('up', 'PortOpStatus'),
			element('port-1', 'up')
		] as Nodes;

		expect(ids(pruneEmptySubcontainers(nodes, containerTypes))).toContain('host');
	});
});
