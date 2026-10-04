<script lang="ts" context="module">
	import type { Host, Interface, IPAddress, Port, Service } from '$lib/features/hosts/types/base';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';
	import { entities, proxmoxGuestTypes, serviceDefinitions } from '$lib/shared/stores/metadata';
	import { entityRef, type TagProps } from '$lib/shared/components/data/types';

	// Context provides the host's children (interfaces, ports, services)
	export interface HostDisplayContext {
		/** The host's addresses, shown under its name. Filtered by `host_id` like `services`. */
		ipAddresses?: IPAddress[];
		interfaces?: Interface[];
		ports?: Port[];
		services?: Service[];
		showEntityTagPicker?: boolean;
		tagPickerDisabled?: boolean;
		entityTags?: import('$lib/features/tags/types/base').Tag[];
		allowTagCreate?: boolean;
		showEditableEntityDescription?: boolean;
		entityDescription?: string | null;
		entityDescriptionDisabled?: boolean;
		onEntityDescriptionSave?: (value: string | null) => void;
		compact?: boolean;
		/** A non-null `disabledReason` renders the option disabled with that tooltip. */
		disabledReason?: string | null;
	}

	export const HostDisplay: EntityDisplayComponent<Host, HostDisplayContext> = {
		getId: (host) => host.id,
		getDisabled: (_host, context) => !!context?.disabledReason,
		getDisabledReason: (_host, context) => context?.disabledReason ?? null,
		getLabel: (host) => hostDisplayName(host),
		// The hostname and addresses, each only when it isn't already the label: once the ladder
		// has fallen through to the hostname or an address, repeating it underneath says nothing. A
		// picker row with one line is the correct rendering of a host with one identifier.
		getDescription: (host, context) => {
			const label = hostDisplayName(host);
			const addresses = (context?.ipAddresses ?? [])
				.filter((ip) => ip.host_id === host.id)
				.sort((a, b) => (a.position ?? 0) - (b.position ?? 0))
				.map((ip) => ip.ip_address);
			return [host.hostname, ...addresses]
				.filter((value): value is string => !!value && value !== label)
				.join(', ');
		},
		getIcon: (host, context) => {
			const services = context?.services?.filter((s) => s.host_id == host.id) ?? [];
			const firstService = services.length > 0 ? services[0] : null;
			if (firstService) {
				return serviceDefinitions.getIconComponent(firstService.service_definition);
			} else {
				return entities.getIconComponent('Host');
			}
		},
		getIconColor: () => entities.getColorHelper('Host').icon,
		getTags: (host, context) => {
			// A Proxmox guest's type (VM or LXC) is part of what the host is, so it shows even in
			// compact rows; its services only in full ones.
			const virtualization = host.virtualization_metadata;
			const guestType =
				virtualization?.type === 'Proxmox' ? virtualization.details.guest_type : null;
			const guestTags: TagProps[] = guestType
				? [
						{
							label: proxmoxGuestTypes.getName(guestType),
							color: proxmoxGuestTypes.getColorHelper(guestType).color
						}
					]
				: [];
			if (context?.compact) return guestTags;
			const services = context?.services?.filter((s) => s.host_id == host.id) ?? [];
			return [
				...guestTags,
				...services.map((service) => ({
					label: serviceDefinitions.getName(service.service_definition),
					color: entities.getColorHelper('Service').color,
					entityRef: entityRef('Service', service.id, service)
				}))
			];
		},
		getTagPickerProps: (host: Host, context: HostDisplayContext) => {
			if (!context.showEntityTagPicker) return null;
			return {
				selectedTagIds: host.tags,
				entityId: host.id,
				entityType: 'Host' as const,
				availableTags: context.entityTags,
				allowCreate: context.allowTagCreate
			};
		}
	};
</script>

<script lang="ts">
	import type { EntityDisplayComponent } from '../types';
	import ListSelectItem from '../ListSelectItem.svelte';

	export let item: Host;
	export let context: HostDisplayContext = {};
</script>

<ListSelectItem {item} {context} displayComponent={HostDisplay} />
