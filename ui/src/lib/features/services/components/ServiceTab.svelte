<script lang="ts">
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import {
		defineFields,
		entityRef,
		type CardAction,
		type LabelledCardFieldItem
	} from '$lib/shared/components/data/types';
	import { usePortsQuery } from '$lib/features/ports/queries';
	import { useIPAddressesQuery } from '$lib/features/ip-addresses/queries';
	import { useSubnetsQuery, isContainerSubnet } from '$lib/features/subnets/queries';
	import { formatIPAddress } from '$lib/features/hosts/queries';
	import { formatPort } from '$lib/shared/utils/formatting';
	import { lastSeenItems } from '$lib/shared/utils/freshness';
	import type { IPAddress, Port } from '$lib/features/hosts/types/base';
	import { tagNames } from '$lib/features/tags/columns';
	import { networkItems } from '$lib/features/networks/columns';
	import { entities } from '$lib/shared/stores/metadata';
	import { Trash2, Edit } from 'lucide-svelte';
	import type { Service } from '../types/base';
	import { matchConfidenceLabel } from '$lib/shared/types';
	import ServiceEditModal from './ServiceEditModal.svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import {
		useServicesQuery,
		useUpdateServiceMutation,
		useDeleteServiceMutation,
		useBulkDeleteServicesMutation,
		type ServicesQueryParams,
		useServicesCacheQuery
	} from '../queries';
	import { useHostsByIds, useHostSummariesQuery } from '$lib/features/hosts/queries';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import type { TabProps } from '$lib/shared/types';
	import type { components } from '$lib/api/schema';
	import { SvelteMap } from 'svelte/reactivity';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { closeModal, modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import {
		common_confirmBulkDelete,
		common_confirmDeleteName,
		common_containerized,
		common_created,
		common_delete,
		common_edit,
		common_host,
		common_lastSeen,
		common_category,
		common_name,
		common_network,
		common_noEntityYet,
		common_port,
		common_position,
		common_services,
		common_tags,
		common_type,
		common_unbound,
		common_portBindings,
		common_ipAddressBindings,
		common_unknown,
		common_unknownEntity,
		common_unknownNetwork,
		common_updated,
		daemons_installPromptServices,
		services_matchConfidence,
		services_notContainerized,
		services_notDiscovered,
		services_subtitle
	} from '$lib/paraglide/messages';
	import {
		serviceDefinitions,
		serviceCategories as serviceCategoryMeta,
		concepts,
		ports as ports_metadata
	} from '$lib/shared/stores/metadata';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';

	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];
	type ServiceOrderField = components['schemas']['ServiceOrderField'];
	type OrderDirection = components['schemas']['OrderDirection'];

	// Well-known port numbers for the filter panel. People look for 443, not
	// "HTTPS", so the options are the numbers themselves. Deduplicated because
	// some ports have both a TCP and a UDP definition (53, 515), and stripped of
	// the Custom entry, whose number is a placeholder 0. Free-form port entry
	// would need a control the value-list filter panel doesn't have.
	let wellKnownPortNumbers = $derived(
		[
			...new Set(
				(ports_metadata.getItems() ?? [])
					.filter((port) => !port.metadata.is_custom)
					.map((port) => port.metadata.number)
			)
		]
			.sort((a, b) => a - b)
			.map(String)
	);

	let { isReadOnly = false }: TabProps = $props();

	// Organization query for onboarding state
	const organizationQuery = useOrganizationQuery();
	let onboarding = $derived((organizationQuery.data?.onboarding ?? []) as OnboardingOperation[]);

	// Pagination state (managed by DataControls, updated via callback)
	let pageSize = $state(20);
	let currentPage = $state(1);

	// Ordering state (for server-side ordering)
	let groupBy = $state<ServiceOrderField | undefined>(undefined);
	let orderBy = $state<ServiceOrderField | undefined>(undefined);
	let orderDirection = $state<OrderDirection>('asc');

	// Tag filter state (for server-side filtering)
	let tagIds = $state<string[]>([]);

	// Exclude categories state (for server-side filtering)
	let excludeCategories = $state<string[]>(['OpenPorts']);

	// Port filter (server-side: a client-side pass would only narrow the loaded
	// page)
	let ports = $state<number[]>([]);

	// Staleness filter state (server-side: the list is server-paginated)
	let stale = $state<boolean | null>(null);

	// Search state (server-side, for the same reason)
	let search = $state('');

	// The remaining field filters, server-side for the same reason: the client
	// holds one page of services, so filtering here would narrow that page while
	// the total count kept describing every match.
	let filterHostIds = $state<string[]>([]);
	let filterNetworkIds = $state<string[]>([]);
	let filterServiceDefinitions = $state<string[]>([]);
	let filterVirtualizationServiceIds = $state<string[]>([]);
	let filterIncludeUncontainerized = $state(false);

	// Queries
	const tagsQuery = useTagsQuery();
	// Paginated services with server-side pagination, ordering, and tag filtering
	const servicesQuery = useServicesQuery((): ServicesQueryParams => ({
		limit: pageSize,
		offset: (currentPage - 1) * pageSize,
		group_by: groupBy,
		order_by: orderBy,
		order_direction: orderDirection,
		tag_ids: tagIds.length > 0 ? tagIds : undefined,
		stale: stale ?? undefined,
		search: search || undefined,
		ports: ports.length > 0 ? ports : undefined,
		exclude_categories:
			excludeCategories.length > 0
				? (excludeCategories as components['schemas']['ServiceCategory'][])
				: undefined,
		host_ids: filterHostIds.length > 0 ? filterHostIds : undefined,
		network_ids: filterNetworkIds.length > 0 ? filterNetworkIds : undefined,
		service_definitions: filterServiceDefinitions.length > 0 ? filterServiceDefinitions : undefined,
		virtualization_service_ids:
			filterVirtualizationServiceIds.length > 0 ? filterVirtualizationServiceIds : undefined,
		include_uncontainerized: filterIncludeUncontainerized || undefined
	}));
	const networksQuery = useNetworksQuery();
	const portsQuery = usePortsQuery();
	// Option sources for the server-side filters. The lists below are the tab's
	// display data, scoped to the loaded page — filter options have to come from
	// the full sets or the panel only ever offers what is already on screen.
	const allHostsQuery = useHostSummariesQuery({});
	const servicesCacheQuery = useServicesCacheQuery();
	const ipAddressesQuery = useIPAddressesQuery();
	const subnetsQuery = useSubnetsQuery();

	// Selective host lookup - only fetches hosts needed for service display
	// Extract host IDs from visible services for host name display
	const hostsQuery = useHostsByIds(() => {
		const ids = (servicesQuery.data?.items ?? [])
			.filter((s) => s.host_id)
			.map((s) => s.host_id)
			.filter((id, idx, arr) => arr.indexOf(id) === idx);
		// Include host for service navigated via EntityTag
		const ms = $modalState;
		if (ms.name === 'service-editor' && ms.entityData?.host_id) {
			const hostId = ms.entityData.host_id as string;
			if (!ids.includes(hostId)) ids.push(hostId);
		}
		return ids;
	});

	// Mutations
	const updateServiceMutation = useUpdateServiceMutation();
	const deleteServiceMutation = useDeleteServiceMutation();
	const bulkDeleteServicesMutation = useBulkDeleteServicesMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let servicesData = $derived(servicesQuery.data?.items ?? []);
	let allHostsData = $derived(allHostsQuery.data?.items ?? []);
	let allServicesData = $derived(servicesCacheQuery.data ?? []);
	let servicesPagination = $derived(servicesQuery.data?.pagination ?? null);
	let hostsData = $derived(hostsQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);
	let portsData = $derived(portsQuery.data ?? []);
	let ipAddressesData = $derived(ipAddressesQuery.data ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);
	// Only show full loading on initial load (no data yet)
	let isInitialLoading = $derived(servicesQuery.isPending && !servicesQuery.data);

	function isContainerSubnetFn(subnetId: string): boolean {
		const subnet = subnetsData.find((s) => s.id === subnetId);
		return subnet ? isContainerSubnet(subnet) : false;
	}

	/**
	 * A service's port bindings, grouped by the interface they are bound to.
	 *
	 * Keyed on the binding's `ip_address_id` rather than the resolved interface:
	 * if the lookup fails (an SCD2-closed address the live query no longer
	 * returns) two distinct bindings would otherwise collapse to the same
	 * 'unbound' literal and trip Svelte's each_key_duplicate.
	 */
	function portBindingItems(service: Service): LabelledCardFieldItem[] {
		const grouped = new SvelteMap<string | null, { iface: IPAddress | null; ports: Port[] }>();

		for (const binding of service.bindings.filter((b) => b.type === 'Port')) {
			const port = portsData.find((p) => p.id === binding.port_id);
			if (!port) continue;

			const interfaceId = binding.ip_address_id ?? null;
			if (!grouped.has(interfaceId)) {
				const iface = interfaceId
					? (ipAddressesData.find((i) => i.id === interfaceId) ?? null)
					: null;
				grouped.set(interfaceId, { iface, ports: [] });
			}
			grouped.get(interfaceId)!.ports.push(port);
		}

		return [...grouped.entries()].map(([interfaceId, { iface, ports: bound }]) => {
			const portList = bound.map((p) => formatPort(p)).join(', ');
			return {
				id: interfaceId ?? 'unbound',
				label: iface
					? `${iface.name ? iface.name + ': ' : ''} ${iface.ip_address} (${portList})`
					: `${common_unbound()} (${portList})`,
				color: entities.getColorHelper('Port').color
			};
		});
	}

	/** Interfaces a service is bound to directly, rather than through a port. */
	function ipBindingItems(service: Service): LabelledCardFieldItem[] {
		return service.bindings
			.filter((b) => b.type === 'IPAddress')
			.map((b) => b.ip_address_id)
			.filter((id): id is string => id !== null)
			.map((id) => ipAddressesData.find((i) => i.id === id))
			.filter((iface): iface is IPAddress => iface !== undefined)
			.map((iface) => ({
				id: iface.id,
				label: formatIPAddress(iface, isContainerSubnetFn),
				color: entities.getColorHelper('IPAddress').color,
				entityRef: entityRef('IPAddress', iface.id, iface, { subnets: subnetsData })
			}));
	}

	// Page change handler for server-side pagination
	function handlePageChange(page: number, newPageSize: number) {
		currentPage = page;
		pageSize = newPageSize;
	}

	// Order change handler for server-side ordering
	// Values are now directly ServiceOrderField values from the orderField property
	function handleOrderChange(
		groupField: string | null,
		orderField: string | null,
		direction: 'asc' | 'desc'
	) {
		groupBy = (groupField as ServiceOrderField) ?? undefined;
		orderBy = (orderField as ServiceOrderField) ?? undefined;
		orderDirection = direction;
	}

	// Tag filter change handler for server-side filtering
	function handleTagFilterChange(selectedTagIds: string[]) {
		tagIds = selectedTagIds;
		// Reset to page 1 is handled by DataControls
	}

	/**
	 * Server-side field filter handler.
	 *
	 * The panel offers what the user reads — a host's title, a network's name —
	 * while the API filters on ids, so each case resolves the labels back
	 * through the same data the options were built from. Every key here must
	 * match a field marked `serverFiltered`; an unhandled one would filter
	 * nothing at all, since DataControls skips the client pass for those fields.
	 */
	function handleFilterChange(fieldKey: string, values: string[]) {
		switch (fieldKey) {
			case 'category':
				excludeCategories = values;
				break;
			case 'port':
				ports = values.map(Number).filter((port) => Number.isFinite(port));
				break;
			case 'host': {
				const wanted = new Set(values);
				filterHostIds = allHostsData
					.filter((host) => wanted.has(hostDisplayName(host)))
					.map((host) => host.id);
				break;
			}
			case 'network_id': {
				const wanted = new Set(values);
				filterNetworkIds = networksData
					.filter((network) => wanted.has(network.name))
					.map((network) => network.id);
				break;
			}
			case 'service_definition': {
				// The options render friendly names; the server filters on the raw
				// definition id, which is what `getGroupValue` already exposes.
				const wanted = new Set(values);
				filterServiceDefinitions = serviceDefinitions
					.getItems()
					.filter((definition) => definition.name !== null && wanted.has(definition.name))
					.map((definition) => definition.id);
				break;
			}
			case 'containerized_by': {
				// "Not Containerized" is a choice about absence, so it is carried
				// as its own flag rather than as an id nothing would match.
				filterIncludeUncontainerized = values.includes(services_notContainerized());
				const wanted = new Set(values.filter((v) => v !== services_notContainerized()));
				filterVirtualizationServiceIds = allServicesData
					.filter((service) => wanted.has(service.name))
					.map((service) => service.id);
				break;
			}
			default:
				throw new Error(
					`ServiceTab: no server-side filter handles "${fieldKey}". A serverFiltered field ` +
						`without a case here filters nothing — neither the client nor the server.`
				);
		}
	}

	function handleStaleFilterChange(next: boolean | null) {
		stale = next;
	}

	// Search change handler for server-side search (debounced by DataControls)
	function handleSearchChange(query: string) {
		search = query;
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('Service', {
			tag_ids: tagIds.length > 0 ? tagIds : undefined,
			order_by: orderBy,
			order_direction: orderDirection
		});
	}

	let showServiceEditor = $state(false);
	let editingService = $state<Service | null>(null);

	// Deep-link: open service editor from URL (edit-only, validates host exists)
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'service-editor',
			servicesData,
			showServiceEditor,
			editingService?.id,
			(s) => !!serviceHosts.get(s.id)
		);
		if (result) {
			editingService = result;
			showServiceEditor = true;
		}
	});

	/** Row actions for table mode, matching what the card offers. */
	function serviceActions(service: Service): CardAction[] {
		if (isReadOnly) return [];

		return [
			{ label: common_edit(), icon: Edit, onClick: () => handleEditService(service) },
			{
				label: common_delete(),
				icon: Trash2,
				class: 'btn-icon-danger',
				onClick: () => handleDeleteService(service)
			}
		];
	}

	function handleEditService(service: Service) {
		editingService = service;
		showServiceEditor = true;
	}
	function handleCloseServiceEditor() {
		// Clear URL modal state before setting local state to prevent the deep-link
		// effect from re-opening the modal (parent effects run before child effects,
		// so the deep-link effect would see showServiceEditor=false but $modalState
		// still pointing to this service)
		closeModal();
		showServiceEditor = false;
		editingService = null;
	}

	let serviceHosts = $derived(
		(() => {
			const map = new SvelteMap(
				servicesData.map((service) => {
					const foundHost = hostsData.find((h) => h.id == service.host_id);
					return [service.id, foundHost] as [string, (typeof hostsData)[0] | undefined];
				})
			);
			// Include entityData service (EntityTag navigation)
			const ms = $modalState;
			if (ms.name === 'service-editor' && ms.entityData?.id && ms.entityData?.host_id) {
				if (!map.has(ms.entityData.id as string)) {
					const host = hostsData.find((h) => h.id === ms.entityData!.host_id);
					if (host) map.set(ms.entityData.id as string, host);
				}
			}
			return map;
		})()
	);

	function handleDeleteService(service: Service) {
		if (confirm(common_confirmDeleteName({ name: service.name }))) {
			deleteServiceMutation.mutate(service.id);
		}
	}

	async function handleServiceUpdate(id: string, data: Service) {
		try {
			await updateServiceMutation.mutateAsync(data);
			showServiceEditor = false;
			editingService = null;
		} catch {
			// Error handled by mutation
		}
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(common_confirmBulkDelete({ count: ids.length, entity: common_services() }))) {
			await bulkDeleteServicesMutation.mutateAsync(ids);
		}
	}

	function getServiceTags(service: Service): string[] {
		return service.tags;
	}

	// Derive available service categories from metadata
	let serviceCategories = $derived.by(() => {
		const items = serviceDefinitions.getItems() || [];
		const categoriesSet = new Set(items.map((i) => serviceDefinitions.getCategory(i.id)));
		return Array.from(categoriesSet)
			.filter((c) => c)
			.sort();
	});

	// Define field configuration for the DataTableControls
	// Uses defineFields to ensure all ServiceOrderField values are covered
	let serviceFields = $derived(
		defineFields<Service, ServiceOrderField>(
			{
				// Identity field: grouping by it would render a header per service.
				name: {
					label: common_name(),
					type: 'string',
					searchable: true,
					groupable: false,
					display: { order: 0, primary: true, width: 220 }
				},
				host: {
					label: common_host(),
					type: 'string',
					searchable: true,
					filterable: true,
					serverFiltered: true,
					// From the full host list, not the loaded page's hosts.
					filterOptions: allHostsData.map(hostDisplayName),
					groupable: true,
					// The server groups on the host's title — the same full ladder rendered below —
					// coalescing services with no host to an empty string. Both sides have to walk
					// every rung or two hosts the server lumps into one group get two alternating
					// headers drawn over their interleaved rows.
					getGroupValue: (service) => {
						const host = serviceHosts.get(service.id);
						return host ? hostDisplayName(host) : '';
					},
					getValue: (service) => {
						const host = serviceHosts.get(service.id);
						return host ? hostDisplayName(host) : common_unknownEntity({ entity: common_host() });
					},
					display: {
						order: 3,
						getItems: (service) => {
							const host = serviceHosts.get(service.id);
							if (!host) return [];
							return [
								{
									id: host.id,
									label: hostDisplayName(host),
									color: entities.getColorHelper('Host').color,
									entityRef: entityRef('Host', host.id, host)
								}
							];
						}
					}
				},
				network_id: {
					label: common_network(),
					type: 'string',
					searchable: true,
					filterable: true,
					serverFiltered: true,
					filterOptions: networksData.map((n) => n.name),
					groupable: true,
					// Displayed as a name, but grouped by id on the server.
					getGroupValue: (item) => item.network_id,
					getValue: (item) =>
						networksData.find((n) => n.id == item.network_id)?.name || common_unknownNetwork(),
					display: { order: 2, getItems: (item) => networkItems(item.network_id, networksData) }
				},
				// Per-service ordinal, so grouping by it is one header per service.
				position: {
					label: common_position(),
					type: 'string',
					groupable: false,
					display: { hiddenByDefault: true, align: 'right' }
				},
				service_definition: {
					label: common_type(),
					type: 'string',
					searchable: true,
					filterable: true,
					serverFiltered: true,
					// Every definition the registry knows, not just those on this page.
					filterOptions: serviceDefinitions
						.getItems()
						.map((definition) => definition.name)
						.filter((name): name is string => name !== null),
					groupable: true,
					// The server groups on the raw definition id; the UI renders its
					// friendly name, so the group key has to be supplied separately.
					getGroupValue: (service) => service.service_definition,
					getValue: (service) => serviceDefinitions.getName(service.service_definition),
					display: {
						order: 4,
						getItems: (service) => [
							{
								id: service.service_definition,
								label: serviceDefinitions.getName(service.service_definition),
								color: serviceDefinitions.getColorHelper(service.service_definition).color,
								icon: serviceDefinitions.getIconComponent(service.service_definition)
							}
						]
					}
				},
				created_at: { label: common_created(), type: 'date', display: { hiddenByDefault: true } },
				updated_at: { label: common_updated(), type: 'date', display: { hiddenByDefault: true } },
				// Staleness rides on the date rather than a Status column of its own: a
				// service has no status, and `getFreshnessTag` returns a tag only when
				// the row is past its network's window — so the column was empty on
				// every healthy service. `getItems` returning undefined falls back to
				// the date.
				last_seen_at: {
					label: common_lastSeen(),
					type: 'date',
					display: { order: 1, getItems: lastSeenItems(() => networksData, 'Service') }
				}
			},
			[
				{
					key: 'port_bindings',
					label: common_portBindings(),
					type: 'array',
					searchable: true,
					getValue: (service) => portBindingItems(service).map((b) => b.label),
					display: { hiddenByDefault: true, getItems: portBindingItems }
				},
				{
					key: 'ip_bindings',
					label: common_ipAddressBindings(),
					type: 'array',
					searchable: true,
					getValue: (service) => ipBindingItems(service).map((b) => b.label),
					display: { hiddenByDefault: true, getItems: ipBindingItems }
				},
				{
					key: 'category',
					label: common_category(),
					type: 'string',
					searchable: true,
					filterable: true,
					serverFiltered: true,
					filterMode: 'exclude',
					filterOptions: serviceCategories,
					filterDefaults: ['OpenPorts'],
					getValue: (item) =>
						serviceDefinitions.getCategory(item.service_definition) || common_unknown(),
					display: {
						order: 5,
						getItems: (item) => {
							const category = serviceDefinitions.getCategory(item.service_definition);
							if (!category) return [];
							// Categories carry their own colour in the metadata fixture, so
							// use it rather than rendering every category identically grey.
							return [
								{
									id: category,
									label: serviceCategoryMeta.getName(category) || category,
									color: serviceCategoryMeta.getColorHelper(category).color,
									icon: serviceCategoryMeta.getIconComponent(category)
								}
							];
						}
					}
				},
				{
					key: 'containerized_by',
					type: 'string',
					label: common_containerized(),
					searchable: true,
					filterable: true,
					serverFiltered: true,
					filterOptions: [
						...new Set(allServicesData.map((s) => s.name).filter((name) => name.length > 0))
					]
						.sort((a, b) => a.localeCompare(b))
						.concat(services_notContainerized()),
					getValue: (item) =>
						servicesData.find((s) => s.id == item.virtualization_service_id)?.name ||
						services_notContainerized(),
					display: {
						hiddenByDefault: true,
						// No chip when a service isn't containerized, so the cell shows an
						// em dash rather than repeating the phrase down the column. The
						// phrase stays in `getValue`, so the filter still offers it.
						getItems: (item) => {
							const runtime = servicesData.find((s) => s.id == item.virtualization_service_id);
							if (!runtime) return [];
							return [
								{
									id: runtime.id,
									label: runtime.name,
									color: concepts.getColorHelper('Containerization').color,
									entityRef: entityRef('Service', runtime.id, runtime)
								}
							];
						}
					}
				},
				{
					key: 'confidence',
					label: services_matchConfidence(),
					type: 'string',
					searchable: true,
					// Deliberately not filterable. The value is derived from the
					// `source` JSONB through `matchConfidenceLabel`, a mapping that
					// exists only in TypeScript, so the server cannot filter on it —
					// and this list is server-paginated, where a client-side filter
					// would narrow the loaded page while the count kept describing
					// every match. Restoring the filter means moving the label
					// mapping to the backend (TypeMetadataProvider + fixture) first.
					display: { hiddenByDefault: true },
					getValue: (item) =>
						item.source.type == 'DiscoveryWithMatch'
							? matchConfidenceLabel(item.source.details.confidence)
							: services_notDiscovered()
				},
				{
					key: 'port',
					label: common_port(),
					type: 'string',
					filterable: true,
					serverFiltered: true,
					filterOptions: wellKnownPortNumbers,
					// Drives the port filter only: it has no `getValue`, so as a column
					// it would render an empty cell on every row.
					display: { hidden: true }
				},
				{
					key: 'tags',
					label: common_tags(),
					type: 'array',
					searchable: true,
					filterable: true,
					getValue: (entity) => tagNames(entity.tags, tagsData)
				}
			]
		)
	);
</script>

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={common_services()} subtitle={services_subtitle()} />

	{#if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title={daemons_installPromptServices()} />
	{:else if isInitialLoading}
		<!-- Loading state (only on initial load) -->
		<Loading />
	{:else if servicesData.length === 0 && !servicesPagination}
		<!-- Empty state -->
		<EmptyState title={common_noEntityYet({ entity: common_services() })} subtitle="" />
	{:else}
		<DataControls
			items={servicesData}
			fields={serviceFields}
			storageKey="scanopy-services-table-state"
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			entityType={isReadOnly ? undefined : 'Service'}
			getItemTags={getServiceTags}
			getItemId={(item) => item.id}
			getIcon={(service) => ({
				icon: serviceDefinitions.getIconComponent(service.service_definition),
				color: serviceDefinitions.getColorHelper(service.service_definition).icon
			})}
			serverPagination={servicesPagination}
			onPageChange={handlePageChange}
			onOrderChange={handleOrderChange}
			onTagFilterChange={handleTagFilterChange}
			onFilterChange={handleFilterChange}
			onStaleFilterChange={handleStaleFilterChange}
			onSearchChange={handleSearchChange}
			onCsvExport={handleCsvExport}
			getActions={serviceActions}
			entityLabel={common_services()}
		></DataControls>
	{/if}
</div>

{#if editingService}
	{@const editingServiceHost = serviceHosts.get(editingService.id)}
	{#if editingServiceHost}
		<ServiceEditModal
			name="service-editor"
			service={editingService}
			host={editingServiceHost}
			isOpen={showServiceEditor}
			onUpdate={handleServiceUpdate}
			onClose={handleCloseServiceEditor}
			onDelete={() => {
				handleDeleteService(editingService!);
				handleCloseServiceEditor();
			}}
		/>
	{/if}
{/if}
