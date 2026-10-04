<script lang="ts" module>
	import { entities } from '$lib/shared/stores/metadata';
	import type { Binding } from '$lib/features/services/types/base';
	import type { IPAddress, Port } from '$lib/features/hosts/types/base';
	import { ALL_IP_ADDRESSES } from '$lib/features/hosts/types/base';
	import { formatIPAddress, formatPortBinding } from '$lib/features/hosts/address-labels';
	import { hosts_bindings_unknownIPAddress } from '$lib/paraglide/messages';

	// Binding-only display — does NOT include service info, since the service is shown
	// separately in the caller (e.g. in the dep-builder card). For a display that
	// combines service + binding, use BindingWithServiceDisplay instead.
	export interface BindingDisplayContext {
		ip_addresses: IPAddress[];
		ports: Port[];
		isContainerSubnet: (subnetId: string) => boolean;
		compact?: boolean;
	}

	export const BindingDisplay: EntityDisplayComponent<Binding, BindingDisplayContext> = {
		getId: (binding: Binding) => binding.id,
		getLabel: (binding: Binding, context: BindingDisplayContext) => {
			if (binding.type === 'IPAddress') {
				const iface = context.ip_addresses.find((i) => i.id === binding.ip_address_id);
				return iface
					? formatIPAddress(iface, context.isContainerSubnet)
					: hosts_bindings_unknownIPAddress();
			}
			const port = context.ports.find((p) => p.id === binding.port_id);
			const iface = binding.ip_address_id
				? context.ip_addresses.find((i) => i.id === binding.ip_address_id)
				: ALL_IP_ADDRESSES;
			return formatPortBinding(port, iface, context.isContainerSubnet);
		},
		getDescription: () => '',
		getIcon: () => entities.getIconComponent('Port'),
		getIconColor: () => entities.getColorHelper('Port').color,
		getTags: () => [],
		getCategory: () => null
	};
</script>

<script lang="ts">
	import type { EntityDisplayComponent } from '../types';
	import ListSelectItem from '../ListSelectItem.svelte';

	interface Props {
		item: Binding;
		context?: BindingDisplayContext;
	}

	let { item, context = { ip_addresses: [], ports: [], isContainerSubnet: () => false } }: Props =
		$props();
</script>

<ListSelectItem {item} {context} displayComponent={BindingDisplay} />
