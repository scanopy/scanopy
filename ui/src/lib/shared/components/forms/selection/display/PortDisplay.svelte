<script lang="ts" context="module">
	import { ALL_IP_ADDRESSES, type IPAddress, type Port } from '$lib/features/hosts/types/base';
	import type { EntityDisplayComponent } from '../types';
	import { entities, ports } from '$lib/shared/stores/metadata';
	import type { Service } from '$lib/features/services/types/base';

	// Context for port display - needs access to interfaces for binding display
	export interface PortDisplayContext {
		currentServices: Service[];
		ip_addresses: IPAddress[];
		isContainerSubnet: (subnetId: string) => boolean;
		/** A non-null `disabledReason` renders the option disabled with that tooltip. */
		disabledReason?: string | null;
	}

	// Helper to format IP address for display
	function formatInterfaceForPort(
		iface: IPAddress | typeof ALL_IP_ADDRESSES,
		isContainerSubnet: (subnetId: string) => boolean
	): string {
		if (iface.id == null) return iface.name;
		return isContainerSubnet(iface.subnet_id)
			? (iface.name ?? iface.ip_address)
			: (iface.name ? iface.name + ': ' : '') + iface.ip_address;
	}

	export const PortDisplay: EntityDisplayComponent<Port, PortDisplayContext> = {
		getId: (port: Port) => `${port.id}`,
		getDisabled: (_port, context) => !!context?.disabledReason,
		getDisabledReason: (_port, context) => context?.disabledReason ?? null,
		getLabel: (port: Port) => {
			let metadata = ports.getMetadata(port.type ?? null);
			let name = ports.getName(port.type ?? null);
			if (metadata && !metadata.is_custom && name) {
				return name + ` (${port.number}/${port.protocol.toLowerCase()})`;
			}
			return `${port.number}/${port.protocol.toLowerCase()}`;
		},
		getDescription: (port: Port, context: PortDisplayContext) => {
			const currentServices = context?.currentServices ?? [];
			const ipAddressesData = context?.ip_addresses ?? [];
			const isContainerSubnetFn = context?.isContainerSubnet ?? (() => false);

			// Use context services if available
			let services: Service[] = currentServices.filter((s) =>
				s.bindings.some((b) => b.type === 'Port' && b.port_id === port.id)
			);

			if (services.length > 0) {
				return services
					.flatMap(
						(s) =>
							s.name +
							' on ' +
							s.bindings
								.filter((b) => b.type == 'Port' && b.port_id == port.id)
								.map((b) => {
									let iface = b.ip_address_id
										? ipAddressesData.find((i) => i.id === b.ip_address_id)
										: ALL_IP_ADDRESSES;
									if (iface) {
										return formatInterfaceForPort(iface, isContainerSubnetFn);
									} else {
										return 'Unknown Interface';
									}
								})
								.join(', ')
					)
					.join(' • ');
			} else {
				return 'Unassigned';
			}
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
