import { describe, it, expect } from 'vitest';
import type { RenderableTopology, TopologyNode } from '#lib/features/topology/types/base.ts';
import { buildContainerRender } from '#lib/features/topology/container-render-data.ts';

/**
 * A collapsed root container lists each subgroup it holds as a row, drawn from that subgroup's
 * own node: its header, its rule, and what it contains.
 */

const SITE_ID = 'site-1';
const INFRA_RULE_ID = 'infra-rule-id';

function serviceElement(id: string, containerId: string) {
	return {
		id,
		node_type: 'Element',
		element_type: 'Service',
		host_id: 'host-1',
		entity_id: id,
		container_id: containerId
	};
}

function buildTopology(): RenderableTopology {
	return {
		id: 'topo-1',
		site_id: SITE_ID,
		name: 'test',
		options: {
			local: {},
			request: {
				element_rules: [
					{
						id: INFRA_RULE_ID,
						rule: { ByServiceCategory: { is_infra_rule: true, categories: ['RemoteAccess'] } }
					},
					{ id: 'tag-rule-id', rule: { ByTag: { tag_ids: [] } } }
				]
			}
		},
		hosts: [{ id: 'host-1', site_id: SITE_ID, tags: [] }],
		subnets: [],
		ip_addresses: [],
		services: [],
		ports: [],
		bindings: [],
		interfaces: [],
		dependencies: [],
		vlans: [],
		entity_tags: [],
		nodes: [
			{
				id: 'hypervisor',
				node_type: 'Container',
				container_type: 'Host',
				header: 'hv-01',
				subgroupSummaries: [
					{ groupId: 'tag-group', childCount: 2 },
					{ groupId: 'infra-group', childCount: 1 }
				]
			},
			{
				id: 'infra-group',
				node_type: 'Container',
				container_type: 'NestedServiceCategory',
				parent_container_id: 'hypervisor',
				element_rule_id: INFRA_RULE_ID,
				header: 'Infrastructure'
			},
			{
				id: 'tag-group',
				node_type: 'Container',
				container_type: 'NestedTag',
				parent_container_id: 'hypervisor',
				element_rule_id: 'tag-rule-id',
				header: 'Production'
			},
			serviceElement('app-1', 'tag-group'),
			serviceElement('app-2', 'tag-group'),
			serviceElement('ssh', 'infra-group')
		],
		edges: []
	} as unknown as RenderableTopology;
}

function render(topology: RenderableTopology, isCollapsed: boolean) {
	const node = topology.nodes.find((n) => n.id === 'hypervisor') as TopologyNode;
	return buildContainerRender({
		nodeId: node.id,
		node,
		topology,
		activeView: 'Workloads',
		options: topology.options,
		sites: [],
		isCollapsed
	});
}

describe('collapsed container subgroup rows', () => {
	it('draws one row per subgroup from that subgroup node', () => {
		const { subgroups } = render(buildTopology(), true);

		expect(subgroups.map((row) => [row.headerText, row.childCount, row.countOnly])).toEqual([
			['Production', 2, false],
			['Infrastructure', 1, true]
		]);
	});

	it('draws no rows while expanded', () => {
		expect(render(buildTopology(), false).subgroups).toEqual([]);
	});
});
