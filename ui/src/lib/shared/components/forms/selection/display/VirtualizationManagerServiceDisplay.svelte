<script lang="ts" context="module">
	import {
		concepts,
		serviceDefinitions,
		serviceVirtualizations
	} from '#lib/shared/stores/metadata.ts';
	import type { Host } from '#lib/features/hosts/types/base.ts';
	import type { Service } from '#lib/features/services/types/base.ts';
	import { guestCountLabel } from '#lib/features/services/virtualized-count.ts';

	// Context for virtualization manager display - needs access to hosts and services for counts
	export interface VirtualizationManagerContext {
		hosts: Host[];
		services: Service[];
	}

	export const VirtualizationManagerServiceDisplay: EntityDisplayComponent<
		Service,
		VirtualizationManagerContext
	> = {
		getId: (service: Service) => service.id,
		getLabel: (service: Service) => service.name,
		getDescription: (service: Service, context: VirtualizationManagerContext) => {
			// A runtime's containers are services (bridge networks) and hosts (macvlan, ipvlan); a
			// hypervisor's VMs and a host's network identities are hosts.
			const hostCount = (context?.hosts ?? []).filter(
				(h) => h.virtualization_service_id == service.id
			).length;
			const serviceCount = (context?.services ?? []).filter(
				(s) => s.virtualization_service_id == service.id
			).length;
			// Only a runtime runs services; a hypervisor's VMs and a host's identities are hosts.
			const runsServices =
				serviceDefinitions.getMetadata(service.service_definition).manages_virtualization ===
				'containers';
			return guestCountLabel(
				service.service_definition,
				runsServices ? serviceCount + hostCount : hostCount
			);
		},
		getIcon: (service: Service) => serviceDefinitions.getIconComponent(service.service_definition),
		getIconColor: (service: Service) =>
			serviceDefinitions.getColorHelper(service.service_definition).icon,
		getTags: (service: Service) => {
			let tags = [];

			if (service.virtualization_metadata) {
				const tag: TagProps = {
					...serviceVirtualizations.getTag(service.virtualization_metadata.type),
					color: concepts.getColorHelper('Virtualization').color
				};

				tags.push(tag);
			}

			return tags;
		},
		getCategory: () => null
	};
</script>

<script lang="ts">
	import ListSelectItem from '#lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { EntityDisplayComponent } from '../types';
	import type { TagProps } from '#lib/shared/components/data/types.ts';

	export let item: Service;
	export let context: VirtualizationManagerContext = { hosts: [], services: [] };
</script>

<ListSelectItem {item} {context} displayComponent={VirtualizationManagerServiceDisplay} />
