<script lang="ts">
	import { entities, discoveryTypes } from '$lib/shared/stores/metadata';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { Discovery } from '../../types/base';
	import DiscoveryEditModal from '../DiscoveryModal/DiscoveryEditModal.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import { formatDuration, formatTimestamp } from '$lib/shared/utils/formatting';
	import { defineFields } from '$lib/shared/components/data/types';
	import {
		useDiscoveryHistoryQuery,
		useCreateDiscoveryMutation,
		useUpdateDiscoveryMutation,
		useBulkDeleteDiscoveriesMutation,
		type DiscoveryHistoryQueryParams
	} from '../../queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';
	import type { components } from '$lib/api/schema';
	import type { TabProps } from '$lib/shared/types';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { modalState, openModal, closeModal } from '$lib/shared/stores/modal-registry';
	import { Info } from 'lucide-svelte';
	import { daemonItems } from '$lib/features/daemons/columns';
	import { networkItems } from '$lib/features/networks/columns';
	import { toColor } from '$lib/shared/utils/styling';
	import type { CardAction } from '$lib/shared/components/data/types';
	import { runOutcomeTag, type OutcomeTag } from '../../utils/outcome';
	import {
		common_created,
		common_daemon,
		common_details,
		common_duration,
		common_name,
		common_status,
		common_warnings,
		common_network,
		common_type,
		common_unknown,
		common_unknownEntity,
		common_unknownNetwork,
		common_updated,
		daemons_installPromptDiscoveries,
		discovery_confirmDeleteHistorical,
		discovery_finishedAt,
		discovery_historyTitle,
		discovery_noHistorySessions,
		discovery_noHistorySessionsSubtitle,
		discovery_startedAt
	} from '$lib/paraglide/messages';

	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];
	type DiscoveryOrderField = components['schemas']['DiscoveryOrderField'];
	type OrderDirection = components['schemas']['OrderDirection'];

	let { isReadOnly = false, isActive = false }: TabProps = $props();

	// Organization query for onboarding state
	const organizationQuery = useOrganizationQuery();
	let onboarding = $derived((organizationQuery.data?.onboarding ?? []) as OnboardingOperation[]);

	// Pagination state (managed by DataControls, updated via callback)
	let pageSize = $state(20);
	let currentPage = $state(1);

	// Ordering state (server-side)
	let groupBy = $state<DiscoveryOrderField | undefined>(undefined);
	let orderBy = $state<DiscoveryOrderField | undefined>(undefined);
	let orderDirection = $state<OrderDirection>('asc');

	// Search state (server-side: the run history is paginated, so a client-side
	// search would only ever match the page in hand)
	let search = $state('');

	// Field filter state, server-side for the same reason.
	let filterNetworkIds = $state<string[]>([]);
	let filterDaemonIds = $state<string[]>([]);
	let filterDiscoveryTypes = $state<string[]>([]);
	let hasServerFilters = $derived(
		filterNetworkIds.length > 0 || filterDaemonIds.length > 0 || filterDiscoveryTypes.length > 0
	);

	// Queries
	const discoveriesQuery = useDiscoveryHistoryQuery(
		(): DiscoveryHistoryQueryParams => ({
			limit: pageSize,
			offset: (currentPage - 1) * pageSize,
			group_by: groupBy,
			order_by: orderBy,
			order_direction: orderDirection,
			search: search || undefined,
			network_ids: filterNetworkIds.length > 0 ? filterNetworkIds : undefined,
			daemon_ids: filterDaemonIds.length > 0 ? filterDaemonIds : undefined,
			discovery_types: filterDiscoveryTypes.length > 0 ? filterDiscoveryTypes : undefined
		}),
		() => isActive
	);
	const daemonsQuery = useDaemonsQuery();
	const networksQuery = useNetworksQuery();

	// Mutations
	const createDiscoveryMutation = useCreateDiscoveryMutation();
	const updateDiscoveryMutation = useUpdateDiscoveryMutation();
	const bulkDeleteDiscoveriesMutation = useBulkDeleteDiscoveriesMutation();

	// Derived data
	let discoveriesData = $derived(discoveriesQuery.data?.items ?? []);
	let discoveriesPagination = $derived(discoveriesQuery.data?.pagination ?? null);
	let daemonsData = $derived(daemonsQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);

	// Only the hosts the daemons run on. This was an unpaginated org-wide hosts
	// query (~1.9MB on a few hundred hosts), issued so the edit modal's daemon
	// picker could label each daemon with its host name — and because TanStack
	// dedupes by key, it was shared with every other consumer, so it loaded on
	// pages that never opened the modal. Scoped to the ids in hand.
	let daemonHostIds = $derived([
		...new Set(daemonsData.map((d) => d.host_id).filter((id): id is string => !!id))
	]);
	const hostsQuery = useHostsByIds(() => daemonHostIds);
	let hostsData = $derived(hostsQuery.data ?? []);

	// Host names are decoration inside the modal, so the list must not block on
	// them — and with no daemons the by-ids query is disabled, which in TanStack
	// means it stays `isPending` forever. The server already returns only
	// historical rows (`historical: true`), so no client-side filter is needed.
	let isLoading = $derived(discoveriesQuery.isPending || daemonsQuery.isPending);

	// Page change handler for server-side pagination
	function handlePageChange(page: number, newPageSize: number) {
		currentPage = page;
		pageSize = newPageSize;
	}

	// Order change handler for server-side ordering
	function handleOrderChange(
		groupField: string | null,
		orderField: string | null,
		direction: 'asc' | 'desc'
	) {
		groupBy = (groupField as DiscoveryOrderField) ?? undefined;
		orderBy = (orderField as DiscoveryOrderField) ?? undefined;
		orderDirection = direction;
	}

	// Search change handler for server-side search (debounced by DataControls)
	function handleSearchChange(query: string) {
		search = query;
	}

	/**
	 * Server-side field filter handler.
	 *
	 * The panel offers what the user reads — a daemon's name, a network's name —
	 * while the API filters on ids, so each case resolves the labels back
	 * through the same data the options were built from. Every key here must
	 * match a field marked `serverFiltered`; an unhandled one would filter
	 * nothing at all, since DataControls skips the client pass for those fields.
	 */
	function handleFilterChange(fieldKey: string, values: string[]) {
		const wanted = new Set(values);
		switch (fieldKey) {
			case 'daemon_id':
				filterDaemonIds = daemonsData.filter((d) => wanted.has(d.name)).map((d) => d.id);
				break;
			case 'network_id':
				filterNetworkIds = networksData.filter((n) => wanted.has(n.name)).map((n) => n.id);
				break;
			case 'discovery_type':
				// Already the raw discriminant the server stores, so it needs no
				// resolution — the column renders the tag itself.
				filterDiscoveryTypes = values;
				break;
			default:
				throw new Error(
					`DiscoveryHistoryTab: no server-side filter handles "${fieldKey}". A serverFiltered ` +
						`field without a case here filters nothing — neither the client nor the server.`
				);
		}
	}

	let showDiscoveryModal = $state(false);
	let editingDiscovery: Discovery | null = $state(null);

	// Deep-link: open detail modal from URL
	$effect(() => {
		if ($modalState.name === 'discovery-history-detail' && !showDiscoveryModal) {
			if ($modalState.id) {
				const disc = discoveriesData.find((d) => d.id === $modalState.id);
				if (disc) {
					editingDiscovery = disc;
					showDiscoveryModal = true;
				}
			}
		}
	});

	function handleEditDiscovery(discovery: Discovery) {
		editingDiscovery = discovery;
		showDiscoveryModal = true;
		openModal('discovery-history-detail', { id: discovery.id });
	}

	async function handleDiscoveryCreate(data: Discovery) {
		await createDiscoveryMutation.mutateAsync(data);
		showDiscoveryModal = false;
		editingDiscovery = null;
	}

	async function handleDiscoveryUpdate(id: string, data: Discovery) {
		await updateDiscoveryMutation.mutateAsync(data);
		showDiscoveryModal = false;
		editingDiscovery = null;
	}

	function handleCloseEditor() {
		showDiscoveryModal = false;
		editingDiscovery = null;
		closeModal();
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(discovery_confirmDeleteHistorical({ count: ids.length }))) {
			await bulkDeleteDiscoveriesMutation.mutateAsync(ids);
		}
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('Discovery', {});
	}

	/**
	 * How a run ended, as a tag. A clean completion returns null and the column renders empty.
	 * Warnings have a column of their own: a tag here said only that the run had some, which made
	 * a single informational note look like a broken credential.
	 */
	function outcomeTag(discovery: Discovery): OutcomeTag | null {
		return runOutcomeTag(
			discovery.run_type.type === 'Historical' ? discovery.run_type.results : null
		);
	}

	/** A run's recorded warnings, or none for a row that is not a completed run. */
	function warningsOf(discovery: Discovery) {
		return discovery.run_type.type === 'Historical'
			? (discovery.run_type.results.warnings ?? [])
			: [];
	}

	/** Row actions for table mode, matching what the card offers. */
	function discoveryActions(discovery: Discovery): CardAction[] {
		return [{ label: common_details(), icon: Info, onClick: () => handleEditDiscovery(discovery) }];
	}

	let fields = $derived(
		defineFields<Discovery, DiscoveryOrderField>(
			{
				// Identity field: grouping by it would render a header per run.
				name: {
					label: common_name(),
					type: 'string',
					searchable: true,
					groupable: false,
					display: { order: 0 }
				},
				daemon_id: {
					label: common_daemon(),
					type: 'string',
					searchable: true,
					filterable: true,
					serverFiltered: true,
					filterOptions: daemonsData.map((d) => d.name),
					groupable: true,
					// Displayed as a name, but grouped by id on the server.
					getGroupValue: (item) => item.daemon_id,
					getValue: (item) =>
						daemonsData.find((d) => d.id === item.daemon_id)?.name ??
						common_unknownEntity({ entity: common_daemon() }),
					display: { order: 7, getItems: (item) => daemonItems(item.daemon_id, daemonsData) }
				},
				network_id: {
					label: common_network(),
					type: 'string',
					searchable: true,
					filterable: true,
					serverFiltered: true,
					filterOptions: networksData.map((n) => n.name),
					groupable: true,
					getGroupValue: (item) => item.network_id,
					getValue: (item) =>
						networksData.find((n) => n.id === item.network_id)?.name ?? common_unknownNetwork(),
					display: { order: 6, getItems: (item) => networkItems(item.network_id, networksData) }
				},
				discovery_type: {
					label: common_type(),
					type: 'string',
					searchable: true,
					filterable: true,
					serverFiltered: true,
					filterOptions: discoveryTypes.getItems().map((type) => type.id),
					groupable: true,
					getValue: (item) => item.discovery_type.type,
					display: { hiddenByDefault: true }
				},
				created_at: { label: common_created(), type: 'date', display: { hiddenByDefault: true } },
				updated_at: { label: common_updated(), type: 'date', display: { hiddenByDefault: true } }
			},
			[
				{
					// The run's outcome, which the card shows as its header tag. It was
					// card-only, so a failed or cancelled run looked identical to a
					// clean one in the table.
					key: 'outcome',
					label: common_status(),
					type: 'string',
					searchable: true,
					// Deliberately not filterable. The value is derived from the
					// `run_type` JSONB (phase and terminal reason) through `outcomeTag`,
					// and the server has no column to filter on — this list is
					// server-paginated, where a client-side filter would narrow the
					// loaded page while the count kept describing every match.
					groupable: true,
					getValue: (item) => outcomeTag(item)?.label ?? '',
					display: {
						order: 1,
						statusTag: true,
						getItems: (item) => {
							const tag = outcomeTag(item);
							return tag
								? [{ id: tag.label, label: tag.label, color: tag.color, title: tag.title }]
								: [];
						}
					}
				},
				// Derived from the run's JSONB results, so these are display-only:
				// there is no column to sort or group on.
				{
					key: 'started_at',
					label: discovery_startedAt(),
					type: 'string',
					getValue: (item) => {
						const results = item.run_type.type == 'Historical' ? item.run_type.results : null;
						return results && results.started_at
							? formatTimestamp(results.started_at)
							: common_unknown();
					},
					display: { order: 3 }
				},
				{
					key: 'finished_at',
					label: discovery_finishedAt(),
					type: 'string',
					getValue: (item) => {
						const results = item.run_type.type == 'Historical' ? item.run_type.results : null;
						return results && results.finished_at
							? formatTimestamp(results.finished_at)
							: common_unknown();
					},
					display: { order: 4 }
				},
				{
					key: 'duration',
					label: common_duration(),
					type: 'string',
					getValue: (item) => {
						const results = item.run_type.type == 'Historical' ? item.run_type.results : null;
						if (results && results.finished_at && results.started_at) {
							return formatDuration(results.started_at, results.finished_at);
						}
						return common_unknown();
					},
					display: { order: 5 }
				},
				{
					// How much the run had to say, next to how it ended — the two questions asked
					// together. Always amber, never red: this column counts warnings, and a run
					// that failed outright says so in Status. A clean run falls back to its plain
					// 0 rather than wearing a chip that means nothing.
					key: 'warnings',
					label: common_warnings(),
					type: 'string',
					getValue: (item) => String(warningsOf(item).length),
					display: {
						order: 2,
						getItems: (item) => {
							const count = warningsOf(item).length;
							if (count === 0) return undefined;
							return [{ id: 'warnings', label: String(count), color: toColor('amber') }];
						}
					}
				}
			]
		)
	);
</script>

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={discovery_historyTitle()} />

	{#if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title={daemons_installPromptDiscoveries()} />
	{:else if isLoading}
		<Loading />
	{:else if discoveriesData.length === 0 && !search && !hasServerFilters}
		<!--
			"No sessions yet" only when nothing is narrowing the list. The search and the
			field filters run server-side and are restored from storage on mount, so an
			empty response under one means "no matches" — and this branch would take the
			controls away with it, re-arming the same no-match query on every reload.
			DataControls renders the filtered-empty state instead.
		-->
		<EmptyState
			title={discovery_noHistorySessions()}
			subtitle={discovery_noHistorySessionsSubtitle()}
		/>
	{:else}
		<DataControls
			items={discoveriesData}
			{fields}
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			storageKey="scanopy-discovery-historical-table-state"
			getItemId={(item) => item.id}
			getIcon={() => ({
				icon: entities.getIconComponent('Discovery'),
				color: entities.getColorHelper('Discovery').icon
			})}
			serverPagination={discoveriesPagination}
			onPageChange={handlePageChange}
			onOrderChange={handleOrderChange}
			onSearchChange={handleSearchChange}
			onFilterChange={handleFilterChange}
			onCsvExport={handleCsvExport}
			getActions={discoveryActions}
			entityLabel={discovery_historyTitle()}
		></DataControls>
	{/if}
</div>

<DiscoveryEditModal
	name="discovery-history-detail"
	isOpen={showDiscoveryModal}
	hosts={hostsData}
	daemons={daemonsData}
	discovery={editingDiscovery}
	onCreate={handleDiscoveryCreate}
	onUpdate={handleDiscoveryUpdate}
	onClose={handleCloseEditor}
/>
