import { describe, it, expect } from 'vitest';
import type { components } from '$lib/api/schema';
import type { RenderableTopology } from '$lib/features/topology/types/base';
import { resolveEditDependencyTargets } from '$lib/features/topology/resolvers';

/**
 * Editing a dependency listed and saved only its stored members, so a node added
 * to the selection drew a preview edge but never reached the sequencing list or
 * the saved members.
 */

type Dependency = components['schemas']['Dependency'];

function service(id: string, ipId: string) {
	return {
		id,
		name: id,
		host_id: 'host-1',
		network_id: 'net-1',
		tags: [],
		bindings: [{ id: `bind-${id}`, ip_address_id: ipId }]
	};
}

function buildTopology(): RenderableTopology {
	return {
		id: 'topo-1',
		network_id: 'net-1',
		name: 'test',
		hosts: [{ id: 'host-1', name: 'host-1', network_id: 'net-1', tags: [] }],
		services: [service('svc-a', 'ip-a'), service('svc-b', 'ip-b'), service('svc-c', 'ip-c')],
		nodes: [
			{ id: 'svc-a', node_type: 'Element', element_type: 'Service' },
			{ id: 'svc-b', node_type: 'Element', element_type: 'Service' },
			{ id: 'svc-c', node_type: 'Element', element_type: 'Service' },
			{ id: 'ip-a', node_type: 'Element', element_type: 'IPAddress', host_id: 'host-1' }
		],
		edges: []
	} as unknown as RenderableTopology;
}

const dependency = {
	id: 'dep-1',
	members: { type: 'Services', service_ids: ['svc-a', 'svc-b'] }
} as unknown as Dependency;

function selected(...ids: string[]) {
	const topology = buildTopology();
	return ids.map((id) => ({ id, data: topology.nodes.find((n) => n.id === id) }));
}

describe('resolveEditDependencyTargets', () => {
	it('appends a node added while editing after the saved members', () => {
		const targets = resolveEditDependencyTargets(
			dependency,
			selected('svc-a', 'svc-b', 'svc-c'),
			buildTopology(),
			new Set()
		);

		expect(targets.map((t) => t.elementId)).toEqual(['svc-a', 'svc-b', 'svc-c']);
	});

	it("doesn't list a saved member again when it's selected as its IP card (L3)", () => {
		const targets = resolveEditDependencyTargets(
			dependency,
			selected('ip-a', 'svc-b'),
			buildTopology(),
			new Set()
		);

		expect(targets.map((t) => t.elementId)).toEqual(['svc-a', 'svc-b']);
	});

	it("drops a removed member, and doesn't bring it back from its still-selected IP card", () => {
		const targets = resolveEditDependencyTargets(
			dependency,
			selected('ip-a', 'svc-b', 'svc-c'),
			buildTopology(),
			new Set(['svc-a'])
		);

		expect(targets.map((t) => t.elementId)).toEqual(['svc-b', 'svc-c']);
	});
});
