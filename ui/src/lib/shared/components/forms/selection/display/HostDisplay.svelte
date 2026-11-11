<script lang="ts" context="module">
	import type {
		Host,
		Interface,
		IPAddress,
		Port,
		Service
	} from '#lib/features/hosts/types/base.ts';
	import { hostDisplayName } from '#lib/features/hosts/host-display-name.ts';
	import { hostIcon, hostTile } from '#lib/features/hosts/host-icon.ts';
	import {
		containerNetworkTypes,
		entities,
		guestStates,
		hostVirtualizations,
		proxmoxGuestTypes,
		serviceDefinitions
	} from '#lib/shared/stores/metadata.ts';
	import { entityRef, type TagProps } from '#lib/shared/components/data/types.ts';
	import { cachedEntities, queryKeys } from '#lib/api/query-client.ts';
	import {
		hosts_networkIdentity_tagTitle,
		hosts_networkIdentity_tagTitleUnresolved
	} from '#lib/paraglide/messages.js';

	/** An entity with this id from any cached query under `rootKey`. */
	function findCached<T extends { id: string }>(rootKey: readonly unknown[], id: string): T | null {
		return cachedEntities<T>(rootKey).find((item) => item.id === id) ?? null;
	}

	/**
	 * The name of the host that presents a network identity: the host of the Network Identities
	 * service the identity hangs off. Null when either is not loaded.
	 */
	function identityPresenterName(host: Host, context?: HostDisplayContext): string | null {
		const serviceId = host.virtualization_service_id;
		if (!serviceId) return null;
		const service =
			context?.services?.find((s) => s.id === serviceId) ??
			findCached<Service>(queryKeys.services.all, serviceId);
		if (!service) return null;
		const presenter = findCached<Host>(queryKeys.hosts.all, service.host_id);
		return presenter ? hostDisplayName(presenter) : null;
	}

	/**
	 * What kind of guest the host is, from its virtualization record: a Proxmox guest's type (VM or
	 * LXC), a container host's LAN network driver (macvlan or ipvlan), or a network identity.
	 */
	function virtualizationTags(host: Host, context?: HostDisplayContext): TagProps[] {
		const virtualization = host.virtualization_metadata;
		// A stopped or paused guest says so; a running one, the usual case, adds nothing.
		const stateTags = (state: string | undefined): TagProps[] =>
			state && state !== 'Running'
				? [{ ...guestStates.getTag(state), role: 'guest' satisfies HostTagRole }]
				: [];
		switch (virtualization?.type) {
			case 'Proxmox': {
				const guestType = virtualization.details.guest_type;
				return [
					...(guestType
						? [{ ...proxmoxGuestTypes.getTag(guestType), role: 'guest' satisfies HostTagRole }]
						: []),
					...stateTags(virtualization.details.state)
				];
			}
			case 'Docker':
			case 'Podman': {
				const networkType = virtualization.details.network_type;
				return [
					{
						...containerNetworkTypes.getTag(networkType),
						role: 'guest' satisfies HostTagRole
					},
					...stateTags(virtualization.details.state)
				];
			}
			case 'NetworkIdentity': {
				const guestName = identityPresenterName(host, context);
				return [
					{
						label: hostVirtualizations.getName(virtualization.type),
						color: hostVirtualizations.getColorHelper(virtualization.type).color,
						title: guestName
							? hosts_networkIdentity_tagTitle({ guestName })
							: hosts_networkIdentity_tagTitleUnresolved(),
						role: 'guest' satisfies HostTagRole
					}
				];
			}
			default:
				return [];
		}
	}

	export type HostTagRole = 'guest' | 'service';

	// Context provides the host's children (interfaces, ports, services)
	export interface HostDisplayContext extends DisplayTagContext<HostTagRole> {
		/** The host's addresses, shown under its name. Filtered by `host_id` like `services`. */
		ipAddresses?: IPAddress[];
		interfaces?: Interface[];
		ports?: Port[];
		services?: Service[];
		showEntityTagPicker?: boolean;
		tagPickerDisabled?: boolean;
		entityTags?: import('#lib/features/tags/types/base.ts').Tag[];
		allowTagCreate?: boolean;
		showEditableEntityDescription?: boolean;
		entityDescription?: string | null;
		entityDescriptionDisabled?: boolean;
		onEntityDescriptionSave?: (value: string | null) => void;
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
		// The host's device type, as on every other surface, not its first service's logo.
		getIcon: (host) => hostTile(host),
		getIconColor: () => entities.getColorHelper('Host').icon,
		getChipIcon: (host) => hostIcon(host),
		// What kind of guest the host is (VM, LXC, macvlan container, network identity) is part of
		// what the host is, so it shows even in compact rows; its services only in full ones. A view
		// whose own heading already names the guest type hides `guest` too.
		compactHides: ['service'] satisfies HostTagRole[],
		getTags: (host, context) => {
			const services = context?.services?.filter((s) => s.host_id == host.id) ?? [];
			return [
				...virtualizationTags(host, context),
				...services.map((service) => ({
					label: serviceDefinitions.getName(service.service_definition),
					color: entities.getColorHelper('Service').color,
					entityRef: entityRef('Service', service.id, service),
					role: 'service' satisfies HostTagRole
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
	import type { DisplayTagContext, EntityDisplayComponent } from '../types';
	import ListSelectItem from '../ListSelectItem.svelte';

	export let item: Host;
	export let context: HostDisplayContext = {};
</script>

<ListSelectItem {item} {context} displayComponent={HostDisplay} />
