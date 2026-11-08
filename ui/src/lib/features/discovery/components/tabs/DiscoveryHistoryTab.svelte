<script lang="ts">
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import { narrowsList } from '$lib/shared/components/data/controls/listQuery';
	import { listIsEmpty } from '$lib/api/list-page';
	import type { TableDefaults } from '$lib/shared/components/data/types';
	import type { Discovery } from '../../types/base';
	import DiscoveryEditModal from '../DiscoveryModal/DiscoveryEditModal.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import LoadFailedState from '$lib/shared/components/layout/LoadFailedState.svelte';
	import { loadFailure } from '$lib/shared/utils/load-failure';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import { discoveryHistoryColumns } from './DiscoveryHistoryTab.columns';
	import { collectionLookup } from '$lib/shared/entity-fields';
	import {
		useDiscoveryHistoryQuery,
		useCreateDiscoveryMutation,
		useUpdateDiscoveryMutation,
		useBulkDeleteDiscoveriesMutation,
		useDeleteDiscoveryMutation
	} from '../../queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useSitesQuery } from '$lib/features/sites/queries';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';
	import { useFieldValuesQuery } from '$lib/shared/api/field-values';
	import type { components } from '$lib/api/schema';
	import type { TabProps } from '$lib/shared/types';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import {
		modalState,
		openModal,
		closeModal,
		resolveModalDeepLink
	} from '$lib/shared/stores/modal-registry';
	import { Info, Trash2 } from '@lucide/svelte';
	import { useTagGroupsQuery, useTagsQuery } from '$lib/features/tags/queries';
	import type { RowAction } from '$lib/shared/components/data/types';
	import { runOutcomeTag, type OutcomeTag } from '../../utils/outcome';
	import {
		common_confirmDeleteName,
		common_delete,
		common_details,
		common_none,
		daemons_installPromptDiscoveries,
		discovery_confirmDeleteHistorical,
		discovery_historyTitle,
		discovery_noHistorySessions,
		discovery_noHistorySessionsSubtitle
	} from '$lib/paraglide/messages';

	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];
	type DiscoveryOrderField = components['schemas']['DiscoveryOrderField'];

	const DISCOVERY_FIELD_VALUES = '/api/v1/discoveries/field-values/{field}';

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
	const tableDefaults: TableDefaults<DiscoveryOrderField> = {
		sort: { field: 'started_at', direction: 'desc' }
	};

	// The list's view as the endpoint's params: search, field filters and ordering, server-side
	// because the run history is paginated, so filtering here would only narrow the page in hand.
	// Null until DataControls has restored the view.
	let listQuery = $state<URLSearchParams | null>(null);
	let isNarrowed = $derived(listQuery !== null && narrowsList(listQuery));

	// Queries
	const discoveriesQuery = useDiscoveryHistoryQuery(() => ({
		limit: pageSize,
		offset: (currentPage - 1) * pageSize,
		list: listQuery ?? undefined,
		// Every tab is mounted at once, so an ungated query would fetch on boot and on every
		// discovery event for a tab nobody is looking at.
		enabled: isActive
	}));
	// Filter options: the values the run history actually holds. Scoped to history rows the way
	// the list is, so scheduled configurations add none, and never narrowed by the active filters,
	// so the options do not shrink as the user filters. Gated like the list.
	const historyScope = () => ({ historical: true });
	const daemonValuesQuery = useFieldValuesQuery(
		DISCOVERY_FIELD_VALUES,
		'daemon_id',
		historyScope,
		() => isActive
	);
	const siteValuesQuery = useFieldValuesQuery(
		DISCOVERY_FIELD_VALUES,
		'site_id',
		historyScope,
		() => isActive
	);
	const typeValuesQuery = useFieldValuesQuery(
		DISCOVERY_FIELD_VALUES,
		'discovery_type',
		historyScope,
		() => isActive
	);
	const phaseValuesQuery = useFieldValuesQuery(
		DISCOVERY_FIELD_VALUES,
		'phase',
		historyScope,
		() => isActive
	);
	const tagsQuery = useTagsQuery();
	const tagGroupsQuery = useTagGroupsQuery();
	const daemonsQuery = useDaemonsQuery();
	const sitesQuery = useSitesQuery();

	// Mutations
	const createDiscoveryMutation = useCreateDiscoveryMutation();
	const updateDiscoveryMutation = useUpdateDiscoveryMutation();
	const bulkDeleteDiscoveriesMutation = useBulkDeleteDiscoveriesMutation();
	const deleteDiscoveryMutation = useDeleteDiscoveryMutation();

	// Derived data
	let discoveriesData = $derived(discoveriesQuery.data?.items ?? []);
	let discoveriesPagination = $derived(discoveriesQuery.data?.pagination ?? null);
	let tagsData = $derived(tagsQuery.data ?? []);
	let daemonsData = $derived(daemonsQuery.data ?? []);
	let sitesData = $derived(sitesQuery.data ?? []);

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
	let pageLoad = $derived(loadFailure(organizationQuery, discoveriesQuery, daemonsQuery));

	// Page change handler for server-side pagination
	function handlePageChange(page: number, newPageSize: number) {
		currentPage = page;
		pageSize = newPageSize;
	}

	let showDiscoveryModal = $state(false);
	let editingDiscovery: Discovery | null = $state(null);

	// Deep-link: open detail modal from URL (fresh open and entity switch). A run linked from
	// another tab (the scan that found a host) is rarely on this page, so the helper falls back to
	// the run the link carried.
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'discovery-history-detail',
			discoveriesData,
			showDiscoveryModal,
			editingDiscovery?.id
		);
		// No create mode here: a run without an id has nothing to show.
		if (result) {
			editingDiscovery = result;
			showDiscoveryModal = true;
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

	function handleDeleteDiscovery(discovery: Discovery) {
		if (confirm(common_confirmDeleteName({ name: discovery.name }))) {
			deleteDiscoveryMutation.mutate(discovery.id);
		}
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
		return runOutcomeTag(resultsOf(discovery));
	}

	/** A completed run's results, or null for a row that is not one. */
	function resultsOf(discovery: Discovery) {
		return discovery.run_type.type === 'Historical' ? discovery.run_type.results : null;
	}

	/** A run's recorded warnings, or none for a row that is not a completed run. */
	function warningsOf(discovery: Discovery) {
		return resultsOf(discovery)?.warnings ?? [];
	}

	/** Row actions. */
	function discoveryActions(discovery: Discovery): RowAction[] {
		const details: RowAction = {
			label: common_details(),
			icon: Info,
			isOpen: true,
			onClick: () => handleEditDiscovery(discovery)
		};
		if (!can('Discovery', 'Delete')) return [details];
		return [
			details,
			{
				label: common_delete(),
				icon: Trash2,
				tone: 'danger',
				isDelete: true,
				onBulk: handleBulkDelete,
				onClick: () => handleDeleteDiscovery(discovery),
				unavailable: refusedUnless(can('Discovery', 'Delete', discovery.site_id))
			}
		];
	}

	// Related entities the registry's discovery fields read: the daemon, the site.
	let lookup = $derived(collectionLookup({ Daemon: daemonsData, Site: sitesData }));

	let fields = $derived(
		discoveryHistoryColumns({
			lookup: () => lookup,
			daemons: () => daemonsData,
			sites: () => sitesData,
			tags: () => tagsData,
			tagGroups: () => tagGroupsQuery.data,
			daemonValues: () => daemonValuesQuery.data,
			siteValues: () => siteValuesQuery.data,
			typeValues: () => typeValuesQuery.data,
			phaseValues: () => phaseValuesQuery.data,
			resultsOf,
			outcomeTag,
			warningsOf,
			startedAtCell: () => startedAtCell,
			finishedAtCell: () => finishedAtCell
		})
	);
</script>

{#snippet startedAtCell(discovery: Discovery)}
	{@render timestampCell(resultsOf(discovery)?.started_at)}
{/snippet}

{#snippet finishedAtCell(discovery: Discovery)}
	{@render timestampCell(resultsOf(discovery)?.finished_at)}
{/snippet}

<!-- The full timestamp: runs are often minutes apart, so a date alone cannot tell them apart. -->
{#snippet timestampCell(timestamp: string | null | undefined)}
	{#if timestamp}
		{formatTimestamp(timestamp)}
	{:else}
		<span class="text-muted" aria-hidden="true">—</span>
		<span class="sr-only">{common_none()}</span>
	{/if}
{/snippet}

<div class="space-y-6">
	{#if pageLoad.failed}
		<LoadFailedState onRetry={pageLoad.retry} />
	{:else if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title={daemons_installPromptDiscoveries()} />
	{:else if isLoading}
		<Loading />
	{:else if listIsEmpty(discoveriesQuery, isNarrowed)}
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
			title={discovery_historyTitle()}
			items={discoveriesData}
			{fields}
			storageKey="scanopy-discovery-historical-table-state"
			defaults={tableDefaults}
			getItemId={(item) => item.id}
			serverPagination={discoveriesPagination}
			onPageChange={handlePageChange}
			onQueryChange={(query) => (listQuery = query)}
			serverQuery={{ tags: false, stale: false }}
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
