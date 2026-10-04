<script lang="ts" module>
	import { isContainerSubnet, getSubnetById } from '$lib/features/subnets/queries';
	import type { Subnet } from '$lib/features/subnets/types/base';
	import { entityRef, type TagProps } from '$lib/shared/components/data/types';
	import type { Network } from '$lib/features/networks/types';
	import { getFreshnessTag } from '$lib/shared/utils/freshness';
	import { ipAddressKey } from '$lib/features/hosts/address-labels';
	import { hosts_noMacAddress } from '$lib/paraglide/messages';

	export type IPAddressTagRole = 'subnet' | 'stale';

	// Context for interface display - needs access to subnets for lookups
	export interface IPAddressDisplayContext extends DisplayTagContext<IPAddressTagRole> {
		subnets: Subnet[];
		/** Networks to judge each address's staleness against. Without them, no Stale tag. */
		networks?: Network[];
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
		// The address alone, so a long IPv6 address gets the whole row; its interface name goes on
		// the description line.
		getLabel: (iface, context?: IPAddressDisplayContext) =>
			ipAddressKey(iface, (subnetId) => {
				const subnet = getSubnetById(context?.subnets ?? [], subnetId);
				return !!subnet && isContainerSubnet(subnet);
			}),
		getDescription: (iface, context?: IPAddressDisplayContext) => {
			if (iface.id == null) return '';
			const label = IPAddressDisplay.getLabel(iface, context);
			const name = iface.name && iface.name !== label ? iface.name : null;
			return [name, iface.mac_address ?? hosts_noMacAddress()].filter(Boolean).join(' · ');
		},
		getIcon: () => entities.getIconComponent('IPAddress'),
		getIconColor: () => entities.getColorHelper('IPAddress').icon,
		// Topology already shows the subnet as the address's container.
		compactHides: ['subnet'] satisfies IPAddressTagRole[],
		getTags: (iface, context: IPAddressDisplayContext) => {
			if (iface.id == null) return [];
			const tags: TagProps[] = [];
			// Each address carries its own verdict, so one the host stopped answering on reads
			// Stale while the host stays current. It comes first: when a narrow row fits one tag,
			// the address's status outranks its subnet.
			const stale = getFreshnessTag(
				iface,
				context?.networks?.find((n) => n.id === iface.network_id),
				{ entityTypeLabel: entities.getName('IPAddress') || undefined }
			);
			if (stale) tags.push({ ...stale, role: 'stale' satisfies IPAddressTagRole });
			const subnet = getSubnetById(context?.subnets ?? [], iface.subnet_id);
			if (subnet && !isContainerSubnet(subnet)) {
				tags.push({
					label: subnet.cidr,
					color: entities.getColorHelper('Subnet').color,
					entityRef: entityRef('Subnet', subnet.id, subnet),
					role: 'subnet' satisfies IPAddressTagRole
				});
			}
			return tags;
		},
		getCategory: () => null
	};
</script>

<script lang="ts">
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { AllIPAddresses, IPAddress } from '$lib/features/hosts/types/base';
	import type { DisplayTagContext, EntityDisplayComponent } from '../types';
	import { entities } from '$lib/shared/stores/metadata';

	interface Props {
		item: IPAddress;
		context?: IPAddressDisplayContext;
	}

	let { item, context = { subnets: [] } }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={IPAddressDisplay} />
