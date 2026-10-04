<script lang="ts" context="module">
	import { ALL_IP_ADDRESSES, type IPAddress, type Port } from '$lib/features/hosts/types/base';
	import type { EntityDisplayComponent } from '../types';
	import { entities, ports } from '$lib/shared/stores/metadata';
	import type { Service } from '$lib/features/services/types/base';
	import { formatIPAddress } from '$lib/features/hosts/address-labels';
	import { formatPort } from '$lib/shared/utils/formatting';
	import {
		common_unassigned,
		hosts_bindings_unknownIPAddress,
		hosts_ports_serviceOnAddresses
	} from '$lib/paraglide/messages';

	// Context for port display - needs access to interfaces for binding display
	export interface PortDisplayContext {
		currentServices: Service[];
		ip_addresses: IPAddress[];
		isContainerSubnet: (subnetId: string) => boolean;
		/** A non-null `disabledReason` renders the option disabled with that tooltip. */
		disabledReason?: string | null;
	}

	export const PortDisplay: EntityDisplayComponent<Port, PortDisplayContext> = {
		getId: (port: Port) => `${port.id}`,
		getDisabled: (_port, context) => !!context?.disabledReason,
		getDisabledReason: (_port, context) => context?.disabledReason ?? null,
		// The number first, as in PortTypeDisplay: it's what tells two ports apart, so a narrow row
		// cuts the name instead.
		getLabel: (port: Port) => {
			const metadata = ports.getMetadata(port.type ?? null);
			const name = ports.getName(port.type ?? null);
			const number = formatPort(port);
			return metadata && !metadata.is_custom && name ? `${number} ${name}` : number;
		},
		getDescription: (port: Port, context: PortDisplayContext) => {
			const currentServices = context?.currentServices ?? [];
			const ipAddressesData = context?.ip_addresses ?? [];
			const isContainerSubnetFn = context?.isContainerSubnet ?? (() => false);

			const services: Service[] = currentServices.filter((s) =>
				s.bindings.some((b) => b.type === 'Port' && b.port_id === port.id)
			);
			if (services.length === 0) return common_unassigned();

			return services
				.map((s) =>
					hosts_ports_serviceOnAddresses({
						service: s.name,
						addresses: s.bindings
							.filter((b) => b.type == 'Port' && b.port_id == port.id)
							.map((b) => {
								const iface = b.ip_address_id
									? ipAddressesData.find((i) => i.id === b.ip_address_id)
									: ALL_IP_ADDRESSES;
								return iface
									? formatIPAddress(iface, isContainerSubnetFn)
									: hosts_bindings_unknownIPAddress();
							})
							.join(', ')
					})
				)
				.join(' • ');
		},
		getIcon: () => entities.getIconComponent('Port'),
		getIconColor: () => entities.getColorHelper('Port').icon,
		getTags: () => [],
		getCategory: () => null
	};
</script>

<script lang="ts">
	import ListSelectItem from '../ListSelectItem.svelte';

	export let item: Port;
	export let context: PortDisplayContext = {
		currentServices: [],
		ip_addresses: [],
		isContainerSubnet: () => false
	};
</script>

<ListSelectItem {item} {context} displayComponent={PortDisplay} />
