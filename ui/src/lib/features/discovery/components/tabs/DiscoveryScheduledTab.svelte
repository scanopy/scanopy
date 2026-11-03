<script lang="ts">
	import daemonVersionGates from '$lib/data/daemon-version-gates.json';
	import CreateButton from '$lib/shared/components/data/controls/CreateButton.svelte';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { TableDefaults } from '$lib/shared/components/data/types';
	import type { Discovery } from '../../types/base';
	import type { DiscoveryConfigOrderField } from '../../queries';
	import SessionPhaseProgress from '../cards/SessionPhaseProgress.svelte';
	import DiscoveryEditModal from '../DiscoveryModal/DiscoveryEditModal.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import LoadFailedState from '$lib/shared/components/layout/LoadFailedState.svelte';
	import { loadFailure } from '$lib/shared/utils/load-failure';
	import { discoveryScheduledColumns } from './DiscoveryScheduledTab.columns';
	import { collectionLookup } from '$lib/shared/entity-fields';
	import { Play, Power, Edit, Trash2, Ban } from '@lucide/svelte';
	import { disabledWhen, hiddenWhen, type CardAction } from '$lib/shared/components/data/types';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import {
		useScheduledDiscoveriesQuery,
		useCreateDiscoveryMutation,
		useUpdateDiscoveryMutation,
		useDeleteDiscoveryMutation,
		useBulkDeleteDiscoveriesMutation,
		useInitiateDiscoveryMutation,
		useActiveSessionsQuery,
		useCancelDiscoveryMutation,
		useBulkStartDiscoveriesMutation,
		useBulkCancelDiscoveriesMutation,
		useBulkSetDiscoveriesEnabledMutation
	} from '../../queries';
	import type { DiscoveryUpdatePayload } from '../../types/api';
	import { SvelteMap } from 'svelte/reactivity';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { isPreUnifiedDaemon } from '$lib/features/daemons/utils';
	import { useSitesQuery } from '$lib/features/sites/queries';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { isPlanLapsed } from '$lib/features/organizations/types';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';
	import type { components } from '$lib/api/schema';
	import type { TabProps } from '$lib/shared/types';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { listPageState } from '$lib/shared/components/data/listPage.svelte';
	import { useFieldValuesQuery } from '$lib/shared/api/field-values';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import {
		common_confirmDeleteName,
		common_create,
		common_cancel,
		common_delete,
		common_disable,
		common_none,
		common_edit,
		common_enable,
		common_run,
		daemons_installPromptDiscoveries,
		discovery_alreadyRunning,
		discovery_cannotDeleteWhileRunning,
		discovery_cannotToggleWhileRunning,
		discovery_rescanRunsOnce,
		discovery_rescanRunsOnceBulk,
		discovery_runningBulk,
		discovery_notRunning,
		discovery_notRunningBulk,
		discovery_noSchedule,
		discovery_noScheduleBulk,
		discovery_confirmDeleteScheduled,
		discovery_disableScheduleTooltip,
		discovery_enableScheduleTooltip,
		common_scans,
		discovery_legacyDaemonsWarning,
		discovery_noScheduledSessions
	} from '$lib/paraglide/messages';

	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];

	import { refusedUnless, useCan } from '$lib/features/auth/access';

	let { isActive = false }: TabProps = $props();
	const can = useCan();

	// Organization query for onboarding state
	const organizationQuery = useOrganizationQuery();
	let onboarding = $derived((organizationQuery.data?.onboarding ?? []) as OnboardingOperation[]);

	// Queries
	const tagsQuery = useTagsQuery();
	// One page of the scan configurations, paged, filtered and ordered on the server. Gated on
	// `isActive`, like the history tab: every tab is mounted at once.
	const page = listPageState(() => isActive);
	const discoveriesQuery = useScheduledDiscoveriesQuery(() => page.params());
	// Filter options: the values the scan configurations hold, scoped the way the list is.
	const DISCOVERY_FIELD_VALUES = '/api/v1/discoveries/field-values/{field}';
	const configScope = () => ({ historical: false });
	const daemonValuesQuery = useFieldValuesQuery(
		DISCOVERY_FIELD_VALUES,
		'daemon_id',
		configScope,
		() => isActive
	);
	const siteValuesQuery = useFieldValuesQuery(
		DISCOVERY_FIELD_VALUES,
		'site_id',
		configScope,
		() => isActive
	);
	const typeValuesQuery = useFieldValuesQuery(
		DISCOVERY_FIELD_VALUES,
		'discovery_type',
		configScope,
		() => isActive
	);
	const runTypeValuesQuery = useFieldValuesQuery(
		DISCOVERY_FIELD_VALUES,
		'run_type',
		configScope,
		() => isActive
	);
	const daemonsQuery = useDaemonsQuery();
	const sitesQuery = useSitesQuery();

	// Active sessions
	const sessionsQuery = useActiveSessionsQuery();
	const cancelDiscoveryMutation = useCancelDiscoveryMutation();

	// Mutations
	const createDiscoveryMutation = useCreateDiscoveryMutation();
	const updateDiscoveryMutation = useUpdateDiscoveryMutation();
	const deleteDiscoveryMutation = useDeleteDiscoveryMutation();
	const bulkDeleteDiscoveriesMutation = useBulkDeleteDiscoveriesMutation();
	const initiateDiscoveryMutation = useInitiateDiscoveryMutation();
	const bulkStartDiscoveriesMutation = useBulkStartDiscoveriesMutation();
	const bulkCancelDiscoveriesMutation = useBulkCancelDiscoveriesMutation();
	const bulkSetEnabledMutation = useBulkSetDiscoveriesEnabledMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let discoveriesData = $derived(discoveriesQuery.data?.items ?? []);
	let discoveriesPagination = $derived(discoveriesQuery.data?.pagination ?? null);
	let daemonsData = $derived(daemonsQuery.data ?? []);
	let sitesData = $derived(sitesQuery.data ?? []);
	let sessionsList = $derived(sessionsQuery.data ?? []);

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
	// means it stays `isPending` forever.
	let isLoading = $derived(
		(discoveriesQuery.isPending && !discoveriesQuery.data) ||
			daemonsQuery.isPending ||
			sessionsQuery.isPending
	);
	let pageLoad = $derived(
		loadFailure(organizationQuery, discoveriesQuery, daemonsQuery, sessionsQuery)
	);

	// Build lookup: discovery_id -> session (server always enriches discovery_id)
	let sessionByDiscoveryId = $derived.by(() => {
		const map = new SvelteMap<string, DiscoveryUpdatePayload>();
		for (const session of sessionsList) {
			if (session.discovery_id) {
				map.set(session.discovery_id, session);
			}
		}
		return map;
	});

	function getActiveSession(discovery: Discovery): DiscoveryUpdatePayload | null {
		return sessionByDiscoveryId.get(discovery.id) ?? null;
	}
	let hasLegacyDaemons = $derived(daemonsData.some(isPreUnifiedDaemon));

	let showDiscoveryModal = $state(false);
	let editingDiscovery: Discovery | null = $state(null);

	// Deep-link: open discovery editor from URL (handles both fresh open and entity switch)
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'discovery-editor',
			discoveriesData,
			showDiscoveryModal,
			editingDiscovery?.id
		);
		if (result !== undefined) {
			editingDiscovery = result;
			showDiscoveryModal = true;
		}
	});

	function handleCreateDiscovery() {
		editingDiscovery = null;
		showDiscoveryModal = true;
	}

	function handleEditDiscovery(discovery: Discovery) {
		editingDiscovery = discovery;
		showDiscoveryModal = true;
	}

	function handleDeleteDiscovery(discovery: Discovery) {
		if (confirm(common_confirmDeleteName({ name: discovery.name }))) {
			deleteDiscoveryMutation.mutate(discovery.id);
		}
	}

	function handleDiscoveryRun(discovery: Discovery) {
		initiateDiscoveryMutation.mutate(discovery.id);
	}

	function handleCancelDiscovery(sessionId: string) {
		cancelDiscoveryMutation.mutate(sessionId);
	}

	async function handleBulkRun(ids: string[]) {
		await bulkStartDiscoveriesMutation.mutateAsync(ids);
	}

	async function handleBulkCancel(ids: string[]) {
		const sessionIds = discoveriesData
			.filter((discovery) => ids.includes(discovery.id))
			.map((discovery) => getActiveSession(discovery)?.session_id)
			.filter((id): id is string => !!id);
		await bulkCancelDiscoveriesMutation.mutateAsync({ discoveryIds: ids, sessionIds });
	}

	/** Turns the given discoveries' schedules on or off; the row toggle passes its one id. */
	function setEnabled(enabled: boolean) {
		return (ids: string[]) => bulkSetEnabledMutation.mutateAsync({ ids, enabled });
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
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(discovery_confirmDeleteScheduled({ count: ids.length }))) {
			await bulkDeleteDiscoveriesMutation.mutateAsync(ids);
		}
	}

	// CSV export handler
	async function handleCsvExport() {
		// A one-off copy for the request, never read reactively.
		// eslint-disable-next-line svelte/prefer-svelte-reactivity
		const params = new URLSearchParams(page.listQuery);
		params.set('historical', 'false');
		await downloadCsv('Discovery', params);
	}

	/**
	 * Row actions: Cancel and the schedule toggle (each only in its state), then Edit, Run and
	 * Delete on every row.
	 *
	 * A run in flight blocks the destructive and scheduling actions, and the
	 * tooltip carries the reason.
	 */
	function discoveryActions(discovery: Discovery): CardAction[] {
		if (!can('Discovery', 'Write')) return [];

		const writeRefusal = refusedUnless(can('Discovery', 'Write', discovery.site_id));
		const session = getActiveSession(discovery);
		const running = session !== null;
		const isRescan = discovery.discovery_type.type === 'Rescan';
		const isScheduled = discovery.run_type.type === 'Scheduled';
		const isEnabled = discovery.run_type.type === 'Scheduled' && discovery.run_type.enabled;
		const whileRescan = disabledWhen(
			isRescan,
			discovery_rescanRunsOnce(),
			discovery_rescanRunsOnceBulk()
		);
		const whileRunning = (reason: string) => disabledWhen(running, reason, discovery_runningBulk());

		return [
			{
				label: common_cancel(),
				icon: Ban,
				tone: 'warning',
				onClick: () => session?.session_id && handleCancelDiscovery(session.session_id),
				onBulk: handleBulkCancel,
				unavailable:
					hiddenWhen(!session?.session_id, discovery_notRunning(), discovery_notRunningBulk()) ??
					writeRefusal
			},
			{
				label: isEnabled ? common_disable() : common_enable(),
				icon: Power,
				tone: isEnabled ? 'success' : undefined,
				shortcut: 'O',
				onClick: () => setEnabled(!isEnabled)([discovery.id]),
				onBulk: setEnabled(!isEnabled),
				unavailable:
					hiddenWhen(!isScheduled, discovery_noSchedule(), discovery_noScheduleBulk()) ??
					writeRefusal ??
					whileRunning(discovery_cannotToggleWhileRunning()),
				tooltip: isEnabled ? discovery_disableScheduleTooltip() : discovery_enableScheduleTooltip()
			},
			{
				label: common_edit(),
				icon: Edit,
				isOpen: true,
				onClick: () => handleEditDiscovery(discovery),
				unavailable: writeRefusal ?? whileRescan
			},
			{
				label: common_run(),
				icon: Play,
				onClick: () => handleDiscoveryRun(discovery),
				onBulk: handleBulkRun,
				unavailable: writeRefusal ?? whileRescan ?? whileRunning(discovery_alreadyRunning())
			},
			{
				label: common_delete(),
				icon: Trash2,
				tone: 'danger',
				isDelete: true,
				onBulk: handleBulkDelete,
				onClick: () => handleDeleteDiscovery(discovery),
				unavailable:
					refusedUnless(can('Discovery', 'Delete', discovery.site_id)) ??
					whileRunning(discovery_cannotDeleteWhileRunning())
			}
		];
	}

	/**
	 * Whether the org's schedules fire at all. A free plan keeps the cron on the
	 * record but never fires it, and a lapsed org (subscription ended, no paid
	 * plan chosen since) is skipped by the scheduler the same way, so the
	 * schedule reads as paused rather than as a time that will not happen.
	 */
	let scheduleLapsed = $derived(
		organizationQuery.data != null && isPlanLapsed(organizationQuery.data)
	);
	let schedulePaused = $derived.by(() => {
		if (scheduleLapsed) return true;
		const planType = organizationQuery.data?.plan?.type;
		if (!planType) return false;
		return !billingPlans.getMetadata(planType).features.scheduled_discovery;
	});

	const tableDefaults: TableDefaults<DiscoveryConfigOrderField> = {
		sort: { field: 'name', direction: 'asc' }
	};

	// Related entities the registry's discovery fields read: the daemon, the site.
	let lookup = $derived(collectionLookup({ Daemon: daemonsData, Site: sitesData }));

	let fields = $derived(
		discoveryScheduledColumns({
			lookup: () => lookup,
			daemons: () => daemonsData,
			sites: () => sitesData,
			tags: () => tagsData,
			scheduleLapsed: () => scheduleLapsed,
			schedulePaused: () => schedulePaused,
			getActiveSession,
			progressCell: () => progressCell,
			daemonValues: () => daemonValuesQuery.data,
			siteValues: () => siteValuesQuery.data,
			typeValues: () => typeValuesQuery.data,
			runTypeValues: () => runTypeValuesQuery.data
		})
	);
</script>

{#snippet progressCell(discovery: Discovery)}
	{@const session = getActiveSession(discovery)}
	{#if session}
		<SessionPhaseProgress {session} />
	{:else}
		<span class="text-muted" aria-hidden="true">—</span>
		<span class="sr-only">{common_none()}</span>
	{/if}
{/snippet}

<!-- The page's own actions, last in the table toolbar beside the filter and columns. -->
{#snippet toolbarActions()}
	{#if hasDaemon(onboarding) && can('Discovery', 'Write')}
		<CreateButton onclick={handleCreateDiscovery} />
	{/if}
{/snippet}

<div class="space-y-6">
	{#if hasLegacyDaemons}
		<InlineWarning
			title=""
			body={discovery_legacyDaemonsWarning({
				version: daemonVersionGates.last_without_unified_discovery
			})}
			dismissableKey="unified-discovery-migration"
		/>
	{/if}

	{#if pageLoad.failed}
		<LoadFailedState onRetry={pageLoad.retry} />
	{:else if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title={daemons_installPromptDiscoveries()} />
	{:else if isLoading}
		<Loading />
	{:else if page.isEmpty(discoveriesQuery)}
		<!-- Empty state -->
		<EmptyState
			title={discovery_noScheduledSessions()}
			subtitle=""
			onClick={can('Discovery', 'Write') ? handleCreateDiscovery : undefined}
			cta={can('Discovery', 'Write') ? common_create() : undefined}
		/>
	{:else}
		<DataControls
			title={common_scans()}
			{toolbarActions}
			items={discoveriesData}
			{fields}
			storageKey="scanopy-discovery-scans-table-state"
			defaults={tableDefaults}
			getItemId={(item) => item.id}
			serverPagination={discoveriesPagination}
			onPageChange={page.onPageChange}
			onQueryChange={page.onQueryChange}
			serverQuery={{ stale: false }}
			entityType="Discovery"
			canEditTags={can('Tag', 'Write')}
			getItemTags={(item) => item.tags}
			onCsvExport={handleCsvExport}
			getActions={discoveryActions}
			entityLabel={common_scans()}
		></DataControls>
	{/if}
</div>

<DiscoveryEditModal
	name="discovery-editor"
	isOpen={showDiscoveryModal}
	daemons={daemonsData}
	hosts={hostsData}
	discovery={editingDiscovery}
	hasActiveSession={editingDiscovery ? !!getActiveSession(editingDiscovery) : false}
	onCreate={handleDiscoveryCreate}
	onUpdate={handleDiscoveryUpdate}
	onClose={handleCloseEditor}
	onDelete={editingDiscovery
		? () => {
				handleDeleteDiscovery(editingDiscovery!);
				handleCloseEditor();
			}
		: null}
/>
