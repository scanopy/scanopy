<script lang="ts">
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import { defineFields } from '$lib/shared/components/data/types';
	import type { Service } from '../types/base';
	import ServiceCard from './ServiceCard.svelte';
	import { matchConfidenceLabel } from '$lib/shared/types';
	import ServiceEditModal from './ServiceEditModal.svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import {
		useServicesQuery,
		useUpdateServiceMutation,
		useDeleteServiceMutation,
		useBulkDeleteServicesMutation,
		type ServicesQueryParams
	} from '../queries';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import type { TabProps } from '$lib/shared/types';
	import type { components } from '$lib/api/schema';
	import { SvelteMap } from 'svelte/reactivity';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import {
		common_services,
		services_confirmBulkDelete,
		services_confirmDelete,
		services_hiddenCategories,
		services_noServicesYet,
		services_subtitle
	} from '$lib/paraglide/messages';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';

	type OnboardingOperation = components['schemas']['OnboardingOperation'];
	type ServiceOrderField = components['schemas']['ServiceOrderField'];
	type OrderDirection = components['schemas']['OrderDirection'];

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

	// Queries
	const tagsQuery = useTagsQuery();
	// Paginated services with server-side pagination, ordering, and tag filtering
	const servicesQuery = useServicesQuery(
		(): ServicesQueryParams => ({
			limit: pageSize,
			offset: (currentPage - 1) * pageSize,
			group_by: groupBy,
			order_by: orderBy,
			order_direction: orderDirection,
			tag_ids: tagIds.length > 0 ? tagIds : undefined,
			exclude_categories:
				excludeCategories.length > 0
					? (excludeCategories as components['schemas']['ServiceCategory'][])
					: undefined
		})
	);
	const networksQuery = useNetworksQuery();

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
	let servicesPagination = $derived(servicesQuery.data?.pagination ?? null);
	let hostsData = $derived(hostsQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);
	// Only show full loading on initial load (no data yet)
	let isInitialLoading = $derived(servicesQuery.isPending && !servicesQuery.data);

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

	// Exclude filter change handler for server-side filtering
	function handleExcludeFilterChange(fieldKey: string, values: string[]) {
		if (fieldKey === 'category') {
			excludeCategories = values;
		}
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

	function handleEditService(service: Service) {
		editingService = service;
		showServiceEditor = true;
	}
	function handleCloseServiceEditor() {
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
		if (confirm(services_confirmDelete({ serviceName: service.name }))) {
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
		if (confirm(services_confirmBulkDelete({ count: ids.length }))) {
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
				name: { label: 'Name', type: 'string', searchable: true },
				host: {
					label: 'Host',
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					getValue: (service) => serviceHosts.get(service.id)?.name || 'Unknown Host'
				},
				network_id: {
					label: 'Network',
					type: 'string',
					filterable: true,
					groupable: true,
					getValue: (item) =>
						networksData.find((n) => n.id == item.network_id)?.name || 'Unknown Network'
				},
				position: { label: 'Position', type: 'string' },
				created_at: { label: 'Created', type: 'date' },
				updated_at: { label: 'Updated', type: 'date' }
			},
			[
				{
					key: 'category',
					label: services_hiddenCategories(),
					type: 'string',
					filterable: true,
					filterMode: 'exclude',
					filterOptions: serviceCategories,
					filterDefaults: ['OpenPorts'],
					getValue: (item) => serviceDefinitions.getCategory(item.service_definition) || 'Unknown'
				},
				{
					key: 'containerized_by',
					type: 'string',
					label: 'Containerized',
					searchable: true,
					filterable: true,
					getValue: (item) =>
						servicesData.find((s) => s.id == item.virtualization?.details.service_id)?.name ||
						'Not Containerized'
				},
				{
					key: 'confidence',
					label: 'Match Confidence',
					type: 'string',
					filterable: true,
					getValue: (item) =>
						item.source.type == 'DiscoveryWithMatch'
							? matchConfidenceLabel(item.source.details.confidence)
							: 'N/A (Not a discovered service)'
				},
				{
					key: 'tags',
					label: 'Tags',
					type: 'array',
					searchable: true,
					filterable: true,
					getValue: (entity) =>
						entity.tags
							.map((id) => tagsData.find((t) => t.id === id)?.name)
							.filter((name): name is string => !!name)
				}
			]
		)
	);
</script>

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={common_services()} subtitle={services_subtitle()} />

	{#if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title="Install a daemon to start discovering services on your network." />
	{:else if isInitialLoading}
		<!-- Loading state (only on initial load) -->
		<Loading />
	{:else if servicesData.length === 0 && !servicesPagination}
		<!-- Empty state -->
		<EmptyState title={services_noServicesYet()} subtitle="" />
	{:else}
		<DataControls
			items={servicesData}
			fields={serviceFields}
			storageKey="scanopy-services-table-state"
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			entityType={isReadOnly ? undefined : 'Service'}
			getItemTags={getServiceTags}
			getItemId={(item) => item.id}
			serverPagination={servicesPagination}
			onPageChange={handlePageChange}
			onOrderChange={handleOrderChange}
			onTagFilterChange={handleTagFilterChange}
			onExcludeFilterChange={handleExcludeFilterChange}
			onCsvExport={handleCsvExport}
		>
			{#snippet children(
				item: Service,
				viewMode: 'card' | 'list',
				isSelected: boolean,
				onSelectionChange: (selected: boolean) => void
			)}
				{@const host = serviceHosts.get(item.id)}
				{#if host}
					<ServiceCard
						service={item}
						selected={isSelected}
						{host}
						{onSelectionChange}
						{viewMode}
						onDelete={isReadOnly ? undefined : handleDeleteService}
						onEdit={isReadOnly ? undefined : handleEditService}
					/>
				{/if}
			{/snippet}
		</DataControls>
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
