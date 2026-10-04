<script lang="ts" module>
	import { isContainerSubnet, getSubnetById } from '$lib/features/subnets/queries';
	import type { Subnet } from '$lib/features/subnets/types/base';
	import { entityRef, type TagProps } from '$lib/shared/components/data/types';
	import type { Network } from '$lib/features/networks/types';
	import { getFreshnessTag } from '$lib/shared/utils/freshness';

	// Context for interface display - needs access to subnets for lookups
	export interface IPAddressDisplayContext {
		subnets: Subnet[];
		/** Networks to judge each address's staleness against. Without them, no Stale tag. */
		networks?: Network[];
		/** Drops the subnet tag, which topology already shows as the address's container. */
		compact?: boolean;
		/** A non-null `disabledReason` renders the option disabled with that tooltip. */
		disabledReason?: string | null;
	}

	/** Option id for the `ALL_IP_ADDRESSES` entry, whose own id is null. */
	export const ALL_IP_ADDRESSES_ID = '__ALL_INTERFACES__';

	export const IPAddressDisplay: EntityDisplayComponent<
		IPAddress | AllIPAddresses,
		IPAddressDisplayContext
	> = {
		getId: (iface) => iface.id ?? ALL_IP_ADDRESSES_ID,
		getDisabled: (_iface, context) => !!context?.disabledReason,
		getDisabledReason: (_iface, context) => context?.disabledReason ?? null,
		getLabel: (iface, context?: IPAddressDisplayContext) => {
			if (iface.id == null) return iface.name;
			// Align with formatIPAddress(): "name: IP" or just "IP" (or name-only for containers)
			const subnetsData = context?.subnets ?? [];
			const subnet = getSubnetById(subnetsData, iface.subnet_id);
			if (subnet && isContainerSubnet(subnet)) {
				return iface.name ?? iface.ip_address;
			}
			return (iface.name ? iface.name + ': ' : '') + iface.ip_address;
		},
		getDescription: (iface) => {
			if (iface.id == null) return '';
			return iface.mac_address ?? 'No MAC';
		},
		getIcon: () => entities.getIconComponent('IPAddress'),
		getIconColor: () => entities.getColorHelper('IPAddress').icon,
		getTags: (iface, context: IPAddressDisplayContext) => {
			if (iface.id == null) return [];
			const tags: TagProps[] = [];
			const subnet = getSubnetById(context?.subnets ?? [], iface.subnet_id);
			if (!context?.compact && subnet && !isContainerSubnet(subnet)) {
				tags.push({
					label: subnet.cidr,
					color: entities.getColorHelper('Subnet').color,
					entityRef: entityRef('Subnet', subnet.id, subnet)
				});
			}
			// Each address carries its own verdict, so one the host stopped answering on reads
			// Stale while the host stays current.
			const stale = getFreshnessTag(
				iface,
				context?.networks?.find((n) => n.id === iface.network_id),
				{ entityTypeLabel: entities.getName('IPAddress') || undefined }
			);
			if (stale) tags.push(stale);
			return tags;
		},
		getCategory: () => null
	};
</script>

<script lang="ts">
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { AllIPAddresses, IPAddress } from '$lib/features/hosts/types/base';
	import type { EntityDisplayComponent } from '../types';
	import { entities } from '$lib/shared/stores/metadata';

	interface Props {
		item: IPAddress;
		context?: IPAddressDisplayContext;
	}

	let { item, context = { subnets: [] } }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={IPAddressDisplay} />
