<script lang="ts">
	import { rowStatusBadges } from '$lib/shared/utils/row-badges';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import LoadFailedState from '$lib/shared/components/layout/LoadFailedState.svelte';
	import { loadFailure } from '$lib/shared/utils/load-failure';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { TableDefaults } from '$lib/shared/components/data/types';
	import {
		entityRef,
		type CardAction,
		type LabelledCardFieldItem
	} from '$lib/shared/components/data/types';
	import { usePortsQuery } from '$lib/features/ports/queries';
	import { virtualizedSummary } from '$lib/features/services/virtualized-count';
	import { useIPAddressesQuery } from '$lib/features/ip-addresses/queries';
	import { useSubnetsQuery, isContainerSubnet } from '$lib/features/subnets/queries';
	import { formatIPAddress } from '$lib/features/hosts/address-labels';
	import { formatPort } from '$lib/shared/utils/formatting';
	import type { IPAddress, Port } from '$lib/features/hosts/types/base';
	import { entities } from '$lib/shared/stores/metadata';
	import { Trash2, Edit } from '@lucide/svelte';
	import type { Service } from '../types/base';
	import ServiceEditModal from './ServiceEditModal.svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import {
		useServicesQuery,
		useUpdateServiceMutation,
		useDeleteServiceMutation,
		useBulkDeleteServicesMutation,
		type ServicesQueryParams,
		useServicesByIds
	} from '../queries';
	import { useDiscoveriesByIds } from '$lib/features/discovery/queries';
	import { discoveryRunIds } from '$lib/features/discovery/columns';
	import { useHostsByIds, useHostSummariesQuery } from '$lib/features/hosts/queries';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';
	import { useSitesQuery } from '$lib/features/sites/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import type { TabProps } from '$lib/shared/types';
	import type { components } from '$lib/api/schema';
	import { serviceColumns } from './ServiceTab.columns';
	import { collectionLookup } from '$lib/shared/entity-fields';
	import { SvelteMap } from 'svelte/reactivity';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { closeModal, modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import {
		common_confirmBulkDelete,
		common_confirmBulkDeleteWithGuests,
		common_confirmDeleteName,
		common_confirmDeleteNameWithGuests,
		common_delete,
		common_edit,
		common_noEntityYet,
		common_services,
		common_unbound,
		daemons_installPromptServices,
		gettingStarted_stepDiscoveryLabel,
		services_subtitle
	} from '$lib/paraglide/messages';
	import { serviceDefinitions, ports as ports_metadata } from '$lib/shared/stores/metadata';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';
	import { fieldValueOptions, useFieldValuesQuery } from '$lib/shared/api/field-values';

	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];
	type ServiceOrderField = components['schemas']['ServiceOrderField'];

	const SERVICE_FIELD_VALUES = '/api/v1/services/field-values/{field}';

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

	import { refusedUnless, useCan } from '$lib/features/auth/access';

	let { isActive = false }: TabProps = $props();
	const can = useCan();

	// Organization query for onboarding state
	const organizationQuery = useOrganizationQuery();
	let onboarding = $derived((organizationQuery.data?.onboarding ?? []) as OnboardingOperation[]);

	// Pagination state (managed by DataControls, updated via callback)
	let pageSize = $state(20);
	let currentPage = $state(1);

	// The grouping and sort the table opens with until the user picks their own.
	const tableDefaults: TableDefaults<ServiceOrderField> = {
		sort: { field: 'name', direction: 'asc' }
	};

	// The list's view as the endpoint's params: search, tags, staleness, field filters and ordering,
	// all server-side because the client holds one page of services, so filtering here would narrow
	// that page while the total count kept describing every match. Null until DataControls has
	// restored the view. The query is not held for it: the table only mounts once a first page
	// has loaded.
	let listQuery = $state<URLSearchParams | null>(null);

	// Queries
	const tagsQuery = useTagsQuery();
	// Held until the tab is opened: every tab mounts at app load.
	const servicesQuery = useServicesQuery((): ServicesQueryParams => ({
		limit: pageSize,
		offset: (currentPage - 1) * pageSize,
		list: listQuery ?? undefined,
		enabled: isActive
	}));
	const sitesQuery = useSitesQuery();
	const portsQuery = usePortsQuery();
	// Option sources for the server-side filters. The lists below are the tab's
	// display data, scoped to the loaded page — filter options have to come from
	// the full sets or the panel only ever offers what is already on screen.
	const allHostsQuery = useHostSummariesQuery({});
	// Filter options: the values the caller's services actually hold, counted by the server.
	// None of these takes the tab's active filters, so the options do not shrink as the user
	// filters.
	const hostValuesQuery = useFieldValuesQuery(
		SERVICE_FIELD_VALUES,
		'host',
		undefined,
		() => isActive
	);
	const siteValuesQuery = useFieldValuesQuery(
		SERVICE_FIELD_VALUES,
		'site_id',
		undefined,
		() => isActive
	);
	const definitionValuesQuery = useFieldValuesQuery(
		SERVICE_FIELD_VALUES,
		'service_definition',
		undefined,
		() => isActive
	);
	const containerizedByValuesQuery = useFieldValuesQuery(
		SERVICE_FIELD_VALUES,
		'containerized_by',
		undefined,
		() => isActive
	);
	const matchConfidenceValuesQuery = useFieldValuesQuery(
		SERVICE_FIELD_VALUES,
		'match_confidence',
		undefined,
		() => isActive
	);
	const sourceValuesQuery = useFieldValuesQuery(
		SERVICE_FIELD_VALUES,
		'source',
		undefined,
		() => isActive
	);
	// The runtimes containerizing the visible services. Rarely on the same page as the services
	// they run, so fetched by id.
	const runtimesQuery = useServicesByIds(() => [
		...new Set(
			(servicesQuery.data?.items ?? [])
				.map((s) => s.virtualization_service_id)
				.filter((id): id is string => id != null)
		)
	]);
	const discoveryRunsQuery = useDiscoveriesByIds(() =>
		discoveryRunIds(servicesQuery.data?.items ?? [])
	);
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
	let runtimesData = $derived(runtimesQuery.data ?? []);
	let discoveryRunsData = $derived(discoveryRunsQuery.data ?? []);
	let servicesPagination = $derived(servicesQuery.data?.pagination ?? null);
	let hostsData = $derived(hostsQuery.data ?? []);
	let sitesData = $derived(sitesQuery.data ?? []);
	let portsData = $derived(portsQuery.data ?? []);
	let ipAddressesData = $derived(ipAddressesQuery.data ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);
	// Only show full loading on initial load (no data yet)
	let isInitialLoading = $derived(servicesQuery.isPending && !servicesQuery.data);
	let pageLoad = $derived(loadFailure(organizationQuery, servicesQuery));

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

	// The export endpoint takes the list's own query, so an export mirrors what the user was
	// looking at.
	async function handleCsvExport() {
		await downloadCsv('Service', listQuery ?? new URLSearchParams());
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

	/** Row actions. */
	function serviceActions(service: Service): CardAction[] {
		if (!can('Service', 'Write')) return [];

		return [
			{
				label: common_edit(),
				icon: Edit,
				isOpen: true,
				onClick: () => handleEditService(service),
				unavailable: refusedUnless(can('Service', 'Write', service.site_id))
			},
			{
				label: common_delete(),
				icon: Trash2,
				tone: 'danger',
				isDelete: true,
				onBulk: handleBulkDelete,
				onClick: () => handleDeleteService(service),
				unavailable: refusedUnless(can('Service', 'Delete', service.site_id))
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

	async function handleDeleteService(service: Service) {
		// Deleting a runtime or hypervisor deletes what it runs, so the question says how much.
		const guests = await virtualizedSummary([service]);
		const question = guests
			? common_confirmDeleteNameWithGuests({ name: service.name, guests })
			: common_confirmDeleteName({ name: service.name });
		if (confirm(question)) {
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
		const guests = await virtualizedSummary(servicesData.filter((s) => ids.includes(s.id)));
		const question = guests
			? common_confirmBulkDeleteWithGuests({ count: ids.length, entity: common_services(), guests })
			: common_confirmBulkDelete({ count: ids.length, entity: common_services() });
		if (confirm(question)) {
			await bulkDeleteServicesMutation.mutateAsync(ids);
		}
	}

	function getServiceTags(service: Service): string[] {
		return service.tags;
	}

	// The categories of the definitions some service matched. Category has no column of its own,
	// so it is read off the present definitions. A default exclusion whose category no service
	// holds stays selected and excludes nothing.
	let serviceCategories = $derived([
		...new Set(
			fieldValueOptions(definitionValuesQuery.data)
				.map((id) => serviceDefinitions.getCategory(id))
				.filter((category): category is string => !!category)
		)
	]);

	// The hosts some service runs on: the server counts their titles, the same ladder
	// `hostDisplayName` renders, and the full host list gives each title's ids.
	let hostOptions = $derived.by(() => {
		const titles = new Set(fieldValueOptions(hostValuesQuery.data));
		return allHostsData.filter((host) => titles.has(hostDisplayName(host))).map((host) => host.id);
	});

	function hostLabel(id: string): string {
		const host = allHostsData.find((h) => h.id === id);
		return host ? hostDisplayName(host) : id;
	}

	// Related entities the registry's Service fields read, from the collections this tab loads.
	let lookup = $derived(
		collectionLookup({
			Host: hostsData,
			Port: portsData,
			IPAddress: ipAddressesData,
			Service: runtimesData,
			Site: sitesData,
			Subnet: subnetsData
		})
	);

	let serviceFields = $derived(
		serviceColumns({
			lookup: () => lookup,
			serviceHosts: () => serviceHosts,
			sites: () => sitesData,
			tags: () => tagsData,
			discoveryRuns: () => discoveryRunsData,
			runtimes: () => runtimesData,
			hostOptions: () => hostOptions,
			hostLabel,
			siteValues: () => siteValuesQuery.data,
			definitionValues: () => definitionValuesQuery.data,
			containerizedByValues: () => containerizedByValuesQuery.data,
			matchConfidenceValues: () => matchConfidenceValuesQuery.data,
			sourceValues: () => sourceValuesQuery.data,
			wellKnownPortNumbers: () => wellKnownPortNumbers,
			serviceCategories: () => serviceCategories,
			portBindingItems,
			ipBindingItems
		})
	);
</script>

<div class="space-y-6">
	{#if pageLoad.failed}
		<LoadFailedState onRetry={pageLoad.retry} />
	{:else if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title={daemons_installPromptServices()} />
	{:else if isInitialLoading}
		<!-- Loading state (only on initial load) -->
		<Loading />
	{:else if servicesData.length === 0 && !servicesPagination}
		<!-- Empty state -->
		<EmptyState
			title={common_noEntityYet({ entity: common_services() })}
			subtitle=""
			onClick={can('Discovery', 'Write')
				? () => (window.location.hash = 'discovery-scans')
				: undefined}
			cta={can('Discovery', 'Write') ? gettingStarted_stepDiscoveryLabel() : ''}
		/>
	{:else}
		<DataControls
			title={common_services()}
			subtitle={services_subtitle()}
			items={servicesData}
			fields={serviceFields}
			storageKey="scanopy-services-table-state"
			defaults={tableDefaults}
			entityType="Service"
			canEditTags={can('Tag', 'Write')}
			getItemTags={getServiceTags}
			getItemId={(item) => item.id}
			serverPagination={servicesPagination}
			onPageChange={handlePageChange}
			onQueryChange={(query) => (listQuery = query)}
			onCsvExport={handleCsvExport}
			getActions={serviceActions}
			getRowBadges={rowStatusBadges(() => sitesData, 'Service')}
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
