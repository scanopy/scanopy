<script lang="ts" context="module">
	import { entities, serviceDefinitions } from '$lib/shared/stores/metadata';
	import type { IPAddressBinding, Service } from '$lib/features/services/types/base';
	import type { HostFormData, IPAddress } from '$lib/features/hosts/types/base';
	import type { EntityDisplayComponent } from '../types';
	import { formatIPAddress } from '$lib/features/hosts/address-labels';
	import { hosts_bindings_unknownIPAddress } from '$lib/paraglide/messages';
	import IPAddressBindingInlineEditor from './IPAddressBindingInlineEditor.svelte';

	// Context for binding display within form editing
	export interface IPAddressBindingDisplayContext {
		service: Service;
		host: HostFormData;
		services: Service[];
		ip_addresses: IPAddress[];
		isContainerSubnet: (subnetId: string) => boolean;
	}

	export const IPAddressBindingDisplay: EntityDisplayComponent<
		IPAddressBinding,
		IPAddressBindingDisplayContext
	> = {
		getId: (binding: IPAddressBinding) => binding.id,
		getLabel: (binding: IPAddressBinding, context: IPAddressBindingDisplayContext) => {
			const interfacesData = context?.ip_addresses ?? [];
			const isContainerSubnetFn = context?.isContainerSubnet ?? (() => false);
			const ipAddr = interfacesData.find((i) => i.id === binding.ip_address_id);
			return ipAddr
				? formatIPAddress(ipAddr, isContainerSubnetFn)
				: hosts_bindings_unknownIPAddress();
		},
		getDescription: () => '',
		getIcon: () => entities.getIconComponent('IPAddress'),
		getIconColor: () => entities.getColorHelper('IPAddress').icon,
		getTags: () => [],
		getCategory: (binding: IPAddressBinding, context: IPAddressBindingDisplayContext) => {
			const servicesData = context?.services ?? [];
			const service = servicesData.find((s) => s.bindings.some((b) => b.id === binding.id));
			if (!service) return null;

			const serviceType = serviceDefinitions.getItem(service.service_definition);
			return serviceType?.category || null;
		},
		supportsInlineEdit: true,
		InlineEditorComponent: IPAddressBindingInlineEditor
	};
</script>

<script lang="ts">
	import ListSelectItem from '../ListSelectItem.svelte';

	export let item: IPAddressBinding;
	export let context: IPAddressBindingDisplayContext;
</script>

<ListSelectItem {item} {context} displayComponent={IPAddressBindingDisplay} />
