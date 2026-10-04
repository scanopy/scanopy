import { describe, it, expect } from 'vitest';
import type { RenderableTopology } from '$lib/features/topology/types/base';
import { tallyContainerElements } from '$lib/features/topology/labels';
import { getInfrastructureRuleIdForTopology } from '$lib/features/topology/queries';

/**
 * A container's total leaves out the services the infrastructure rule grouped
 * into its own subcontainer, which is what the infra rule's note promises
 * ("excluded from workload counts"). Before this, a hypervisor holding two VMs
 * and one infrastructure service read "3 workloads".
 */

const NETWORK_ID = 'net-1';
const INFRA_RULE_ID = 'infra-rule-id';

function serviceElement(id: string, containerId: string) {
	return {
		id,
		node_type: 'Element',
		element_type: 'Service',
		host_id: 'host-1',
		container_id: containerId
	};
}

function buildTopology(): RenderableTopology {
	return {
		id: 'topo-1',
		network_id: NETWORK_ID,
		name: 'test',
		options: {
			request: {
				element_rules: [
					{
						id: INFRA_RULE_ID,
						rule: {
							ByServiceCategory: { is_infra_rule: true, categories: ['RemoteAccess', 'OpenPorts'] }
						}
					},
					{ id: 'tag-rule-id', rule: { ByTag: { tag_ids: [] } } }
				]
			}
		},
		hosts: [
			{ id: 'host-1', network_id: NETWORK_ID, tags: [] },
			{ id: 'vm-host', network_id: NETWORK_ID, tags: [] }
		],
		subnets: [],
		ip_addresses: [],
		services: [
			// Inlined on the VM's host element: SSH is infrastructure, Home Assistant isn't.
			{ id: 'vm-ssh', host_id: 'vm-host', service_definition: 'SSH', tags: [], bindings: [] },
			{
				id: 'vm-app',
				host_id: 'vm-host',
				service_definition: 'Home Assistant',
				tags: [],
				bindings: []
			}
		],
		ports: [],
		bindings: [],
		interfaces: [],
		dependencies: [],
		vlans: [],
		entity_tags: [],
		nodes: [
			{ id: 'hypervisor', node_type: 'Container', container_type: 'Host' },
			{
				id: 'infra-group',
				node_type: 'Container',
				container_type: 'NestedServiceCategory',
				parent_container_id: 'hypervisor',
				element_rule_id: INFRA_RULE_ID
			},
			{
				id: 'tag-group',
				node_type: 'Container',
				container_type: 'NestedTag',
				parent_container_id: 'hypervisor',
				element_rule_id: 'tag-rule-id'
			},
			serviceElement('vm-1', 'hypervisor'),
			serviceElement('vm-2', 'tag-group'),
			serviceElement('dns', 'infra-group'),
			{
				id: 'vm-element',
				node_type: 'Element',
				element_type: 'Host',
				host_id: 'vm-host',
				container_id: 'hypervisor'
			}
		],
		edges: []
	} as unknown as RenderableTopology;
}

describe('container workload count', () => {
	it('leaves out infrastructure services in the subcontainer and inlined on a VM', () => {
		const topology = buildTopology();
		const infraRuleId = getInfrastructureRuleIdForTopology(topology);

		const counts = tallyContainerElements('hypervisor', topology, infraRuleId);

		// Services: vm-1 (direct), vm-2 (another rule's subcontainer) and vm-app
		// (inlined on the VM). dns (infra subcontainer) and vm-ssh (inlined SSH,
		// RemoteAccess) don't count.
		expect(counts.get('Service')).toBe(3);
		expect(counts.get('Host')).toBe(1);
	});

	it('counts everything when no rule is excluded', () => {
		const counts = tallyContainerElements('hypervisor', buildTopology());

		expect(counts.get('Service')).toBe(5);
		expect(counts.get('Host')).toBe(1);
	});

	it("still counts the infrastructure subcontainer's own contents", () => {
		const counts = tallyContainerElements('infra-group', buildTopology());

		expect(counts.get('Service')).toBe(1);
	});
});
