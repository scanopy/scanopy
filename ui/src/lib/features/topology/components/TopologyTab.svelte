<script lang="ts">
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';
	import TopologyViewer from './visualization/TopologyViewer.svelte';
	import TopologyOptionsPanel from './panel/TopologyOptionsPanel.svelte';
	import { Camera, Radar, Share2, Trash2 } from 'lucide-svelte';
	import ExportButton from './ExportButton.svelte';
	import ExportModal from './ExportModal.svelte';
	import SharesModal from '$lib/features/shares/components/SharesModal.svelte';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import { SvelteFlowProvider } from '@xyflow/svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import { onMount, onDestroy, setContext } from 'svelte';
	import { get, writable } from 'svelte/store';
	import {
		useTopologiesQuery,
		useTopologyDataQuery,
		markTopologyViewed,
		selectedTopologyId,
		selectedNetworkId,
		selectedSnapshotId,
		selectedNodes,
		consumePreferredNetwork,
		loadSelectedNetworkFromStorage,
		activeView,
		topologyReadOnly,
		topologyOptions,
		updateTopologyOptions,
		hydrateStoresFromTopology,
		getTopologyParamsFromUrl,
		pushTopologyParams,
		showViewSwitcherHint,
		showDependencyTutorial,
		type TopologyView
	} from '../queries';
	import {
		useSnapshotsQuery,
		useTakeSnapshotMutation,
		useDeleteSnapshotMutation,
		type Snapshot
	} from '$lib/features/snapshots/queries';
	import { SnapshotDisplay } from '$lib/shared/components/forms/selection/display/SnapshotDisplay.svelte';
	import { NetworkDisplay } from '$lib/shared/components/forms/selection/display/NetworkDisplay.svelte';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { triggerUpgrade } from '$lib/features/billing/trigger-upgrade';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { makeGraphRule } from '../types/grouping';
	import type { ContainerGraphRule } from '../types/grouping';
	import { newNodeIds, updateTagFilter } from '../interactions';
	import { clearSelection } from '../selection';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import {
		SimpleOptionDisplay,
		type SimpleOption
	} from '$lib/shared/components/forms/selection/display/SimpleOptionDisplay';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import { useActiveSessionsQuery } from '$lib/features/discovery/queries';
	import { toRenderableTopology } from '$lib/features/topology/enriched';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import { formatTimestamp, formatDate } from '$lib/shared/utils/formatting';
	import ApplicationSetupWizard from './application-wizard/ApplicationSetupWizard.svelte';
	import L2EmptyStateOverlay from './L2EmptyStateOverlay.svelte';
	import ViewSwitcherHint from './ViewSwitcherHint.svelte';
	import DependencyTutorial from './DependencyTutorial.svelte';
	import { TUTORIAL_TOPOLOGY } from './dependency-tutorial-data';
	import { useUsersQuery } from '$lib/features/users/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { waitForOrgUpdate } from '$lib/shared/billing/wait-for-org-update';
	import type { components } from '$lib/api/schema';
	import { billingPlans, entities, permissions, views } from '$lib/shared/stores/metadata';
	import { getInspectorConfig } from './panel/inspectors/view-config';
	import type { TabProps } from '$lib/shared/types';
	import {
		common_delete,
		topology_lastScanned,
		topology_liveView,
		topology_noTopologySelected,
		topology_snapshotDeleteConfirm,
		topology_takeSnapshot
	} from '$lib/paraglide/messages';
	import { useConfigQuery } from '$lib/shared/stores/config-query';

	let { isReadOnly = false, isActive = false }: TabProps = $props();

	// Get current user to check permissions
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);
	let canViewUsers = $derived(
		currentUser
			? (permissions.getMetadata(currentUser.permissions)?.grantable_user_permissions?.length ??
					0) > 0
			: false
	);

	// Snapshot selection drives as-of-T entity reads. When a snapshot is
	// selected, entity queries read SCD2 state as of its `taken_at` instead of
	// live, so the inspector shows the host/service/etc. as they were captured.
	const snapshotsQuery = useSnapshotsQuery(() => $selectedNetworkId ?? undefined);
	let snapshotsData = $derived(snapshotsQuery.data ?? []);
	let selectedSnapshot = $derived(
		$selectedSnapshotId ? snapshotsData.find((s) => s.id === $selectedSnapshotId) : null
	);
	// Queries - TanStack Query handles deduplication
	const tagsQuery = useTagsQuery();
	useUsersQuery({ enabled: () => canViewUsers });
	const topologiesQuery = useTopologiesQuery();
	const networksQuery = useNetworksQuery();
	const organizationQuery = useOrganizationQuery();
	const activeSessionsQuery = useActiveSessionsQuery();
	const configQuery = useConfigQuery();
	// Single bundle query for the topology view's entity set. `snapshot_id =
	// undefined` returns live entities; `snapshot_id = <id>` returns the
	// captured set as-of that snapshot's `taken_at`. Same code path for both
	// — see backend `GET /api/v1/topology/data`.
	const topologyDataQuery = useTopologyDataQuery(
		() => $selectedNetworkId ?? undefined,
		() => $selectedSnapshotId ?? undefined
	);

	// Derived data
	let topologiesData = $derived(topologiesQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);
	let isLoading = $derived(topologiesQuery.isPending || topologyDataQuery.isPending);

	let hasEmail = $derived(configQuery.data?.has_email_service ?? false);

	// Snapshot retention (0 = snapshots disabled on this plan).
	// Reads the per-plan fixture value via the billingPlans metadata store
	// with the deployment-wide env override (`/api/config`) taking precedence —
	// mirrors `BillingPlan::snapshot_retention_days` on the backend.
	let snapshotRetentionDays = $derived.by(() => {
		const override = configQuery.data?.snapshot_retention_days_override;
		if (override != null) return override;
		const planType = organizationQuery.data?.plan?.type;
		if (!planType) return 0;
		return billingPlans.getMetadata(planType)?.features?.snapshot_retention_days ?? 0;
	});
	let snapshotsEnabled = $derived(snapshotRetentionDays > 0);

	// Application wizard gate
	let appTags = $derived((tagsQuery.data ?? []).filter((t) => t.is_application));
	let wizardOpen = $state(false);

	let currentInspectorConfig = $derived(getInspectorConfig($activeView));

	// View-only when the tab is read-only or a snapshot is selected. Single
	// reactive source consumed by the inspectors / grouping editor / edit mode.
	$effect(() => {
		topologyReadOnly.set(isReadOnly || $selectedSnapshotId !== null);
	});

	// Auto-open wizard when entering a view with app picker and no app tags.
	// Never on a read-only view — the wizard creates tags.
	let tagsLoaded = $derived(!tagsQuery.isLoading && !tagsQuery.isPending);
	$effect(() => {
		if (
			isActive &&
			tagsLoaded &&
			!$topologyReadOnly &&
			currentInspectorConfig.show_application_picker &&
			appTags.length === 0 &&
			!wizardOpen
		) {
			wizardOpen = true;
		}
	});

	let showAppWizard = $derived(
		isActive && currentInspectorConfig.show_application_picker && wizardOpen
	);

	// Sentinel constant for the live-view option in the snapshot dropdown.
	// Snapshot ids are UUIDv4, never match this string.
	const LIVE_VIEW_SENTINEL = '__live__';

	// Mutations
	const takeSnapshotMutation = useTakeSnapshotMutation();
	const deleteSnapshotMutation = useDeleteSnapshotMutation();

	// There is exactly one topology row per network (it holds only grouping
	// `options`). Both live and snapshot views use it — the snapshot-ness lives
	// in the entity/graph bundle, which is built on request from the snapshot's
	// closed copies when one is selected.
	let currentTopologyRow = $derived.by(() => {
		const networkId = $selectedNetworkId;
		if (!networkId) return null;
		return topologiesData.find((t) => t.network_id === networkId) ?? null;
	});

	// Display name for the currently selected topology — network name for
	// live, formatted snapshot timestamp otherwise.
	let currentTopologyName = $derived.by(() => {
		if (!currentTopologyRow) return '';
		if (selectedSnapshot) {
			return formatTimestamp(selectedSnapshot.taken_at);
		}
		const network = networksData.find((n) => n.id === currentTopologyRow.network_id);
		return network?.name ?? '';
	});

	// Enriched topology: graph row + entity bundle for the selected view.
	// Live view → live entities; snapshot view → entities as-of the snapshot's
	// `taken_at` (single backend endpoint serves both).
	let currentTopology = $derived.by(() => {
		if (!currentTopologyRow) return null;
		const bundle = topologyDataQuery.data;
		if (!bundle) return null;
		return toRenderableTopology(
			currentTopologyRow,
			{
				hosts: bundle.hosts,
				services: bundle.services,
				subnets: bundle.subnets,
				ip_addresses: bundle.ip_addresses,
				ports: bundle.ports,
				bindings: bundle.bindings,
				interfaces: bundle.interfaces,
				dependencies: bundle.dependencies,
				vlans: bundle.vlans,
				entity_tags: bundle.tags,
				// Per-view graph built on request by the backend (snapshot-aware).
				nodes: bundle.nodes,
				edges: bundle.edges
			},
			currentTopologyName,
			$activeView
		);
	});

	// Expose the enriched topology to descendant inspectors via context. They
	// resolve it through `useTopology()` and read entity arrays off it.
	// svelte-ignore state_referenced_locally
	const topologyContext = writable<RenderableTopology | null>(currentTopology);
	setContext('topology', topologyContext);
	$effect(() => {
		topologyContext.set(currentTopology);
	});

	// L2 Physical: show empty state overlay when no nodes (no neighbor data discovered)
	let showL2EmptyState = $derived(
		isActive &&
			$activeView === 'L2Physical' &&
			currentTopology != null &&
			currentTopology.nodes.length === 0
	);

	// Update tag filter stores when topology or options change
	$effect(() => {
		const hideMetadata = ($topologyOptions.request.hide_metadata_values ?? {}) as Record<
			string,
			Record<string, Record<string, string[]>>
		>;
		const hideEntities = ($topologyOptions.request.hide_entities ?? {}) as Record<string, string[]>;
		updateTagFilter(
			currentTopology ?? undefined,
			$topologyOptions.local.tag_filter,
			$activeView,
			hideMetadata[$activeView],
			hideEntities[$activeView] ?? []
		);
	});

	// Find active discovery session for current topology's network
	let activeSession = $derived(
		currentTopology
			? (activeSessionsQuery.data ?? []).find((s) => s.network_id === currentTopology.network_id)
			: null
	);
	let discoveryColor = $derived(entities.getColorHelper('Discovery'));

	// View selector — built from fixture data
	import viewsJson from '$lib/data/views.json';

	const allViewOptions: SimpleOption[] = viewsJson.map((p) => ({
		value: p.id,
		label: p.name,
		description: p.description,
		icon: views.getIconComponent(p.id),
		iconColor: views.getColorHelper(p.id).icon
	}));

	// A snapshot can only show views whose data it captured (you can't set up
	// SNMP or create app tags on a historical snapshot), so restrict the picker
	// to `available_views`. Live shows every view, with its setup prompts.
	let viewOptions = $derived.by<SimpleOption[]>(() => {
		if ($selectedSnapshotId == null) return allViewOptions;
		const available = topologyDataQuery.data?.available_views;
		if (!available) return allViewOptions;
		return allViewOptions.filter((o) => available.includes(o.value as TopologyView));
	});

	// If the active view isn't available in the selected snapshot, fall back to
	// an available one (prefer L3Logical) so the picker and canvas stay in sync.
	$effect(() => {
		if ($selectedSnapshotId == null) return;
		const available = topologyDataQuery.data?.available_views;
		if (available && available.length > 0 && !available.includes($activeView)) {
			activeView.set(available.includes('L3Logical') ? 'L3Logical' : available[0]);
		}
	});

	let viewColorStyle = $derived(views.getColorHelper($activeView));

	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];
	let onboarding = $derived((organizationQuery.data?.onboarding ?? []) as OnboardingOperation[]);
	let hasCompletedFirstDiscovery = $derived(
		onboarding.length === 0 || onboarding.includes('FirstDiscoveryCompleted')
	);

	// Complete the "View your topology" onboarding step the first time the user is focused on
	// the topology tab with hosts to look at (and discovery already done). markTopologyViewed
	// is a one-shot GET carrying mark_viewed=true — only sent here, never by the background
	// data query — so the milestone only fires from an actual on-tab view, never from other
	// tabs. Then poll the org until the milestone lands so the checklist updates. The
	// `tracked` flag only flips once we actually call, so if discovery completes later while
	// the user is on the tab, the effect re-fires.
	let firstTopologyMilestoneTracked = $state(false);
	$effect(() => {
		const hasHosts = (topologyDataQuery.data?.hosts.length ?? 0) > 0;
		if (
			isActive &&
			$selectedNetworkId &&
			hasHosts &&
			onboarding.includes('FirstDiscoveryCompleted') &&
			!firstTopologyMilestoneTracked &&
			!onboarding.includes('FirstTopologyRebuild')
		) {
			firstTopologyMilestoneTracked = true;
			const networkId = $selectedNetworkId;
			void (async () => {
				await markTopologyViewed(networkId);
				void waitForOrgUpdate((o) => o.onboarding.includes('FirstTopologyRebuild'));
			})();
		}
	});

	// URL params: read once on init for topology/view deep-linking
	const urlParams = getTopologyParamsFromUrl();
	let urlViewConsumed = false;

	// Initialize/validate the selected network against the accessible list.
	// Runs whenever networksData changes so a stale persisted id (deleted network,
	// changed org, revoked perms) is replaced instead of fetched — which would 404
	// and fire a toast on every reload.
	$effect(() => {
		// No networks: leave selection null so useTopologyDataQuery stays disabled.
		if (networksData.length === 0) return;
		// Current selection is still accessible — nothing to do.
		if ($selectedNetworkId && networksData.some((n) => n.id === $selectedNetworkId)) return;

		// Persisted selection, if still accessible.
		const persisted = loadSelectedNetworkFromStorage();
		if (persisted && networksData.some((n) => n.id === persisted)) {
			selectedNetworkId.set(persisted);
			return;
		}
		// Preferred (e.g. just-onboarded network), if accessible.
		const preferredNetworkId = consumePreferredNetwork();
		if (preferredNetworkId && networksData.some((n) => n.id === preferredNetworkId)) {
			selectedNetworkId.set(preferredNetworkId);
			return;
		}
		// Fall back to the first available network; the store subscription persists
		// it, overwriting any stale value in localStorage.
		selectedNetworkId.set(networksData[0].id);
	});

	// Reset snapshot selection when network changes (live view by default)
	$effect(() => {
		// Touch the store so the effect re-runs on network change.
		void $selectedNetworkId;
		selectedSnapshotId.set(null);
	});

	// Sync selected topology id from currentTopology so dependent code paths
	// (URL sync, viewer query) stay consistent.
	$effect(() => {
		if (currentTopology && currentTopology.id !== $selectedTopologyId) {
			selectedTopologyId.set(currentTopology.id);
		}
	});

	// Hydrate stores from topology options when a topology is selected.
	let lastHydratedId: string | null = null;
	let isFirstHydration = true;
	$effect(() => {
		if (currentTopology && currentTopology.id !== lastHydratedId) {
			lastHydratedId = currentTopology.id;
			hydrateStoresFromTopology(currentTopology, isFirstHydration);
			if (!urlViewConsumed && urlParams.view) {
				urlViewConsumed = true;
				activeView.set(urlParams.view);
			}
			isFirstHydration = false;
		}
	});

	// New node highlight: snapshot pre-rebuild node IDs, detect new ones after rebuild
	let preRebuildNodeIds: Set<string> | null = $state(null);

	// Detect new nodes after live updates arrive
	$effect(() => {
		if (!preRebuildNodeIds || !currentTopology) return;
		const currentIds = currentTopology.nodes.map((n) => n.id);
		const addedIds = currentIds.filter((id) => !preRebuildNodeIds!.has(id));
		preRebuildNodeIds = null;
		if (addedIds.length > 0) {
			newNodeIds.set(new Set(addedIds));
			topologyViewer?.triggerFitView();
			setTimeout(() => {
				newNodeIds.set(new Set());
			}, 2000);
		}
	});

	let isShareModalOpen = $state(false);
	let isExportModalOpen = $state(false);

	let topologyViewer: TopologyViewer | null = $state(null);

	// Track topology_viewed once per topology when tab is active and data is loaded
	let topologyViewTracked = new SvelteSet<string>();
	$effect(() => {
		if (!isActive || !currentTopology) return;
		if (topologyViewTracked.has(currentTopology.id)) return;
		topologyViewTracked.add(currentTopology.id);
		trackEvent('topology_viewed', {
			network_id: currentTopology.network_id,
			node_count: currentTopology.nodes.length,
			view_type: 'app'
		});
	});

	function clearMultiSelect() {
		selectedNodes.set([]);
	}

	// Handle network selection
	function handleNetworkChange(value: string) {
		selectedNetworkId.set(value);
		// Reset to live view synchronously here (not just via the network-change
		// $effect) so the snapshot is cleared before the data query recomputes —
		// otherwise it briefly fires with the new network + old snapshot id and
		// the backend 403s ("Snapshot belongs to a different network").
		selectedSnapshotId.set(null);
		clearSelection();
	}

	// Build snapshot dropdown options. Includes a synthetic "Live view" entry
	// at the top with id LIVE_VIEW_SENTINEL.
	let snapshotOptions = $derived<Snapshot[]>([
		{
			id: LIVE_VIEW_SENTINEL,
			network_id: $selectedNetworkId ?? '',
			taken_at: new Date(0).toISOString(),
			created_by_user_id: null,
			created_at: new Date(0).toISOString(),
			updated_at: new Date(0).toISOString()
		} as Snapshot,
		...snapshotsData
	]);

	// Live view subtitle: when the network was last scanned, derived from the
	// most recent `last_seen_at` across the live host set (discovery refreshes
	// it on every observation). Empty when nothing has been scanned yet.
	let lastScannedDescription = $derived.by(() => {
		const times = (topologyDataQuery.data?.hosts ?? [])
			.map((h) => h.last_seen_at)
			.filter((t): t is string => !!t);
		if (times.length === 0) return '';
		const latest = times.reduce((a, b) => (a > b ? a : b));
		return topology_lastScanned({ date: formatDate(latest) });
	});

	// Override the SnapshotDisplay to render the live-view sentinel with a
	// localized label and a "last scanned" subtitle rather than a date string.
	let snapshotDisplayWithLive = $derived({
		...SnapshotDisplay,
		getLabel: (s: Snapshot, ctx: object) =>
			s.id === LIVE_VIEW_SENTINEL ? topology_liveView() : SnapshotDisplay.getLabel(s, ctx),
		getDescription: (s: Snapshot, ctx: object) =>
			s.id === LIVE_VIEW_SENTINEL
				? lastScannedDescription
				: (SnapshotDisplay.getDescription?.(s, ctx) ?? '')
	});

	function handleSnapshotChange(value: string) {
		const next = value === LIVE_VIEW_SENTINEL ? null : value;
		selectedSnapshotId.set(next);
		clearSelection();
	}

	// Handle view selection (user-initiated)
	function handleViewChange(value: string) {
		const view = value as TopologyView;
		pushTopologyParams(get(selectedTopologyId), view);
		activeView.set(view);
		clearSelection();
		showViewSwitcherHint.set(false);
	}

	// Tutorial / hint state
	let viewSwitcherEl: HTMLDivElement | undefined = $state();
	let tutorialTypeToggled = $state(false);

	function dismissDependencyTutorial() {
		showDependencyTutorial.set(false);
		selectedNodes.set([]);
		tutorialTypeToggled = false;
	}

	$effect(() => {
		const active = isActive;
		return () => {
			if (active && get(showDependencyTutorial)) {
				dismissDependencyTutorial();
			}
		};
	});

	async function handleTakeSnapshot() {
		const networkId = $selectedNetworkId;
		if (!networkId) return;

		if (!snapshotsEnabled) {
			triggerUpgrade({
				surface: 'topology_tab',
				gate_type: 'plan_required',
				feature: 'snapshots',
				source: 'topology_tab'
			});
			return;
		}

		await takeSnapshotMutation.mutateAsync({ network_id: networkId });
	}

	async function handleDeleteSnapshot() {
		const networkId = $selectedNetworkId;
		const snapshotId = $selectedSnapshotId;
		if (!networkId || !snapshotId) return;
		if (!confirm(topology_snapshotDeleteConfirm())) return;
		// Return to live view BEFORE deleting: the mutation's onSuccess invalidates
		// the topology queries, which would otherwise refetch the just-deleted
		// snapshot's data and 404 with a misleading "snapshot not found" toast.
		selectedSnapshotId.set(null);
		await deleteSnapshotMutation.mutateAsync({
			snapshot_id: snapshotId,
			network_id: networkId
		});
	}

	function handleWizardComplete() {
		wizardOpen = false;
		const tagIds = appTags.map((t) => t.id);
		updateTopologyOptions((current) => {
			const allRules = current.request.container_rules ?? {};
			const appRules: ContainerGraphRule[] = allRules['Application'] ?? [];
			return {
				...current,
				request: {
					...current.request,
					container_rules: {
						...allRules,
						Application: [
							...appRules.filter(
								(r) =>
									typeof r.rule === 'string' ||
									!('ByApplication' in (r.rule as Record<string, unknown>))
							),
							makeGraphRule({ ByApplication: { tag_ids: tagIds } }) as ContainerGraphRule
						]
					}
				}
			};
		});
	}

	// Browser back/forward: restore topology + view from URL params
	function handlePopState() {
		const params = getTopologyParamsFromUrl();
		if (params.topologyId && params.topologyId !== get(selectedTopologyId)) {
			selectedTopologyId.set(params.topologyId);
			selectedNodes.set([]);
		}
		if (params.view && params.view !== get(activeView)) {
			activeView.set(params.view);
		}
	}

	onMount(() => {
		window.addEventListener('popstate', handlePopState);
	});

	onDestroy(() => {
		window.removeEventListener('popstate', handlePopState);
	});
</script>

<SvelteFlowProvider>
	{#if organizationQuery.isPending}
		<!-- org(含 onboarding)未加载完前先 loading,避免闪「装 daemon」空状态:
		     pending 时 data 为空 → onboarding=[] → hasDaemon=false 会误判成无 daemon。 -->
		<div class="flex min-h-[60vh] items-center justify-center">
			<Loading />
		</div>
	{:else if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title="Install a daemon to start mapping your network topology." />
	{:else}
		<div class="space-y-3">
			<!-- Header -->
			<div
				class="card card-static flex items-center justify-evenly gap-2 px-2 py-2"
				style="border-bottom: 2px solid {viewColorStyle.rgb}; transition: border-color 0.3s ease;"
			>
				{#if currentTopology}
					<div class="flex items-center gap-2">
						<ExportButton onclick={() => (isExportModalOpen = true)} />
						<!-- A share always renders the LIVE view (the backend builds the share
						     graph from live entities + options), so the button stays available
						     while viewing a snapshot; the share modal shows an inline notice
						     clarifying that a share captures the live view, not the snapshot. -->
						{#if !isReadOnly}
							{#if currentUser && !currentUser.email_verified}
								<span data-tooltip="Please verify email to share topology" use:tooltip>
									<button class="btn-secondary opacity-50" disabled title="Share">
										<Share2 class="my-1 h-5 w-5" />
									</button>
								</span>
							{:else}
								<button
									class="btn-secondary"
									onclick={() => (isShareModalOpen = true)}
									title="Share"
								>
									<Share2 class="my-1 h-5 w-5" />
								</button>
							{/if}
						{/if}
					</div>

					{#if !isReadOnly}
						<div class="card-divider-v self-stretch"></div>
						{#if activeSession}
							{@const estimate = activeSession.estimated_remaining_secs}
							<div
								class="flex cursor-default flex-col items-center {discoveryColor.icon}"
								data-tooltip={!hasCompletedFirstDiscovery && hasEmail
									? "We'll email you when your scan is complete."
									: ''}
								use:tooltip
							>
								<div class="flex items-center">
									<Radar class="mr-2 h-4 w-4 animate-pulse [animation-duration:5s]" />
									{#if activeSession.progress > 0}
										<span class="text-xs">{activeSession.progress}%</span>
									{/if}
								</div>
								{#if estimate != null}
									<span class="text-[10px]">~{Math.round(estimate / 60)} min left</span>
								{:else}
									<span class="text-[10px]">Estimating...</span>
								{/if}
							</div>
							<div class="card-divider-v self-stretch"></div>
						{/if}
					{/if}
				{/if}

				{#if networksData.length > 0}
					<RichSelect
						label=""
						selectedValue={$selectedNetworkId ?? ''}
						displayComponent={NetworkDisplay}
						onSelect={handleNetworkChange}
						options={networksData}
					/>

					<div class="card-divider-v self-stretch"></div>

					<RichSelect
						label=""
						selectedValue={$selectedSnapshotId ?? LIVE_VIEW_SENTINEL}
						displayComponent={snapshotDisplayWithLive}
						onSelect={handleSnapshotChange}
						options={snapshotOptions}
						fitToContent
					/>

					{#if !isReadOnly && $selectedSnapshotId == null}
						<button
							class="btn-secondary"
							onclick={handleTakeSnapshot}
							disabled={takeSnapshotMutation.isPending || !$selectedNetworkId}
							aria-label={topology_takeSnapshot()}
							data-tooltip={topology_takeSnapshot()}
							use:tooltip
						>
							<Camera class="h-5 w-5" />
							{#if !snapshotsEnabled}
								<Tag label="Upgrade" color="Yellow" />
							{/if}
						</button>
					{/if}

					{#if !isReadOnly}
						{#if $selectedSnapshotId}
							<button
								class="btn-danger"
								onclick={handleDeleteSnapshot}
								disabled={deleteSnapshotMutation.isPending}
								title={common_delete()}
							>
								<Trash2 class="my-1 h-5 w-5" />
							</button>
						{/if}
					{/if}

					<div class="card-divider-v self-stretch"></div>

					<div bind:this={viewSwitcherEl}>
						<RichSelect
							label=""
							selectedValue={$activeView}
							displayComponent={SimpleOptionDisplay}
							onSelect={handleViewChange}
							options={viewOptions}
							minWidth="22rem"
						/>
					</div>
				{/if}
			</div>

			{#if $showViewSwitcherHint && viewSwitcherEl}
				<ViewSwitcherHint anchor={viewSwitcherEl} />
			{/if}

			{#if isLoading}
				<Loading />
			{:else if currentTopology}
				<div class="relative" id="topology-view-area">
					<TopologyOptionsPanel
						topology={currentTopology}
						tutorialTopology={$showDependencyTutorial ? TUTORIAL_TOPOLOGY : undefined}
						{isReadOnly}
						onClearSelection={$showDependencyTutorial
							? dismissDependencyTutorial
							: clearMultiSelect}
						onGroupCreated={() => {
							clearMultiSelect();
							updateTopologyOptions((opts) => ({
								...opts,
								local: {
									...opts.local,
									hide_edge_types: opts.local.hide_edge_types.filter(
										(e) => e !== 'RequestPath' && e !== 'HubAndSpoke'
									)
								}
							}));
						}}
						onDependencyTypeChange={$showDependencyTutorial
							? () => (tutorialTypeToggled = true)
							: undefined}
					/>
					<TopologyViewer bind:this={topologyViewer} topology={currentTopology} {isActive} />
					{#if $showDependencyTutorial}
						<DependencyTutorial
							onDismiss={dismissDependencyTutorial}
							dependencyTypeToggled={tutorialTypeToggled}
						/>
					{/if}
					{#if showAppWizard}
						<ApplicationSetupWizard
							{appTags}
							networkId={currentTopology.network_id}
							onComplete={handleWizardComplete}
						/>
					{/if}
					{#if showL2EmptyState}
						<L2EmptyStateOverlay
							hasSnmpCredential={onboarding.includes('FirstSnmpCredentialCreated')}
						/>
					{/if}
				</div>
			{:else}
				<div class="card card-static text-secondary">
					{topology_noTopologySelected()}
				</div>
			{/if}
		</div>

		{#if currentTopology}
			<ExportModal topologyId={currentTopology.id} bind:isOpen={isExportModalOpen} />
		{/if}
	{/if}
</SvelteFlowProvider>

{#if currentTopology}
	<SharesModal
		name="topology-share"
		topologyDisplayName={currentTopology.name}
		isOpen={isShareModalOpen}
		topologyId={currentTopology.id}
		networkId={currentTopology.network_id}
		isSnapshotView={$selectedSnapshotId != null}
		onClose={() => (isShareModalOpen = false)}
	/>
{/if}
