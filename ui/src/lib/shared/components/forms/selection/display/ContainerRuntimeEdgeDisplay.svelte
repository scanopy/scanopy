<script lang="ts" module>
	import { edgeTypes, serviceDefinitions } from '$lib/shared/stores/metadata';
	import type { RenderableTopology, TopologyEdge } from '$lib/features/topology/types/base';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';
	import { topology_containerCount } from '$lib/paraglide/messages';
	import { containerHostsOfEdge } from '$lib/features/topology/resolvers';

	export const ContainerRuntimeEdgeDisplay: EntityDisplayComponent<
		TopologyEdge,
		EdgeDisplayContext
	> = {
		getId: (edge) => edge.id,
		getLabel: (edge, context) => {
			const fallback = edgeTypes.getName('ContainerRuntime');
			if (!context?.topology || edge.edge_type !== 'ContainerRuntime') return fallback;
			const topology = context.topology;
			// The containers this edge stands for: the containerized services it names, or, on an
			// edge to a container host (macvlan, ipvlan), which names none, those hosts.
			const containerNames = [
				...edge.containerized_service_ids.flatMap(
					(id) => topology.services.find((s) => s.id === id)?.name ?? []
				),
				...containerHostsOfEdge(topology, edge).map(hostDisplayName)
			];
			if (containerNames.length === 0) return fallback;
			if (containerNames.length === 1) return containerNames[0];
			return topology_containerCount({ count: containerNames.length });
		},
		getDescription: (edge, context) => {
			if (!context?.topology || !('service_id' in edge)) return '';
			const containerizer = context.topology.services.find((s) => s.id === edge.service_id);
			const host =
				'host_id' in edge ? context.topology.hosts.find((h) => h.id === edge.host_id) : null;
			const parts: string[] = [];
			if (containerizer) parts.push(containerizer.name);
			if (host) parts.push(hostDisplayName(host));
			return parts.join(' · ');
		},
		getIcon: () => edgeTypes.getIconComponent('ContainerRuntime'),
		getIconColor: () => edgeTypes.getColorHelper('ContainerRuntime').icon,
		getTags: (edge, context) => {
			if (!context?.topology || !('service_id' in edge)) return [];
			const containerizer = context.topology.services.find((s) => s.id === edge.service_id);
			if (!containerizer) return [];
			const defName = serviceDefinitions.getName(containerizer.service_definition);
			return defName
				? [{ label: defName, color: edgeTypes.getColorHelper('ContainerRuntime').color }]
				: [];
		}
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

<ListSelectItem {item} {context} displayComponent={ContainerRuntimeEdgeDisplay} />
