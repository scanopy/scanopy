import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
	activeViewFilters,
	hiddenEntityIds,
	matchesHoveredMetadata,
	tagHiddenNodeIds,
	updateTagFilter
} from '$lib/features/topology/interactions';
import { filterFor, hiddenValuesFor, withFilterValues } from '$lib/features/topology/view-filters';
import type { RenderableTopology } from '$lib/features/topology/types/base';
import type { Site } from '$lib/features/sites/types';

/**
 * Workloads' virtualization filter covers hosts and services: a macvlan container is drawn as a
 * Host card, a container on a bridge network as a Service card, and Containerized is both. The
 * filter is read from the views fixture, so these follow whatever entities the backend declares.
 */

const SITE_ID = 'net-1';
const site = { id: SITE_ID } as Site;

function buildTopology(): RenderableTopology {
	const base = { site_id: SITE_ID, tags: [] };
	return {
		id: 'topo-1',
		site_id: SITE_ID,
		hosts: [
			{ ...base, id: 'docker-host' },
			{
				...base,
				id: 'macvlan',
				virtualization_metadata: {
					type: 'Docker',
					details: { network_type: 'MacVlan' }
				}
			}
		],
		services: [
			{
				...base,
				id: 'bridge-container',
				host_id: 'docker-host',
				virtualization_metadata: { type: 'Docker', details: {} }
			},
			{ ...base, id: 'sshd', host_id: 'docker-host' }
		],
		nodes: [
			{
				id: 'bridge-container',
				node_type: 'Element',
				element_type: 'Service',
				host_id: 'docker-host'
			},
			{ id: 'sshd', node_type: 'Element', element_type: 'Service', host_id: 'docker-host' },
			{ id: 'macvlan-card', node_type: 'Element', element_type: 'Host', host_id: 'macvlan' }
		],
		edges: [],
		subnets: [],
		ip_addresses: [],
		ports: [],
		bindings: [],
		interfaces: [],
		dependencies: [],
		vlans: [],
		entity_tags: []
	} as unknown as RenderableTopology;
}

const filter = () => {
	const f = filterFor('Workloads', 'Service', 'Virtualization');
	if (!f) throw new Error('Workloads declares no Service virtualization filter');
	return f;
};

beforeEach(() => {
	tagHiddenNodeIds.set(new Set());
	hiddenEntityIds.set(new Set());
});

describe('a filter covering hosts and services', () => {
	it('is one filter for both entities', () => {
		expect(filterFor('Workloads', 'Host', 'Virtualization')).toBe(filter());
	});

	it('rings a bridge container service and a macvlan host on Containerized', () => {
		const topo = buildTopology();
		const hovered = {
			entityTypes: filter().entities,
			filterType: 'Virtualization',
			valueId: 'Containerized',
			color: 'Blue'
		};
		const [, macvlan] = topo.hosts;
		const [container, sshd] = topo.services;
		expect(matchesHoveredMetadata(container, 'Service', hovered, site, topo)).toBe(true);
		expect(matchesHoveredMetadata(macvlan, 'Host', hovered, site, topo)).toBe(true);
		expect(matchesHoveredMetadata(sshd, 'Service', hovered, site, topo)).toBe(false);
	});

	it('hides both when Containerized is hidden through the chip', () => {
		const hidden = withFilterValues(filter(), undefined, ['Containerized']);
		updateTagFilter(buildTopology(), undefined, 'Workloads', hidden, [], site);

		const nodes = get(tagHiddenNodeIds);
		expect(nodes.has('bridge-container')).toBe(true);
		expect(nodes.has('macvlan-card')).toBe(true);
		expect(nodes.has('sshd')).toBe(false);
	});

	it('reads as hidden when any covered entity hides the value', () => {
		expect(hiddenValuesFor(filter(), { Host: { Virtualization: ['Virtualized'] } })).toEqual([
			'Virtualized'
		]);
	});

	it('is named once as the cause of an emptied view', () => {
		const hidden = withFilterValues(filter(), undefined, ['Containerized']);
		const summaries = activeViewFilters('Workloads', buildTopology(), hidden, [], undefined, site);

		expect(summaries).toHaveLength(1);
		expect(summaries[0]).toMatchObject({ label: filter().label, count: 2 });
	});
});
