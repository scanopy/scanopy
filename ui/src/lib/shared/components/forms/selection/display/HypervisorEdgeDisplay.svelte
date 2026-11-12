<script lang="ts" module>
	import { edgeTypes, serviceDefinitions } from '#lib/shared/stores/metadata.ts';
	import type { RenderableTopology, TopologyEdge } from '#lib/features/topology/types/base.ts';
	import { hostDisplayName } from '#lib/features/hosts/host-display-name.ts';

	export const HypervisorEdgeDisplay: EntityDisplayComponent<TopologyEdge, EdgeDisplayContext> = {
		getId: (edge) => edge.id,
		getLabel: (edge, context) => {
			if (!context?.topology || !('hypervisor_service_id' in edge)) return 'Hypervisor';
			const vmService = context.topology.services.find((s) => s.id === edge.hypervisor_service_id);
			return vmService?.name ?? 'Unknown VM';
		},
		getDescription: (edge, context) => {
			if (!context?.topology || !('hypervisor_service_id' in edge)) return '';
			const vmService = context.topology.services.find((s) => s.id === edge.hypervisor_service_id);
			if (!vmService) return '';
			// Find the hypervisor host (the host running the VM service)
			const hypervisorHost = context.topology.hosts.find((h) => h.id === vmService.host_id);
			const parts: string[] = [];
			const defName = serviceDefinitions.getName(vmService.service_definition);
			if (defName && defName !== vmService.name) parts.push(defName);
			if (hypervisorHost) parts.push(`on ${hostDisplayName(hypervisorHost)}`);
			return parts.join(' · ');
		},
		getIcon: (edge, context) => {
			if (!context?.topology || !('hypervisor_service_id' in edge))
				return edgeTypes.getIconComponent('Hypervisor');
			const vmService = context.topology.services.find((s) => s.id === edge.hypervisor_service_id);
			if (vmService) return serviceDefinitions.getIconComponent(vmService.service_definition);
			return edgeTypes.getIconComponent('Hypervisor');
		},
		getIconColor: (edge, context) => {
			if (!context?.topology || !('hypervisor_service_id' in edge))
				return edgeTypes.getColorHelper('Hypervisor').icon;
			const vmService = context.topology.services.find((s) => s.id === edge.hypervisor_service_id);
			if (vmService) return serviceDefinitions.getColorHelper(vmService.service_definition).icon;
			return edgeTypes.getColorHelper('Hypervisor').icon;
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

<ListSelectItem {item} {context} displayComponent={HypervisorEdgeDisplay} />
