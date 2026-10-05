<script lang="ts">
	import { lastSeenItems } from '$lib/shared/utils/freshness';
	import type {
		Host,
		CreateHostWithServicesRequest,
		UpdateHostWithServicesRequest
	} from '../types/base';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';
	import {
		hostOsFamilyIds,
		hostOsFamilyName,
		type HostOsFamily
	} from '$lib/features/hosts/host-os';
	import OsTag from './OsTag.svelte';
	import { interfaceDisplayName } from '$lib/features/hosts/interface-display-name';
	import HostEditor from './HostEditModal/HostEditor.svelte';
	import HostConsolidationModal from './HostConsolidationModal.svelte';
	import HostExportModal from './HostExportModal.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import { defineFields, entityRef, type CardAction } from '$lib/shared/components/data/types';
	import { tagNames } from '$lib/features/tags/columns';
	import { siteItems } from '$lib/features/sites/columns';
	import { credentialItems } from '$lib/features/credentials/columns';
	import {
		entities,
		entitySources,
		concepts,
		serviceDefinitions
	} from '$lib/shared/stores/metadata';
	import { Plus, Trash2, RefreshCw, Replace, Eye, Edit } from 'lucide-svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import UpgradeButton from '$lib/shared/components/UpgradeButton.svelte';
	import type { TabProps } from '$lib/shared/types';
	import {
		common_confirmDeleteName,
		common_consolidate,
		common_create,
		common_created,
		common_delete,
		common_description,
		common_edit,
		common_hide,
		common_hidden,
		common_hostname,
		common_credentials,
		common_hosts,
		common_interfaces,
		common_ports,
		common_firstFoundBy,
		common_lastFoundBy,
		common_ipAddresses,
		common_lastSeen,
		common_macAddress,
		common_confirmBulkDelete,
		common_manufacturer,
		common_model,
		common_name,
		common_site,
		common_noEntityYet,
		common_rescan,
		common_serialNumber,
		common_assetTag,
		common_source,
		common_firmwareRevision,
		common_softwareRevision,
		common_operatingSystem,
		common_host,
		common_service,
		common_services,
		common_tags,
		common_unknownEntity,
		common_unknownSite,
		common_updated,
		common_contact,
		common_location,
		daemons_installPromptHosts,
		hosts_fields_presentedBy,
		hosts_fields_virtualizedBy,
		hosts_notVirtualized,
		hosts_snmp_chassisId,
		hosts_snmp_managementUrl,
		hosts_snmp_sysDescr,
		hosts_snmp_sysName,
		hosts_snmp_sysObjectId
	} from '$lib/paraglide/messages';
	import { entitySourceItems } from '$lib/shared/utils/entity-source';

	let { isReadOnly = false }: TabProps = $props();
	import {
		missingRootIds,
		virtualizationGroupKey,
		virtualizationGroupLabel,
		virtualizationTree
	} from '../virtualization-tree';
	import {
		useHostsQuery,
		useHostsByIds,
		useCreateHostMutation,
		useUpdateHostMutation,
		useDeleteHostMutation,
		useBulkDeleteHostsMutation,
		useConsolidateHostsMutation,
		useRescanHostMutation,
		type HostQueryOptions
	} from '../queries';
	import { useServicesByIds, useServicesCacheQuery } from '$lib/features/services/queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useIPAddressesQuery } from '$lib/features/ip-addresses/queries';
	import { usePortsQuery } from '$lib/features/ports/queries';
	import { formatPort } from '$lib/shared/utils/formatting';
	import { useInterfacesByIds, useInterfacesQuery } from '$lib/features/interfaces/queries';
	import { useDiscoveriesByIds } from '$lib/features/discovery/queries';
	import { discoveryRunIds, discoveryRunItems } from '$lib/features/discovery/columns';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { useSubnetsQuery, isContainerSubnet } from '$lib/features/subnets/queries';
	import type { Credential } from '$lib/features/credentials/types/base';
	import type { Interface } from '$lib/features/credentials/types/base';
	import { formatIPAddress } from '../address-labels';
	import { useSitesQuery } from '$lib/features/sites/queries';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import type { components } from '$lib/api/schema';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';
	import {
		fieldValueOptions,
		hasEmptyFieldValue,
		labelledFieldValueOptions,
		useFieldValuesQuery
	} from '$lib/shared/api/field-values';

	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];
	type HostOrderField = components['schemas']['HostOrderField'];
	type OrderDirection = components['schemas']['OrderDirection'];
	type EntitySourceType = components['schemas']['EntitySourceDiscriminants'];

	const HOST_FIELD_VALUES = '/api/v1/hosts/field-values/{field}';
	const SERVICE_FIELD_VALUES = '/api/v1/services/field-values/{field}';

	// Pagination state
	let pageSize = $state(20);
	let currentPage = $state(1);

	// Ordering state (for server-side ordering)
	let groupBy = $state<HostOrderField | undefined>(undefined);
	let orderBy = $state<HostOrderField | undefined>(undefined);
	let orderDirection = $state<OrderDirection>('asc');

	// Tag filter state (for server-side filtering)
	let tagIds = $state<string[]>([]);
	// Staleness filter state (server-side: the list is server-paginated)
	let stale = $state<boolean | null>(null);
	// Search state (server-side, for the same reason)
	let search = $state('');

	// Field filter state. Server-side for the same reason as the two above: the
	// client holds one page of hosts, so filtering here would narrow that page
	// while the total count kept describing every match.
	let filterSiteIds = $state<string[]>([]);
	let filterHidden = $state<boolean[]>([]);
	let filterVirtualizationServiceNames = $state<string[]>([]);
	let filterIncludeUnvirtualized = $state(false);
	let filterServiceNames = $state<string[]>([]);
	let filterSources = $state<EntitySourceType[]>([]);
	let filterManufacturers = $state<string[]>([]);
	let filterModels = $state<string[]>([]);
	let filterSysLocations = $state<string[]>([]);
	let filterOsFamilies = $state<HostOsFamily[]>([]);
	let filterCredentialIds = $state<string[]>([]);

	/** The hardware, OS and credential filters, shared by the list and the export. */
	function fieldFilterParams() {
		return {
			manufacturers: filterManufacturers.length > 0 ? filterManufacturers : undefined,
			models: filterModels.length > 0 ? filterModels : undefined,
			sys_locations: filterSysLocations.length > 0 ? filterSysLocations : undefined,
			os_families: filterOsFamilies.length > 0 ? filterOsFamilies : undefined,
			credential_ids: filterCredentialIds.length > 0 ? filterCredentialIds : undefined
		};
	}

	// Queries
	const organizationQuery = useOrganizationQuery();
	let org = $derived(organizationQuery.data);
	let hostLimit = $derived(org?.plan?.included_hosts ?? null);
	let canBuyMoreHosts = $derived(
		org?.plan?.host_cents !== undefined && org?.plan?.host_cents !== null
	);
	let onboarding = $derived((org?.onboarding ?? []) as OnboardingOperation[]);

	const tagsQuery = useTagsQuery();
	// Paginated hosts with server-side pagination, ordering, and tag filtering.
	//
	// Deliberately NOT gated on `isActive`, unlike the other tabs' list queries.
	// `useHostsQuery` is the last remaining writer of the ip-addresses / ports /
	// services / interfaces caches (it populates them from its nested response),
	// and those caches have no fetcher of their own. Gating this would leave the
	// services tab's binding chips and the host editor's interface lists empty for
	// anyone who never opens the hosts tab. Un-gate it only once those caches have
	// real queries — see planned-work/child-cache-rearchitecture.md.
	const hostsQuery = useHostsQuery((): HostQueryOptions => ({
		limit: pageSize,
		offset: (currentPage - 1) * pageSize,
		group_by: groupBy,
		order_by: orderBy,
		order_direction: orderDirection,
		tag_ids: tagIds.length > 0 ? tagIds : undefined,
		stale: stale ?? undefined,
		search: search || undefined,
		site_ids: filterSiteIds.length > 0 ? filterSiteIds : undefined,
		// Both values checked is no constraint, so it is sent as nothing.
		hidden: filterHidden.length === 1 ? filterHidden : undefined,
		virtualization_service_names:
			filterVirtualizationServiceNames.length > 0 ? filterVirtualizationServiceNames : undefined,
		include_unvirtualized: filterIncludeUnvirtualized || undefined,
		service_names: filterServiceNames.length > 0 ? filterServiceNames : undefined,
		sources: filterSources.length > 0 ? filterSources : undefined,
		...fieldFilterParams()
	}));
	const sitesQuery = useSitesQuery();
	useDaemonsQuery();
	const ipAddressesQuery = useIPAddressesQuery();
	const portsQuery = usePortsQuery();
	const interfacesQuery = useInterfacesQuery();
	const credentialsQuery = useCredentialsQuery();
	const subnetsQuery = useSubnetsQuery();
	// Filter options: the values the caller's hosts actually hold, counted by the server. The
	// loaded page would only offer the values on it, and the fixtures and caches would offer values
	// no host holds. None of these takes the tab's active filters, so the options do not shrink as
	// the user filters.
	const siteValuesQuery = useFieldValuesQuery(HOST_FIELD_VALUES, 'site_id');
	const virtualizedByValuesQuery = useFieldValuesQuery(HOST_FIELD_VALUES, 'virtualized_by');
	const sourceValuesQuery = useFieldValuesQuery(HOST_FIELD_VALUES, 'source');
	const osFamilyValuesQuery = useFieldValuesQuery(HOST_FIELD_VALUES, 'os_family');
	const manufacturerValuesQuery = useFieldValuesQuery(HOST_FIELD_VALUES, 'manufacturer');
	const modelValuesQuery = useFieldValuesQuery(HOST_FIELD_VALUES, 'model');
	const sysLocationValuesQuery = useFieldValuesQuery(HOST_FIELD_VALUES, 'sys_location');
	// Every service belongs to a host, so the services' names are the names some host runs.
	const serviceNameValuesQuery = useFieldValuesQuery(SERVICE_FIELD_VALUES, 'name');

	// Selective service lookup - only fetches services needed for virtualization display
	// Extract service IDs from visible hosts for "Virtualized By" field
	const servicesQuery = useServicesByIds(() => {
		return (hostsQuery.data?.items ?? [])
			.map((h) => h.virtualization_service_id)
			.filter((id): id is string => id != null)
			.filter((id, idx, arr) => arr.indexOf(id) === idx);
	});
	// The interface a virtualizing host presents each network identity from. It belongs to the
	// virtualizing host, which is rarely on this page, so fetched by id.
	const presentingInterfacesQuery = useInterfacesByIds(() => [
		...new Set(
			(hostsQuery.data?.items ?? [])
				.map((h) => h.virtualization_interface_id)
				.filter((id): id is string => id != null)
		)
	]);
	const discoveryRunsQuery = useDiscoveriesByIds(() =>
		discoveryRunIds(hostsQuery.data?.items ?? [])
	);
	// The hosts that head a virtualization tree on this page but sit on another, so the group
	// header can name them.
	const virtualizationRootsQuery = useHostsByIds(() =>
		missingRootIds(hostsQuery.data?.items ?? [])
	);

	// Mutations
	const createHostMutation = useCreateHostMutation();
	const updateHostMutation = useUpdateHostMutation();
	const deleteHostMutation = useDeleteHostMutation();
	const bulkDeleteHostsMutation = useBulkDeleteHostsMutation();
	const consolidateHostsMutation = useConsolidateHostsMutation();
	const rescanHostMutation = useRescanHostMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let hostsData = $derived(hostsQuery.data?.items ?? []);
	let hostsPagination = $derived(hostsQuery.data?.pagination ?? null);
	let servicesData = $derived(servicesQuery.data ?? []);
	let presentingInterfacesData = $derived(presentingInterfacesQuery.data ?? []);
	let discoveryRunsData = $derived(discoveryRunsQuery.data ?? []);
	let virtualizationRoots = $derived(
		new Map([...hostsData, ...(virtualizationRootsQuery.data ?? [])].map((h) => [h.id, h]))
	);
	const servicesCacheQuery = useServicesCacheQuery();
	let allServicesData = $derived(servicesCacheQuery.data ?? []);
	let sitesData = $derived(sitesQuery.data ?? []);
	let ipAddressesData = $derived(ipAddressesQuery.data ?? []);
	let portsData = $derived(portsQuery.data ?? []);
	let interfacesData = $derived(interfacesQuery.data ?? []);
	let credentialsData = $derived(credentialsQuery.data ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);
	// Only show full loading on initial load (no data yet)
	let isInitialLoading = $derived(hostsQuery.isPending && !hostsQuery.data);

	// Host limit tracking
	let totalHostCount = $derived(hostsPagination?.total_count ?? hostsData.length);
	let isAtHostLimit = $derived(
		hostLimit !== null && totalHostCount >= hostLimit && !canBuyMoreHosts
	);
	let isNearHostLimit = $derived(
		hostLimit !== null &&
			totalHostCount >= hostLimit - 5 &&
			totalHostCount < hostLimit &&
			!canBuyMoreHosts
	);

	// Page change handler for server-side pagination
	function handlePageChange(page: number, newPageSize: number) {
		currentPage = page;
		pageSize = newPageSize;
	}

	// Order change handler for server-side ordering
	// Values are now directly HostOrderField values from the orderField property
	function handleOrderChange(
		groupField: string | null,
		orderField: string | null,
		direction: 'asc' | 'desc'
	) {
		groupBy = (groupField as HostOrderField) ?? undefined;
		orderBy = (orderField as HostOrderField) ?? undefined;
		orderDirection = direction;
	}

	// Tag filter change handler for server-side filtering
	function handleTagFilterChange(selectedTagIds: string[]) {
		tagIds = selectedTagIds;
		// Reset to page 1 is handled by DataControls
	}

	function handleStaleFilterChange(next: boolean | null) {
		stale = next;
	}

	// Search change handler for server-side search (debounced by DataControls)
	function handleSearchChange(query: string) {
		search = query;
	}

	/**
	 * Server-side field filter handler.
	 *
	 * The panel offers what the user reads — a site's name, a service's name —
	 * while the API filters on ids, so each case resolves the labels back through
	 * the same data the options were built from. Every key here must match a
	 * field marked `serverFiltered`; an unhandled one would filter nothing at
	 * all, since DataControls skips the client pass for those fields.
	 */
	function handleFilterChange(fieldKey: string, values: string[]) {
		switch (fieldKey) {
			case 'site_id':
				filterSiteIds = idsForNames(values, sitesData);
				break;
			case 'hidden':
				filterHidden = values.map((value) => value === 'true');
				break;
			case 'virtualized_by':
				// "Not Virtualized" is a choice about absence, so it is carried as
				// its own flag rather than as a name nothing would match. The rest are the
				// virtualizing services' names, which the server filters on directly: the
				// options come from every host, and no client cache holds every service.
				filterIncludeUnvirtualized = values.includes(hosts_notVirtualized());
				filterVirtualizationServiceNames = values.filter(
					(value) => value !== hosts_notVirtualized()
				);
				break;
			case 'services':
				filterServiceNames = values;
				break;
			case 'source':
				// Fixture ids are the backend's `EntitySourceDiscriminants` names, emitted from
				// that same enum, so an id is a valid filter value by construction.
				filterSources = entitySources
					.getItems()
					.filter((source) => values.includes(entitySources.getName(source.id)))
					.map((source) => source.id as EntitySourceType);
				break;
			// The field-values options are the stored strings themselves, so they pass through.
			case 'manufacturer':
				filterManufacturers = values;
				break;
			case 'model':
				filterModels = values;
				break;
			case 'sys_location':
				filterSysLocations = values;
				break;
			case 'os_family':
				filterOsFamilies = hostOsFamilyIds.filter((family) =>
					values.includes(hostOsFamilyName(family))
				);
				break;
			case 'credentials':
				filterCredentialIds = idsForNames(values, credentialsData);
				break;
			default:
				throw new Error(
					`HostTab: no server-side filter handles "${fieldKey}". A serverFiltered field ` +
						`without a case here filters nothing — neither the client nor the server.`
				);
		}
	}

	/** Resolve display names back to the ids the API filters on. */
	function idsForNames(names: string[], from: { id: string; name: string }[]): string[] {
		const wanted = new Set(names);
		return from.filter((entry) => wanted.has(entry.name)).map((entry) => entry.id);
	}

	// Export modal state
	let showExportModal = $state(false);
	// Carries the field filters too: the export handler applies the same
	// `apply_field_filters` as the list, so an export mirrors what the user was
	// looking at rather than the whole network.
	let exportParams = $derived({
		tag_ids: tagIds.length > 0 ? tagIds : undefined,
		order_by: orderBy,
		order_direction: orderDirection,
		site_ids: filterSiteIds.length > 0 ? filterSiteIds : undefined,
		hidden: filterHidden.length === 1 ? filterHidden : undefined,
		virtualization_service_names:
			filterVirtualizationServiceNames.length > 0 ? filterVirtualizationServiceNames : undefined,
		include_unvirtualized: filterIncludeUnvirtualized || undefined,
		service_names: filterServiceNames.length > 0 ? filterServiceNames : undefined,
		sources: filterSources.length > 0 ? filterSources : undefined,
		...fieldFilterParams()
	});

	let showHostEditor = $state(false);
	// Track the host being edited by id + a snapshot, and resolve `editingHost` from the
	// live query cache. So when an external change (e.g. a credential assignment removed
	// elsewhere) invalidates and refetches hosts, the open editor reflects it without a
	// page reload. The snapshot is the fallback for hosts not in the current page.
	let editingHostId = $state<string | null>(null);
	let editingHostSnapshot = $state<Host | null>(null);
	let editingHost = $derived(
		editingHostId ? (hostsData.find((h) => h.id === editingHostId) ?? editingHostSnapshot) : null
	);
	function setEditingHost(host: Host | null) {
		editingHostId = host?.id ?? null;
		editingHostSnapshot = host;
	}

	let otherHost = $state<Host | null>(null);
	let showHostConsolidationModal = $state(false);

	// Deep-link: open host editor from URL (handles both fresh open and entity switch)
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'host-editor',
			hostsData,
			showHostEditor,
			editingHost?.id
		);
		if (result !== undefined) {
			setEditingHost(result);
			showHostEditor = true;
		}
	});

	// What a host holds. These were resolved inside HostCard, so the table had no
	// way to show them; resolving here gives both views the same columns.
	function hostCredentials(host: Host): Credential[] {
		return (host.credential_assignments ?? [])
			.map((a) => credentialsData.find((c) => c.id === a.credential_id))
			.filter((c): c is Credential => c != null);
	}

	function hostInterfaces(host: Host): Interface[] {
		return interfacesData.filter((i) => i.host_id === host.id);
	}

	function hostIPAddresses(host: Host) {
		return ipAddressesData.filter((i) => i.host_id === host.id);
	}

	function hostPorts(host: Host) {
		return portsData.filter((p) => p.host_id === host.id).sort((a, b) => a.number - b.number);
	}

	/** The host's distinct MACs across its IP addresses and interfaces, lowest first — the
	 *  first is the one the server sorts the host by. */
	function hostMacAddresses(host: Host): string[] {
		const macs = [...hostIPAddresses(host), ...hostInterfaces(host)]
			.map((i) => i.mac_address?.toUpperCase())
			.filter((m): m is string => !!m);
		return [...new Set(macs)].sort();
	}

	function isContainerSubnetFn(subnetId: string): boolean {
		const subnet = subnetsData.find((s) => s.id === subnetId);
		return subnet ? isContainerSubnet(subnet) : false;
	}

	// Define field configuration for the DataTableControls
	// Uses defineFields to ensure all HostOrderField values are covered
	let hostFields = $derived(
		defineFields<Host, HostOrderField>(
			{
				// Identity fields: grouping by one would render a header per host.
				name: {
					label: common_name(),
					type: 'string',
					searchable: true,
					groupable: false,
					// The title, not the stored `name`. `getValue` rather than `display.cell`
					// because this one accessor also feeds the row header cell, the row
					// checkbox's accessible name and the card title — a `cell` snippet would fix
					// the table and leave those three rendering an empty string.
					//
					// The key stays `name`: it is the `HostOrderField` sent to the server, which
					// now orders by the same ladder this renders.
					getValue: (host) => hostDisplayName(host),
					display: {
						primary: true,
						width: 220,
						order: 0,
						// Only when the name shown is the stored one, not a rung further down the ladder.
						getSource: (host) => (host.display_name_rung === 'Name' ? host.name_source : null)
					}
				},
				hostname: {
					label: common_hostname(),
					type: 'string',
					searchable: true,
					groupable: false,
					display: { hiddenByDefault: true, getSource: (host) => host.hostname_source }
				},
				virtualized_by: {
					label: hosts_fields_virtualizedBy(),
					type: 'string',
					searchable: true,
					filterable: true,
					serverFiltered: true,
					// The names of the services that virtualize some host, and "Not Virtualized"
					// only when some host has no virtualizing service (counted as '').
					filterOptions: fieldValueOptions(virtualizedByValuesQuery.data).concat(
						hasEmptyFieldValue(virtualizedByValuesQuery.data) ? [hosts_notVirtualized()] : []
					),
					groupable: true,
					// Grouped as a tree: every host under the host at the top of its chain, parent
					// first. The server groups, orders and counts by the root's id ('' for a host in no
					// tree); the column itself still shows, sorts and filters on the immediate runtime.
					getGroupValue: virtualizationGroupKey,
					getGroupLabel: (host) =>
						virtualizationGroupLabel(host, virtualizationRoots, {
							notVirtualized: hosts_notVirtualized(),
							unknownRoot: common_unknownEntity({ entity: common_host() })
						}),
					tree: virtualizationTree,
					getValue: (host) => {
						if (host.virtualization_service_id) {
							const virtualizationService = servicesData.find(
								(s) => s.id === host.virtualization_service_id
							);
							if (virtualizationService) {
								return (
									virtualizationService?.name || common_unknownEntity({ entity: common_service() })
								);
							}
						}
						return hosts_notVirtualized();
					},
					display: {
						// Not in the default column set — it stays a filter and group axis.
						hiddenByDefault: true,
						// No chips when a host isn't virtualized, so the cell renders the
						// em dash rather than repeating "Not Virtualized" down the column.
						// `getValue` keeps the phrase, so the filter still offers it.
						getItems: (host) => {
							const service = servicesData.find((s) => s.id === host.virtualization_service_id);
							if (!service) return [];
							return [
								{
									id: service.id,
									label: service.name,
									color: entities.getColorHelper('Service').color,
									entityRef: entityRef('Service', service.id, service)
								}
							];
						}
					}
				},
				interface_ip: {
					// The card calls this "IP Addresses"; it named one thing two ways.
					label: common_ipAddresses(),
					type: 'string',
					searchable: true,
					// Near-unique per host, so grouping by it is one header per host.
					groupable: false,
					getValue: (host) => {
						const iface = ipAddressesData
							.filter((i) => i.host_id === host.id)
							.sort((a, b) => (a.position ?? 0) - (b.position ?? 0))[0];
						return iface?.ip_address ?? '';
					},
					display: {
						order: 5,
						// The server orders on the primary address, but a host usually has
						// several — showing only the first would misrepresent the row.
						getItems: (host) =>
							hostIPAddresses(host)
								.sort((a, b) => (a.position ?? 0) - (b.position ?? 0))
								.map((i) => ({
									id: i.id,
									label: formatIPAddress(i, isContainerSubnetFn),
									color: entities.getColorHelper('IPAddress').color,
									entityRef: entityRef('IPAddress', i.id, i, { subnets: subnetsData })
								}))
					}
				},
				mac_address: {
					label: common_macAddress(),
					type: 'string',
					searchable: true,
					groupable: false,
					getValue: (host) => hostMacAddresses(host)[0] ?? null,
					display: {
						order: 5,
						getItems: (host) =>
							hostMacAddresses(host).map((mac) => ({
								id: mac,
								label: mac
							}))
					}
				},
				site_id: {
					label: common_site(),
					type: 'string',
					searchable: true,
					filterable: true,
					serverFiltered: true,
					groupable: true,
					// The sites some host is on, by name.
					filterOptions: labelledFieldValueOptions(
						siteValuesQuery.data,
						(id) => sitesData.find((n) => n.id === id)?.name
					),
					// Displayed as a name, but grouped by id on the server.
					getGroupValue: (item) => item.site_id,
					getValue: (item) =>
						sitesData.find((n) => n.id == item.site_id)?.name || common_unknownSite(),
					display: { order: 2, getItems: (item) => siteItems(item.site_id, sitesData) }
				},
				// Audit dates stay available but off by default: 12 columns at once
				// is unreadable, and these are rarely what someone is scanning for.
				created_at: { label: common_created(), type: 'date', display: { hiddenByDefault: true } },
				updated_at: { label: common_updated(), type: 'date', display: { hiddenByDefault: true } },
				last_seen_at: {
					label: common_lastSeen(),
					type: 'date',
					staleFilter: true,
					display: { recency: true, order: 1, getItems: lastSeenItems(() => sitesData, 'Host') }
				},
				// How the host came to exist, read from `source.type`. An inferred host (one a
				// neighbour advertised and nothing scanned) looks the same as a down device by its
				// ports and services, so the chip and filter read the stamped source, never those.
				source: {
					label: common_source(),
					type: 'string',
					filterable: true,
					serverFiltered: true,
					// The sources some host was stamped with, by name.
					filterOptions: labelledFieldValueOptions(sourceValuesQuery.data, (id) =>
						entitySources.getName(id)
					),
					getValue: (host) => entitySources.getName(host.source.type),
					// The server groups on the raw `source.type`.
					getGroupValue: (host) => host.source.type,
					display: { order: 1, getItems: (host) => entitySourceItems(host.source) }
				},
				// Hardware identity, off by default: populated only for hosts a credentialed scan
				// reached. The options come from every stored value, and the server groups on the
				// stored string, which is also what renders.
				manufacturer: {
					label: common_manufacturer(),
					type: 'string',
					filterable: true,
					serverFiltered: true,
					filterOptions: fieldValueOptions(manufacturerValuesQuery.data),
					display: { hiddenByDefault: true, getSource: (host) => host.manufacturer_source }
				},
				model: {
					label: common_model(),
					type: 'string',
					filterable: true,
					serverFiltered: true,
					filterOptions: fieldValueOptions(modelValuesQuery.data),
					display: { hiddenByDefault: true, getSource: (host) => host.model_source }
				},
				sys_location: {
					label: common_location(),
					type: 'string',
					filterable: true,
					serverFiltered: true,
					filterOptions: fieldValueOptions(sysLocationValuesQuery.data),
					display: { hiddenByDefault: true, getSource: (host) => host.sys_location_source }
				},
				// Sorted, grouped and filtered by family; the cell shows the full product and release.
				os_family: {
					label: common_operatingSystem(),
					type: 'string',
					filterable: true,
					serverFiltered: true,
					// The families some host runs, resolved through the same list the handler
					// maps names back with.
					filterOptions: labelledFieldValueOptions(osFamilyValuesQuery.data, (value) => {
						const family = hostOsFamilyIds.find((id) => id === value);
						return family ? hostOsFamilyName(family) : null;
					}),
					getValue: (host) => (host.os ? hostOsFamilyName(host.os.family) : null),
					getGroupValue: (host) => host.os?.family ?? null,
					display: { hiddenByDefault: true, cell: osCell }
				},
				hidden: {
					label: common_hidden(),
					type: 'boolean',
					filterable: true,
					serverFiltered: true,
					// Useful as a filter, but almost always false — a column of "false"
					// earns none of the width it takes.
					display: { hiddenByDefault: true }
				}
			},
			[
				{
					key: 'description',
					label: common_description(),
					type: 'string',
					searchable: true,
					display: { hiddenByDefault: true }
				},
				// Off by default: populated only for hosts a credentialed scan reached. Identifiers,
				// vendor version strings, opaque OIDs and free text, so none sorts, groups or filters.
				{
					key: 'serial_number',
					label: common_serialNumber(),
					type: 'string',
					display: { hiddenByDefault: true, getSource: (host) => host.serial_number_source }
				},
				{
					key: 'asset_tag',
					label: common_assetTag(),
					type: 'string',
					display: { hiddenByDefault: true, getSource: (host) => host.asset_tag_source }
				},
				{
					key: 'firmware_revision',
					label: common_firmwareRevision(),
					type: 'string',
					display: { hiddenByDefault: true, getSource: (host) => host.firmware_revision_source }
				},
				{
					key: 'software_revision',
					label: common_softwareRevision(),
					type: 'string',
					display: { hiddenByDefault: true, getSource: (host) => host.software_revision_source }
				},
				{
					key: 'sys_name',
					label: hosts_snmp_sysName(),
					type: 'string',
					display: { hiddenByDefault: true, getSource: (host) => host.sys_name_source }
				},
				{
					key: 'sys_descr',
					label: hosts_snmp_sysDescr(),
					type: 'string',
					display: { hiddenByDefault: true, getSource: (host) => host.sys_descr_source }
				},
				{
					key: 'sys_object_id',
					label: hosts_snmp_sysObjectId(),
					type: 'string',
					display: { hiddenByDefault: true, getSource: (host) => host.sys_object_id_source }
				},
				{
					key: 'sys_contact',
					label: common_contact(),
					type: 'string',
					display: { hiddenByDefault: true, getSource: (host) => host.sys_contact_source }
				},
				{
					key: 'chassis_id',
					label: hosts_snmp_chassisId(),
					type: 'string',
					display: { hiddenByDefault: true, getSource: (host) => host.chassis_id_source }
				},
				{
					key: 'management_url',
					label: hosts_snmp_managementUrl(),
					type: 'string',
					display: { hiddenByDefault: true, getSource: (host) => host.management_url_source }
				},
				{
					key: 'presented_by',
					label: hosts_fields_presentedBy(),
					type: 'string',
					display: {
						hiddenByDefault: true,
						getItems: (host) => {
							const iface = presentingInterfacesData.find(
								(i) => i.id === host.virtualization_interface_id
							);
							if (!iface) return [];
							return [
								{
									id: iface.id,
									label: interfaceDisplayName(iface),
									color: entities.getColorHelper('Interface').color,
									entityRef: entityRef('Interface', iface.id, iface)
								}
							];
						}
					}
				},
				{
					key: 'first_found_by',
					label: common_firstFoundBy(),
					type: 'string',
					display: {
						hiddenByDefault: true,
						getItems: (host) => discoveryRunItems(host.first_discovery_id, discoveryRunsData)
					}
				},
				{
					key: 'last_found_by',
					label: common_lastFoundBy(),
					type: 'string',
					display: {
						hiddenByDefault: true,
						getItems: (host) => discoveryRunItems(host.last_discovery_id, discoveryRunsData)
					}
				},
				{
					key: 'tags',
					label: common_tags(),
					type: 'array',
					searchable: true,
					filterable: true,
					// Search and filter match the same names the cell renders, so a row
					// can never show a tag the filter above it disagrees with.
					getValue: (entity) => tagNames(entity.tags, tagsData)
				},
				{
					key: 'credentials',
					label: common_credentials(),
					type: 'array',
					searchable: true,
					filterable: true,
					serverFiltered: true,
					// Names, resolved back to ids for the server. Only credentials assigned to some host:
					// the rest match nothing.
					filterOptions: credentialsData
						.filter((c) => (c.host_assignments ?? []).length > 0)
						.map((c) => c.name),
					getValue: (host) => hostCredentials(host).map((c) => c.name),
					display: {
						order: 3,
						getItems: (host) => credentialItems(hostCredentials(host))
					}
				},
				{
					key: 'interfaces',
					label: common_interfaces(),
					type: 'array',
					searchable: true,
					getValue: (host) => hostInterfaces(host).map((i) => interfaceDisplayName(i)),
					display: {
						order: 4,
						getItems: (host) =>
							hostInterfaces(host).map((iface) => ({
								id: iface.id,
								label: interfaceDisplayName(iface),
								color: entities.getColorHelper('Interface').color,
								entityRef: entityRef('Interface', iface.id, iface)
							}))
					}
				},
				{
					// Off by default: Services already says what answers on a port, and the open ports
					// alone matter when nothing was matched to them.
					key: 'ports',
					label: common_ports(),
					type: 'array',
					searchable: true,
					getValue: (host) => hostPorts(host).map((p) => formatPort(p)),
					display: {
						hiddenByDefault: true,
						getItems: (host) =>
							hostPorts(host).map((port) => ({
								id: port.id,
								label: formatPort(port),
								color: entities.getColorHelper('Port').color,
								// What the popover needs to name the service on the port and its addresses.
								entityRef: entityRef('Port', port.id, port, {
									currentServices: allServicesData.filter((s) => s.host_id === host.id),
									ip_addresses: hostIPAddresses(host),
									isContainerSubnet: isContainerSubnetFn
								})
							}))
					}
				},
				{
					key: 'services',
					label: common_services(),
					type: 'array',
					searchable: true,
					filterable: true,
					serverFiltered: true,
					// Names, not ids: the server matches a host's services by name. The options are
					// every live service name the server counts; the services cache only holds the
					// loaded page's.
					filterOptions: fieldValueOptions(serviceNameValuesQuery.data),
					getValue: (host) =>
						allServicesData.filter((s) => s.host_id === host.id).map((s) => s.name),
					display: {
						order: 6,
						// Containers are services too, so they share this column rather
						// than getting one of their own; the colour carries the
						// distinction instead of a second header.
						getItems: (host) =>
							allServicesData
								.filter((s) => s.host_id === host.id)
								.map((s) => ({
									id: s.id,
									label: s.name,
									color: s.virtualization_metadata
										? concepts.getColorHelper('Containerization').color
										: entities.getColorHelper('Service').color,
									entityRef: entityRef('Service', s.id, s)
								}))
					}
				}
			]
		)
	);

	function handleCreateHost() {
		setEditingHost(null);
		showHostEditor = true;
	}

	/**
	 * Row actions for table mode, matching what the card offers.
	 *
	 * The table never renders a card, so the actions the card builds for itself
	 * are not reachable from it — the tab already owns every handler, so it is
	 * the natural place to describe them once for both.
	 */
	function hostActions(host: Host): CardAction[] {
		if (isReadOnly) return [];

		return [
			{ label: common_edit(), icon: Edit, onClick: () => handleEditHost(host) },
			// A rescan targets the host's known addresses, so a host with none has nothing to
			// scan and the endpoint answers 400. Offering the action anyway meant the only way to
			// find that out was to trigger it and read the error toast. Devices identified purely
			// by a MAC are the reason such a host exists at all.
			...(hostIPAddresses(host).length > 0
				? [{ label: common_rescan(), icon: RefreshCw, onClick: () => handleRescanHost(host) }]
				: []),
			{ label: common_consolidate(), icon: Replace, onClick: () => handleStartConsolidate(host) },
			{
				label: common_hide(),
				icon: Eye,
				class: host.hidden ? 'text-blue-400' : '',
				onClick: () => handleHostHide(host)
			},
			{
				label: common_delete(),
				icon: Trash2,
				class: 'btn-icon-danger',
				onClick: () => handleDeleteHost(host)
			}
		];
	}

	function handleEditHost(host: Host) {
		setEditingHost(host);
		showHostEditor = true;
	}

	function handleStartConsolidate(host: Host) {
		otherHost = host;
		showHostConsolidationModal = true;
	}

	function handleRescanHost(host: Host) {
		rescanHostMutation.mutate({ id: host.id, name: hostDisplayName(host) });
	}

	function handleDeleteHost(host: Host) {
		if (confirm(common_confirmDeleteName({ name: hostDisplayName(host) }))) {
			deleteHostMutation.mutate(host.id);
		}
	}

	async function handleHostCreate(data: CreateHostWithServicesRequest) {
		try {
			await createHostMutation.mutateAsync(data);
			showHostEditor = false;
			setEditingHost(null);
		} catch {
			// Error handled by mutation
		}
	}

	async function handleHostUpdate(data: UpdateHostWithServicesRequest) {
		try {
			await updateHostMutation.mutateAsync(data);
			showHostEditor = false;
			setEditingHost(null);
		} catch {
			// Error handled by mutation
		}
	}

	async function handleConsolidateHosts(destinationHostId: string, otherHostId: string) {
		try {
			await consolidateHostsMutation.mutateAsync({
				destinationHostId,
				otherHostId,
				otherHostName: otherHost ? hostDisplayName(otherHost) : undefined
			});
			showHostConsolidationModal = false;
			otherHost = null;
		} catch {
			// Error handled by mutation
		}
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(common_confirmBulkDelete({ count: ids.length, entity: common_hosts() }))) {
			await bulkDeleteHostsMutation.mutateAsync(ids);
		}
	}

	function getHostTags(host: Host): string[] {
		return host.tags;
	}

	async function handleHostHide(host: Host) {
		const updatedHost = { ...host, hidden: !host.hidden };
		await updateHostMutation.mutateAsync({
			host: updatedHost,
			ip_addresses: null,
			ports: null,
			services: null
		});
	}

	function handleCloseHostEditor() {
		showHostEditor = false;
		setEditingHost(null);
	}
</script>

{#snippet osCell(host: Host)}
	{#if host.os}
		<OsTag os={host.os} source={host.os_source} />
	{:else}
		<span class="text-tertiary text-sm">—</span>
	{/if}
{/snippet}

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={common_hosts()}>
		<svelte:fragment slot="actions">
			{#if hasDaemon(onboarding)}
				<div class="flex items-center gap-3">
					{#if hostLimit !== null && !canBuyMoreHosts}
						<span
							class="text-sm {isAtHostLimit
								? 'text-amber-400'
								: isNearHostLimit
									? 'text-yellow-400'
									: 'text-tertiary'}"
						>
							{totalHostCount} / {hostLimit}
						</span>
					{/if}
					{#if !isReadOnly}
						{#if isAtHostLimit}
							<UpgradeButton feature="hosts" surface="hosts_tab" gate_type="limit_hit" />
						{:else}
							{#if isNearHostLimit}
								<UpgradeButton feature="hosts" surface="hosts_tab" gate_type="limit_hit" />
							{/if}
							<button class="btn-primary flex items-center" onclick={handleCreateHost}
								><Plus class="h-5 w-5" />{common_create()}</button
							>
						{/if}
					{/if}
				</div>
			{/if}
		</svelte:fragment>
	</TabHeader>

	{#if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title={daemons_installPromptHosts()} {isReadOnly} />
	{:else if isInitialLoading}
		<!-- Loading state (only on initial load) -->
		<Loading />
	{:else if hostsData.length === 0 && !hostsPagination}
		<!-- Empty state -->
		<EmptyState
			title={common_noEntityYet({ entity: common_hosts() })}
			subtitle=""
			onClick={handleCreateHost}
			cta={common_create()}
		/>
	{:else}
		<DataControls
			items={hostsData}
			fields={hostFields}
			storageKey="scanopy-hosts-table-state"
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			entityType={isReadOnly ? undefined : 'Host'}
			getItemTags={getHostTags}
			getItemId={(item) => item.id}
			getIcon={(host) => {
				const first = allServicesData.find(
					(s) => s.host_id === host.id && s.service_definition !== 'Unclaimed Open Ports'
				);
				return {
					icon: first
						? serviceDefinitions.getIconComponent(first.service_definition)
						: entities.getIconComponent('Host'),
					color: entities.getColorHelper('Host').icon
				};
			}}
			getLink={(host) => (host.hostname ? `http://${host.hostname}` : undefined)}
			serverPagination={hostsPagination}
			onPageChange={handlePageChange}
			onOrderChange={handleOrderChange}
			onTagFilterChange={handleTagFilterChange}
			onFilterChange={handleFilterChange}
			onStaleFilterChange={handleStaleFilterChange}
			onSearchChange={handleSearchChange}
			onExportClick={() => {
				showExportModal = true;
			}}
			getActions={hostActions}
			entityLabel={common_hosts()}
		></DataControls>
	{/if}
</div>

<HostEditor
	isOpen={showHostEditor}
	name="host-editor"
	host={editingHost}
	onCreate={handleHostCreate}
	onDelete={handleDeleteHost}
	onUpdate={handleHostUpdate}
	onClose={handleCloseHostEditor}
/>

<HostConsolidationModal
	isOpen={showHostConsolidationModal}
	{otherHost}
	onConsolidate={handleConsolidateHosts}
	onClose={() => (showHostConsolidationModal = false)}
/>

<HostExportModal
	isOpen={showExportModal}
	onClose={() => (showExportModal = false)}
	{exportParams}
/>
