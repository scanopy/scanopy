<script lang="ts" context="module">
	import { entities, serviceDefinitions } from '$lib/shared/stores/metadata';
	import { formatPortBinding } from '$lib/features/hosts/address-labels';
	import type { PortBinding, Service } from '$lib/features/services/types/base';
	import {
		ALL_IP_ADDRESSES,
		type HostFormData,
		type IPAddress,
		type Port
	} from '$lib/features/hosts/types/base';
	import type { EntityDisplayComponent } from '../types';
	import PortBindingInlineEditor from './PortBindingInlineEditor.svelte';

	// Context for binding display within form editing
	export interface PortBindingDisplayContext {
		service: Service;
		host: HostFormData;
		services: Service[];
		ip_addresses: IPAddress[];
		ports: Port[];
		isContainerSubnet: (subnetId: string) => boolean;
	}

	export const PortBindingDisplay: EntityDisplayComponent<PortBinding, PortBindingDisplayContext> =
		{
			getId: (binding: PortBinding) => binding.id,
			getLabel: (binding: PortBinding, context: PortBindingDisplayContext) => {
				const portsData = context?.ports ?? [];
				const ipAddressesData = context?.ip_addresses ?? [];
				const isContainerSubnetFn = context?.isContainerSubnet ?? (() => false);

				const port = portsData.find((p) => p.id === binding.port_id);
				const ipAddr = binding.ip_address_id
					? ipAddressesData.find((i) => i.id === binding.ip_address_id)
					: ALL_IP_ADDRESSES;
				return formatPortBinding(port, ipAddr, isContainerSubnetFn);
			},
			getDescription: () => '',
			getIcon: () => entities.getIconComponent('Port'),
			getIconColor: () => entities.getColorHelper('Port').icon,
			getTags: () => [],
			getCategory: (binding: PortBinding, context: PortBindingDisplayContext) => {
				const servicesData = context?.services ?? [];
				const service = servicesData.find((s) => s.bindings.some((b) => b.id === binding.id));
				if (!service) return null;

				const serviceType = serviceDefinitions.getItem(service.service_definition);
				return serviceType?.category || null;
			},
			supportsInlineEdit: true,
			InlineEditorComponent: PortBindingInlineEditor
		};
</script>

<script lang="ts">
	import ListSelectItem from '../ListSelectItem.svelte';

	export let item: PortBinding;
	export let context: PortBindingDisplayContext;
</script>

<ListSelectItem {item} {context} displayComponent={PortBindingDisplay} />
