<script lang="ts" module>
	import { edgeTypes } from '$lib/shared/stores/metadata';
	import type { RenderableTopology, TopologyEdge } from '$lib/features/topology/types/base';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';
	import { interfaceDisplayName } from '$lib/features/hosts/interface-display-name';
	import { common_host, common_interface, common_unknownEntity } from '$lib/paraglide/messages';

	// Serves both LLDP/CDP link types. `PhysicalLink` names the two ports, and the ports are what
	// tell apart several links between the same two switches, so they're the label and the hosts
	// (reached through the ports) the description. `NeighborLink` carries the host ids directly
	// because the ports are exactly what could not be resolved, so it reads as "host ↔ host".
	function endpointsFor(edge: TopologyEdge, topology: RenderableTopology) {
		// The fallbacks are an entity the topology bundle didn't carry, not one without a name —
		// `hostDisplayName` and `interfaceDisplayName` have already handled that.
		const unknownHost = common_unknownEntity({ entity: common_host() });
		const hostName = (hostId: string | undefined) => {
			const host = topology.hosts.find((h) => h.id === hostId);
			return host ? hostDisplayName(host) : unknownHost;
		};
		if ('source_host_id' in edge && 'target_host_id' in edge) {
			return {
				hosts: [hostName(edge.source_host_id), hostName(edge.target_host_id)],
				ports: null
			};
		}
		if ('source_entity_id' in edge && 'target_entity_id' in edge) {
			const source = topology.interfaces.find((i) => i.id === edge.source_entity_id);
			const target = topology.interfaces.find((i) => i.id === edge.target_entity_id);
			const unknownPort = common_unknownEntity({ entity: common_interface() });
			return {
				hosts: [hostName(source?.host_id), hostName(target?.host_id)],
				ports: [
					source ? interfaceDisplayName(source) : unknownPort,
					target ? interfaceDisplayName(target) : unknownPort
				]
			};
		}
		return { hosts: [unknownHost, unknownHost], ports: null };
	}

	const protocolOf = (edge: TopologyEdge) =>
		'protocol' in edge ? ((edge.protocol as string | null) ?? '') : '';

	export const PhysicalLinkEdgeDisplay: EntityDisplayComponent<TopologyEdge, EdgeDisplayContext> = {
		getId: (edge) => edge.id,
		getLabel: (edge, context) => {
			if (!context?.topology) return edgeTypes.getName(edge.edge_type);
			const { hosts, ports } = endpointsFor(edge, context.topology);
			return (ports ?? hosts).join(' ↔ ');
		},
		getDescription: (edge, context) => {
			const protocol = protocolOf(edge);
			if (!context?.topology) return protocol;
			const { hosts, ports } = endpointsFor(edge, context.topology);
			return [ports ? hosts.join(' ↔ ') : '', protocol].filter(Boolean).join(' · ');
		},
		getIcon: (edge) => edgeTypes.getIconComponent(edge.edge_type),
		getIconColor: (edge) => edgeTypes.getColorHelper(edge.edge_type).icon
	};

	export interface EdgeDisplayContext {
		topology?: RenderableTopology;
	}
</script>

<script lang="ts">
	import type { EntityDisplayComponent } from '../types';
	import ListSelectItem from '../ListSelectItem.svelte';

	interface Props {
		item: TopologyEdge;
		context: EdgeDisplayContext;
	}

	let { item, context }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={PhysicalLinkEdgeDisplay} />
