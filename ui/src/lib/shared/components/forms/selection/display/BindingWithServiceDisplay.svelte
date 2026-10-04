<script lang="ts" context="module">
	import { entities, serviceDefinitions } from '$lib/shared/stores/metadata';
	import type { Binding, Service } from '$lib/features/services/types/base';
	import type { Host, IPAddress, Port } from '$lib/features/hosts/types/base';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';
	import { common_service, common_unknownEntity } from '$lib/paraglide/messages';
	import { BindingDisplay } from './BindingDisplay.svelte';

	// Context for binding display - needs access to services, hosts, interfaces, ports
	export interface BindingWithServiceContext {
		services: Service[];
		hosts: Host[];
		ip_addresses: IPAddress[];
		ports: Port[];
		isContainerSubnet: (subnetId: string) => boolean;
		compact?: boolean;
	}

	export const BindingWithServiceDisplay: EntityDisplayComponent<
		Binding,
		BindingWithServiceContext
	> = {
		getId: (binding: Binding) => binding.id,
		getLabel: (binding: Binding, context: BindingWithServiceContext) => {
			const servicesData = context?.services ?? [];
			const service = servicesData.find((s) => s.bindings.some((b) => b.id === binding.id));
			return service?.name || common_unknownEntity({ entity: common_service() });
		},
		getDescription: (binding: Binding, context: BindingWithServiceContext) =>
			BindingDisplay.getLabel(binding, context),
		getIcon: (binding: Binding, context: BindingWithServiceContext) => {
			const servicesData = context?.services ?? [];
			const service = servicesData.find((s) => s.bindings.some((b) => b.id === binding.id));
			if (!service) return entities.getIconComponent('Service');

			return serviceDefinitions.getIconComponent(service.service_definition);
		},
		getIconColor: (binding: Binding, context: BindingWithServiceContext) => {
			const servicesData = context?.services ?? [];
			const service = servicesData.find((s) => s.bindings.some((b) => b.id === binding.id));
			if (!service) return 'text-secondary';

			return serviceDefinitions.getColorHelper(service.service_definition).icon;
		},
		getTags: () => [],
		getCategory: (binding: Binding, context: BindingWithServiceContext) => {
			const servicesData = context?.services ?? [];
			const hostsData = context?.hosts ?? [];
			const service = servicesData.find((s) => s.bindings.some((b) => b.id === binding.id));
			if (!service) return null;
			const host = hostsData.find((h) => h.id === service.host_id);
			if (!host) return null;

			return hostDisplayName(host);
		}
	};
</script>

<script lang="ts">
	import type { EntityDisplayComponent } from '../types';
	import ListSelectItem from '../ListSelectItem.svelte';

	export let item: Binding;
	export let context: BindingWithServiceContext = {
		services: [],
		hosts: [],
		ip_addresses: [],
		ports: [],
		isContainerSubnet: () => false
	};
</script>

<ListSelectItem {context} {item} displayComponent={BindingWithServiceDisplay} />
