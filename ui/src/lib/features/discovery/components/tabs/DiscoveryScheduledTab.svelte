<script lang="ts">
	import { entities, billingPlans, discoveryTypes } from '$lib/shared/stores/metadata';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { Discovery } from '../../types/base';
	import { discoveryFields, formatScheduleDisplay, cancellingSessions } from '../../queries';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import SessionProgress from '../cards/SessionProgress.svelte';
	import DiscoveryEstimation from '../DiscoveryEstimation.svelte';
	import DiscoveryEditModal from '../DiscoveryModal/DiscoveryEditModal.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import { getFieldKey, type FieldConfig } from '$lib/shared/components/data/types';
	import { Plus, Play, Power, Edit, Trash2, Ban } from 'lucide-svelte';
	import type { CardAction } from '$lib/shared/components/data/types';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import {
		useDiscoveriesQuery,
		useCreateDiscoveryMutation,
		useUpdateDiscoveryMutation,
		useDeleteDiscoveryMutation,
		useBulkDeleteDiscoveriesMutation,
		useInitiateDiscoveryMutation,
		useActiveSessionsQuery,
		useCancelDiscoveryMutation
	} from '../../queries';
	import type { DiscoveryUpdatePayload } from '../../types/api';
	import { SvelteMap } from 'svelte/reactivity';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';
	import type { components } from '$lib/api/schema';
	import type { TabProps } from '$lib/shared/types';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import {
		common_confirmDeleteName,
		common_create,
		common_cancel,
		common_delete,
		common_disable,
		common_cancelling,
		common_lastRun,
		common_legacy,
		common_manual,
		common_never,
		common_none,
		common_phase,
		common_progress,
		common_schedule,
		common_status,
		discovery_schedulePausedFreePlan,
		common_edit,
		common_enable,
		common_run,
		common_tags,
		daemons_installPromptDiscoveries,
		discovery_alreadyRunning,
		discovery_cannotDeleteWhileRunning,
		discovery_cannotToggleWhileRunning,
		discovery_confirmDeleteScheduled,
		discovery_disableScheduleTooltip,
		discovery_enableScheduleTooltip,
		common_scans,
		discovery_legacyDaemonsWarning,
		discovery_noScheduledSessions,
		discovery_runType
	} from '$lib/paraglide/messages';

	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];

	let { isReadOnly = false, isActive = false }: TabProps = $props();

	// Organization query for onboarding state
	const organizationQuery = useOrganizationQuery();
	let onboarding = $derived((organizationQuery.data?.onboarding ?? []) as OnboardingOperation[]);

	// Queries
	const tagsQuery = useTagsQuery();
	// Gated on `isActive` — see the matching comment in DiscoveryHistoryTab. Both
	// subscribers must be gated for the query to actually go inactive.
	const discoveriesQuery = useDiscoveriesQuery(() => isActive);
	const daemonsQuery = useDaemonsQuery();
	const networksQuery = useNetworksQuery();

	// Active sessions
	const sessionsQuery = useActiveSessionsQuery();
	const cancelDiscoveryMutation = useCancelDiscoveryMutation();

	// Mutations
	const createDiscoveryMutation = useCreateDiscoveryMutation();
	const updateDiscoveryMutation = useUpdateDiscoveryMutation();
	const deleteDiscoveryMutation = useDeleteDiscoveryMutation();
	const bulkDeleteDiscoveriesMutation = useBulkDeleteDiscoveriesMutation();
	const initiateDiscoveryMutation = useInitiateDiscoveryMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let discoveriesData = $derived(discoveriesQuery.data ?? []);
	let daemonsData = $derived(daemonsQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);
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
		discoveriesQuery.isPending || daemonsQuery.isPending || sessionsQuery.isPending
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
	let hasLegacyDaemons = $derived(
		daemonsData.some((d) => d.version_status?.supports_unified_discovery === false)
	);

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

	function handleToggleEnabled(discovery: Discovery) {
		if (discovery.run_type.type !== 'Scheduled') return;
		updateDiscoveryMutation.mutate({
			...discovery,
			run_type: {
				...discovery.run_type,
				enabled: !discovery.run_type.enabled
			}
		});
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
		await downloadCsv('Discovery', {});
	}

	/**
	 * Row actions for table mode, matching what the card offers.
	 *
	 * A run in flight blocks the destructive and scheduling actions, and the
	 * tooltip carries the reason — same gating the card applies.
	 */
	function discoveryActions(discovery: Discovery): CardAction[] {
		if (isReadOnly) return [];

		const running = getActiveSession(discovery) !== null;
		const isRescan = discovery.discovery_type.type === 'Rescan';
		const isEnabled = discovery.run_type.type === 'Scheduled' && discovery.run_type.enabled;
		const actions: CardAction[] = [];

		if (!isRescan) {
			actions.push({
				label: common_edit(),
				icon: Edit,
				onClick: () => handleEditDiscovery(discovery)
			});
			actions.push({
				label: common_run(),
				icon: Play,
				onClick: () => handleDiscoveryRun(discovery),
				disabled: running,
				tooltip: running ? discovery_alreadyRunning() : undefined
			});

			if (discovery.run_type.type === 'Scheduled') {
				actions.push({
					label: isEnabled ? common_disable() : common_enable(),
					icon: Power,
					class: isEnabled ? 'btn-icon-success' : 'btn-icon',
					onClick: () => handleToggleEnabled(discovery),
					disabled: running,
					tooltip: running
						? discovery_cannotToggleWhileRunning()
						: isEnabled
							? discovery_disableScheduleTooltip()
							: discovery_enableScheduleTooltip()
				});
			}
		}

		const session = getActiveSession(discovery);
		if (session?.session_id) {
			actions.push({
				label: common_cancel(),
				icon: Ban,
				class: 'btn-icon-warning',
				onClick: () => handleCancelDiscovery(session.session_id!)
			});
		}

		actions.push({
			label: common_delete(),
			icon: Trash2,
			class: 'btn-icon-danger',
			onClick: () => handleDeleteDiscovery(discovery),
			disabled: running,
			tooltip: running ? discovery_cannotDeleteWhileRunning() : undefined
		});

		return actions;
	}

	/**
	 * Whether the org's plan runs schedules at all. A free plan keeps the cron on
	 * the record but never fires it, so the schedule reads as paused rather than
	 * as a time that will not happen.
	 */
	let schedulePaused = $derived.by(() => {
		const planType = organizationQuery.data?.plan?.type;
		if (!planType) return false;
		return !billingPlans.getMetadata(planType).features.scheduled_discovery;
	});

	/**
	 * How the shared discovery fields sit on this tab. `discovery_type` is the same
	 * on every scheduled scan, so it says nothing as a column — it stays a filter
	 * and group axis. `created_at` remains available from the field menu.
	 */
	const SHARED_FIELD_DISPLAY: Record<string, Record<string, unknown>> = {
		name: { order: 0 },
		network_id: { order: 2 },
		daemon_id: { order: 3 },
		discovery_type: { hidden: true },
		created_at: { hiddenByDefault: true }
	};

	let fields: FieldConfig<Discovery>[] = $derived([
		...discoveryFields(daemonsData, networksData).map((field) => {
			const overrides = SHARED_FIELD_DISPLAY[getFieldKey(field)];
			return overrides ? { ...field, display: { ...field.display, ...overrides } } : field;
		}),
		{
			key: 'legacy',
			label: common_status(),
			type: 'string',
			filterable: true,
			groupable: true,
			// Legacy-ness comes from the backend's own `is_legacy`, not a local list —
			// a `!== 'Unified'` check flagged Rescan, which is new rather than frozen.
			getValue: (item) =>
				discoveryTypes.getMetadata(item.discovery_type.type).is_legacy ? common_legacy() : '',
			display: {
				statusTag: true,
				getItems: (item) =>
					discoveryTypes.getMetadata(item.discovery_type.type).is_legacy
						? [{ id: 'legacy', label: common_legacy(), color: 'Yellow' }]
						: []
			}
		},
		{
			key: 'run_type',
			label: discovery_runType(),
			type: 'string',
			searchable: true,
			filterable: true,
			groupable: true,
			getValue: (item) => item.run_type.type,
			display: { order: 1 }
		},
		{
			key: 'schedule',
			label: common_schedule(),
			type: 'string',
			searchable: true,
			getValue: (item) =>
				item.run_type.type !== 'Scheduled'
					? common_manual()
					: schedulePaused
						? discovery_schedulePausedFreePlan()
						: formatScheduleDisplay(item.run_type.cron_schedule, item.run_type.timezone),
			display: { hiddenByDefault: true }
		},
		{
			key: 'last_run',
			label: common_lastRun(),
			type: 'string',
			getValue: (item) =>
				item.run_type.type !== 'Historical' && item.run_type.last_run
					? formatTimestamp(item.run_type.last_run)
					: common_never(),
			display: { hiddenByDefault: true }
		},
		{
			key: 'progress',
			label: common_progress(),
			// Progress belongs to the run, not the record, so there is nothing to
			// sort, filter or group by — but it is still a field, so a running scan
			// shows its tracker in the table as well as on the card.
			type: 'string',
			getValue: (item) => getActiveSession(item)?.phase ?? '',
			// After the tags column, immediately before the row actions: it is the
			// row's live state rather than one of its attributes.
			display: { trailing: true, cell: progressCell }
		},
		{
			key: 'tags',
			label: common_tags(),
			type: 'array',
			searchable: true,
			filterable: true,
			getValue: (entity) => {
				// Return tag names for search/filter display
				return entity.tags
					.map((id) => tagsData.find((t) => t.id === id)?.name)
					.filter((name): name is string => !!name);
			}
		}
		// `created_at` (sortable) comes from the shared `discoveryFields()` spread above.
	]);
</script>

{#snippet progressCell(discovery: Discovery)}
	{@const session = getActiveSession(discovery)}
	{#if session}
		{@const isCancelling = $cancellingSessions.get(session.session_id) === true}
		{@const phase = isCancelling ? common_cancelling() : session.phase}
		<div class="min-w-[16rem] space-y-2">
			<div class="flex items-center gap-2">
				<span class="text-secondary text-sm font-medium">{common_phase()}:</span>
				<span class="text-accent text-sm font-medium">{phase}</span>
			</div>

			<DiscoveryEstimation
				{phase}
				hosts_discovered={session.hosts_discovered}
				estimated_remaining_secs={session.estimated_remaining_secs}
			/>

			<SessionProgress
				session_id={session.session_id}
				phase={session.phase}
				progress={session.progress}
				cancelling={isCancelling}
			/>
		</div>
	{:else}
		<span class="text-muted" aria-hidden="true">—</span>
		<span class="sr-only">{common_none()}</span>
	{/if}
{/snippet}

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={common_scans()}>
		<svelte:fragment slot="actions">
			{#if hasDaemon(onboarding) && !isReadOnly}
				<button class="btn-primary flex items-center" onclick={handleCreateDiscovery}
					><Plus class="h-5 w-5" />{common_create()}</button
				>
			{/if}
		</svelte:fragment>
	</TabHeader>

	{#if hasLegacyDaemons}
		<InlineWarning
			title=""
			body={discovery_legacyDaemonsWarning()}
			dismissableKey="unified-discovery-migration"
		/>
	{/if}

	{#if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title={daemons_installPromptDiscoveries()} />
	{:else if isLoading}
		<Loading />
	{:else if discoveriesData.length === 0}
		<!-- Empty state -->
		<EmptyState
			title={discovery_noScheduledSessions()}
			subtitle=""
			onClick={isReadOnly ? undefined : handleCreateDiscovery}
			cta={isReadOnly ? undefined : common_create()}
		/>
	{:else}
		<DataControls
			items={discoveriesData.filter(
				(d) => d.run_type.type == 'AdHoc' || d.run_type.type == 'Scheduled'
			)}
			{fields}
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			storageKey="scanopy-discovery-scans-table-state"
			getItemId={(item) => item.id}
			getIcon={() => ({
				icon: entities.getIconComponent('Discovery'),
				color: entities.getColorHelper('Discovery').icon
			})}
			entityType={isReadOnly ? undefined : 'Discovery'}
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
