<script lang="ts" module>
	import { isContainerSubnet, getSubnetById } from '#lib/features/subnets/queries.ts';
	import type { Subnet } from '#lib/features/subnets/types/base.ts';
	import { entityRef, type TagProps } from '#lib/shared/components/data/types.ts';
	import type { Site } from '#lib/features/sites/types.ts';
	import { getFreshnessTag } from '#lib/shared/utils/freshness.ts';
	import { ipAddressKey } from '#lib/features/hosts/address-labels.ts';
	import { hosts_noMacAddress } from '#lib/paraglide/messages.js';
	import type { Service } from '#lib/features/services/types/base.ts';
	import { concepts, guestStates, serviceVirtualizations } from '#lib/shared/stores/metadata.ts';

	export type IPAddressTagRole = 'subnet' | 'stale' | 'virtualization';

	// Context for interface display - needs access to subnets for lookups
	export interface IPAddressDisplayContext extends DisplayTagContext<IPAddressTagRole> {
		subnets: Subnet[];
		/** Sites to judge each address's staleness against. Without them, no Stale tag. */
		sites?: Site[];
		/** A non-null `disabledReason` renders the option disabled with that tooltip. */
		disabledReason?: string | null;
		/**
		 * The host's services. With them, a bridge container's address is labelled with its
		 * container, which is otherwise indistinguishable from the next 172.17.x address.
		 */
		services?: Service[];
	}

	/** The container service bound to this address, if any. */
	function containerAt(ipAddressId: string, services: Service[] | undefined): Service | undefined {
		return services?.find(
			(s) =>
				s.virtualization_metadata != null && s.bindings.some((b) => b.ip_address_id === ipAddressId)
		);
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
			const container =
				containerAt(iface.id, context?.services)?.virtualization_metadata?.details.container_name ??
				null;
			return [container, name, iface.mac_address ?? hosts_noMacAddress()]
				.filter(Boolean)
				.join(' · ');
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
				context?.sites?.find((n) => n.id === iface.site_id),
				{ entityTypeLabel: entities.getName('IPAddress') || undefined }
			);
			if (stale) tags.push({ ...stale, role: 'stale' satisfies IPAddressTagRole });
			// A container's address, tagged as the service row tags the container. A stopped or
			// paused one says so, since a credential is not tried there until it runs.
			const virtualization = containerAt(iface.id, context?.services)?.virtualization_metadata;
			if (virtualization) {
				tags.push({
					...serviceVirtualizations.getTag(virtualization.type),
					color: concepts.getColorHelper('Virtualization').color,
					role: 'virtualization' satisfies IPAddressTagRole
				});
				const state = virtualization.details.state;
				if (state && state !== 'Running') {
					tags.push({
						...guestStates.getTag(state),
						role: 'virtualization' satisfies IPAddressTagRole
					});
				}
			}
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
	import ListSelectItem from '#lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { AllIPAddresses, IPAddress } from '#lib/features/hosts/types/base.ts';
	import type { DisplayTagContext, EntityDisplayComponent } from '../types';
	import { entities } from '#lib/shared/stores/metadata.ts';

	interface Props {
		item: IPAddress;
		context?: IPAddressDisplayContext;
	}

	let { item, context = { subnets: [] } }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={IPAddressDisplay} />
