<script lang="ts">
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';
	import TopologyViewer from './visualization/TopologyViewer.svelte';
	import TopologyOptionsPanel from './panel/TopologyOptionsPanel.svelte';
	import { Edit, Lock, Plus, Radar, Radio, RefreshCcw, Share2, Trash2 } from 'lucide-svelte';
	import ExportButton from './ExportButton.svelte';
	import ExportModal from './ExportModal.svelte';
	import ShareModal from '$lib/features/shares/components/ShareModal.svelte';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import { SvelteFlowProvider } from '@xyflow/svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import {
		useTopologiesQuery,
		useDeleteTopologyMutation,
		useRebuildTopologyMutation,
		useLockTopologyMutation,
		useUnlockTopologyMutation,
		autoRebuild,
		hasConflicts,
		selectedTopologyId,
		selectedNodes,
		consumePreferredNetwork
	} from '../queries';
	import type { Topology } from '../types/base';
	import TopologyModal from './TopologyModal.svelte';
	import { newNodeIds } from '../interactions';
	import { getTopologyState } from '../state';
	import StateBadge from './StateBadge.svelte';
	import InlineDanger from '$lib/shared/components/feedback/InlineDanger.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import RefreshConflictsModal from './RefreshConflictsModal.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import { TopologyDisplay } from '$lib/shared/components/forms/selection/display/TopologyDisplay.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { useActiveSessionsQuery } from '$lib/features/discovery/queries';
	import { useGroupsQuery } from '$lib/features/groups/queries';
	import { useUsersQuery } from '$lib/features/users/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import type { components } from '$lib/api/schema';
	import { entities, permissions } from '$lib/shared/stores/metadata';
	import { modalState, openModal } from '$lib/shared/stores/modal-registry';
	import type { TabProps } from '$lib/shared/types';
	import {
		common_auto,
		common_lock,
		common_manual,
		common_unlock,
		topology_anotherUser,
		topology_confirmDelete,
		topology_conflictsDetected,
		topology_conflictsDetectedBody,
		topology_lastRebuild,
		topology_locked,
		topology_lockedInfoBody,
		topology_lockedTimestamp,
		topology_noTopologySelected,
		topology_staleData,
		topology_staleDataBody
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

	// Queries - TanStack Query handles deduplication
	const subnetsQuery = useSubnetsQuery();
	const groupsQuery = useGroupsQuery();
	const usersQuery = useUsersQuery({ enabled: () => canViewUsers });
	const topologiesQuery = useTopologiesQuery();
	const organizationQuery = useOrganizationQuery();
	const activeSessionsQuery = useActiveSessionsQuery();
	const configQuery = useConfigQuery();

	// Derived data
	let usersData = $derived(usersQuery.data ?? []);
	let topologiesData = $derived(topologiesQuery.data ?? []);
	let isLoading = $derived(
		subnetsQuery.isPending || groupsQuery.isPending || topologiesQuery.isPending
	);

	let hasEmail = $derived(configQuery.data?.has_email_service ?? false);

	// Selected topology (derived from ID + query data)
	let currentTopology = $derived(
		$selectedTopologyId ? topologiesData.find((t) => t.id === $selectedTopologyId) : null
	);

	// Find active discovery session for current topology's network
	let activeSession = $derived(
		currentTopology
			? (activeSessionsQuery.data ?? []).find((s) => s.network_id === currentTopology.network_id)
			: null
	);
	let discoveryColor = $derived(entities.getColorHelper('Discovery'));

	type OnboardingOperation = components['schemas']['OnboardingOperation'];
	let onboarding = $derived((organizationQuery.data?.onboarding ?? []) as OnboardingOperation[]);
	let hasCompletedFirstDiscovery = $derived(
		onboarding.length === 0 || onboarding.includes('FirstDiscoveryCompleted')
	);
	let hasCompletedFirstRebuild = $derived(
		onboarding.length === 0 || onboarding.includes('FirstDiscoveryCompleted')
	);

	// Mutations
	const deleteTopologyMutation = useDeleteTopologyMutation();
	const rebuildTopologyMutation = useRebuildTopologyMutation();
	const lockTopologyMutation = useLockTopologyMutation();
	const unlockTopologyMutation = useUnlockTopologyMutation();

	// Initialize selected topology when data loads
	$effect(() => {
		if (topologiesData.length > 0 && !$selectedTopologyId) {
			// Check for preferred network from onboarding
			const preferredNetworkId = consumePreferredNetwork();
			if (preferredNetworkId) {
				const preferred = topologiesData.find((t) => t.network_id === preferredNetworkId);
				if (preferred) {
					selectedTopologyId.set(preferred.id);
					return;
				}
			}
			// Default to first topology
			selectedTopologyId.set(topologiesData[0].id);
		}
	});

	// New node highlight: snapshot pre-rebuild node IDs, detect new ones after rebuild
	let preRebuildNodeIds: Set<string> | null = $state(null);

	function snapshotBeforeRebuild() {
		if (currentTopology) {
			preRebuildNodeIds = new Set(currentTopology.nodes.map((n) => n.id));
		}
	}

	// Detect new nodes after rebuild completes
	$effect(() => {
		if (!preRebuildNodeIds || !currentTopology || currentTopology.is_stale) return;

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

	let isCreateEditOpen = $state(false);
	let editingTopology: Topology | null = $state(null);

	// Deep-link: open topology editor from URL
	$effect(() => {
		if ($modalState.name === 'topology-editor' && !isCreateEditOpen) {
			if ($modalState.id) {
				const entity = topologiesData.find((e) => e.id === $modalState.id);
				if (entity) {
					editingTopology = entity;
					isCreateEditOpen = true;
				}
			} else {
				editingTopology = null;
				isCreateEditOpen = true;
			}
		}
	});

	let isRefreshConflictsOpen = $state(false);
	let isShareModalOpen = $state(false);
	let isExportModalOpen = $state(false);

	// Deep-link: open share modal from modal registry
	$effect(() => {
		if ($modalState.name === 'topology-share' && !isShareModalOpen) {
			isShareModalOpen = true;
		}
	});

	let topologyViewer: TopologyViewer | null = $state(null);

	// Track which topologies have had their initial auto-rebuild check
	let initialRebuildChecked = new SvelteSet<string>();
	let onboardingRebuildChecked = new SvelteSet<string>();
	let topologyViewTracked = new SvelteSet<string>();

	// Track topology_viewed once per topology when tab is active and data is loaded
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

	// One-time rebuild for onboarding: triggers when FirstTopologyRebuild milestone is missing,
	// regardless of autoRebuild setting, so the checklist item clears on first topology visit.
	// Must run before auto-rebuild so it can trigger unconditionally.
	// Guard on isActive because all tabs are mounted simultaneously (hidden via CSS).
	$effect(() => {
		if (!isActive) {
			return;
		}

		if (!currentTopology || currentTopology.is_locked) {
			return;
		}

		if (onboardingRebuildChecked.has(currentTopology.id)) {
			return;
		}

		if (onboarding.includes('FirstTopologyRebuild')) {
			return;
		}

		// Don't auto-rebuild in manual mode — user controls when rebuilds happen
		if (!$autoRebuild) {
			return;
		}

		onboardingRebuildChecked.add(currentTopology.id);
		// Also mark auto-rebuild as done to avoid a redundant second rebuild
		initialRebuildChecked.add(currentTopology.id);

		void rebuildTopologyMutation.mutateAsync(currentTopology).then(() => {
			topologyViewer?.triggerFitView();
		});
	});

	// Auto-rebuild on initial load when autoRebuild is enabled
	$effect(() => {
		// Guard: need a selected, non-locked topology with autoRebuild enabled
		if (!currentTopology || currentTopology.is_locked || !$autoRebuild) {
			return;
		}

		// Guard: already checked this topology (or handled by onboarding rebuild)
		if (initialRebuildChecked.has(currentTopology.id)) {
			return;
		}

		// Mark as checked BEFORE triggering rebuild to prevent race conditions
		initialRebuildChecked.add(currentTopology.id);

		// Determine if rebuild is needed: stale or empty
		const needsRebuild = currentTopology.is_stale || currentTopology.nodes.length === 0;

		if (needsRebuild) {
			void rebuildTopologyMutation.mutateAsync(currentTopology).then(() => {
				topologyViewer?.triggerFitView();
			});
		}
	});

	function handleCreateTopology() {
		isCreateEditOpen = true;
		editingTopology = null;
	}

	function handleEditTopology() {
		isCreateEditOpen = true;
		editingTopology = currentTopology ?? null;
	}

	function onSubmit() {
		isCreateEditOpen = false;
		editingTopology = null;
	}

	function onClose() {
		isCreateEditOpen = false;
		editingTopology = null;
	}

	function clearMultiSelect() {
		selectedNodes.set([]);
	}

	// Handle topology selection
	function handleTopologyChange(value: string) {
		selectedTopologyId.set(value);
		selectedNodes.set([]);
	}

	async function handleDelete() {
		if (!currentTopology) return;
		// Capture values before async operation (currentTopology becomes null after query refetch)
		const toDeleteId = currentTopology.id;
		const toDeleteName = currentTopology.name;
		if (confirm(topology_confirmDelete({ name: toDeleteName }))) {
			await deleteTopologyMutation.mutateAsync(toDeleteId);
			// After mutation, topologiesData is already updated without the deleted topology
			if (topologiesData.length > 0) {
				selectedTopologyId.set(topologiesData[0].id);
			} else {
				selectedTopologyId.set(null);
			}
		}
	}

	async function handleAutoRebuildToggle() {
		autoRebuild.set(!$autoRebuild);
		if ($autoRebuild) {
			await handleRefresh();
		}
	}

	async function handleRefresh() {
		if (!currentTopology) return;

		if (hasConflicts(currentTopology)) {
			// Open modal to review conflicts
			isRefreshConflictsOpen = true;
		} else {
			// Safe to refresh directly
			snapshotBeforeRebuild();
			await rebuildTopologyMutation.mutateAsync(currentTopology);
			topologyViewer?.triggerFitView();
		}
	}

	async function handleReset() {
		if (!currentTopology) return;
		snapshotBeforeRebuild();
		let resetTopology = { ...currentTopology };
		resetTopology.nodes = [];
		resetTopology.edges = [];
		await rebuildTopologyMutation.mutateAsync(resetTopology);
		topologyViewer?.triggerFitView();
	}

	async function handleConfirmRefresh() {
		if (!currentTopology) return;
		snapshotBeforeRebuild();
		await rebuildTopologyMutation.mutateAsync(currentTopology);
		topologyViewer?.triggerFitView();
		isRefreshConflictsOpen = false;
	}

	async function handleLockFromConflicts() {
		if (!currentTopology) return;
		await lockTopologyMutation.mutateAsync(currentTopology);
		isRefreshConflictsOpen = false;
	}

	async function handleToggleLock() {
		if (!currentTopology) return;
		if (currentTopology.is_locked) {
			await unlockTopologyMutation.mutateAsync(currentTopology);
		} else {
			await lockTopologyMutation.mutateAsync(currentTopology);
		}
	}

	let stateConfig = $derived(
		currentTopology
			? getTopologyState(currentTopology, $autoRebuild, {
					onRefresh: handleRefresh,
					onReset: handleReset
				})
			: null
	);

	let lockedByUser = $derived(
		currentTopology?.locked_by ? usersData.find((u) => u.id === currentTopology.locked_by) : null
	);
	let lockedByDisplay = $derived(
		lockedByUser?.email ?? (currentTopology?.locked_by ? topology_anotherUser() : null)
	);
</script>

<SvelteFlowProvider>
	{#if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title="Install a daemon to start mapping your network topology." />
	{:else}
		<div class="space-y-6">
			<!-- Header -->
			<div class="card card-static flex items-center justify-evenly gap-4 px-4 py-2">
				{#if currentTopology}
					<div class="flex items-center gap-4 py-2">
						<ExportButton onclick={() => (isExportModalOpen = true)} />
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
									onclick={() => openModal('topology-share')}
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
								<div class="flex items-center p-2">
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
							{#if hasCompletedFirstRebuild}
								<div class="card-divider-v self-stretch"></div>
							{/if}
						{/if}
						{#if hasCompletedFirstRebuild}
							<div class="flex items-center">
								<div class="mx-2 flex flex-col text-center">
									<div class="flex justify-around gap-6">
										<button
											onclick={handleToggleLock}
											class={`text-xs ${currentTopology.is_locked ? 'btn-icon-info' : 'btn-icon'}`}
											data-tooltip="Lock your topology layout to prevent discovery from changing it"
											use:tooltip
											title={currentTopology.is_locked ? 'Unlock' : 'Lock'}
										>
											<Lock class="mr-2 h-4 w-4" />
											{currentTopology.is_locked ? common_unlock() : common_lock()}
										</button>

										{#if !currentTopology.is_locked}
											<button
												onclick={handleAutoRebuildToggle}
												type="button"
												class={`text-xs ${$autoRebuild && !currentTopology.is_locked ? 'btn-icon-success' : 'btn-icon'}`}
												disabled={currentTopology.is_locked}
												data-tooltip={$autoRebuild
													? 'Your topology rebuilds automatically when new data arrives from discovery'
													: "In manual mode, your topology won't update until you trigger a rebuild"}
												use:tooltip
												title={$autoRebuild ? 'Auto' : 'Manual'}
											>
												{#if $autoRebuild}
													<Radio class="mr-2 h-4 w-4" /> {common_auto()}
												{:else}
													<RefreshCcw class="mr-2 h-4 w-4" /> {common_manual()}
												{/if}
											</button>
										{/if}
									</div>
									{#if currentTopology.is_locked && currentTopology.locked_at}
										<span class="text-tertiary whitespace-nowrap text-[10px]"
											>{topology_lockedTimestamp({
												timestamp: formatTimestamp(currentTopology.locked_at),
												user: lockedByDisplay ?? ''
											})}</span
										>
									{:else}
										<span class="text-tertiary whitespace-nowrap text-[10px]"
											>{topology_lastRebuild({
												timestamp: formatTimestamp(currentTopology.last_refreshed)
											})}</span
										>
									{/if}
								</div>
								<!-- State Badge / Action Button -->
								{#if stateConfig && !currentTopology.is_locked && !$autoRebuild}
									<div class="flex flex-col items-center gap-2">
										<div class="flex items-center">
											<StateBadge
												disabled={stateConfig?.disabled || false}
												Icon={stateConfig.icon}
												label={stateConfig.buttonText}
												cls={stateConfig.class}
												onClick={stateConfig.action}
											/>
										</div>
									</div>
								{/if}
							</div>

							<div class="card-divider-v self-stretch"></div>
						{/if}
					{/if}

					{#if hasCompletedFirstRebuild && topologiesData.length > 0}
						<RichSelect
							label=""
							selectedValue={currentTopology.id}
							displayComponent={TopologyDisplay}
							onSelect={handleTopologyChange}
							options={topologiesData}
						/>
					{/if}
				{/if}

				{#if !isReadOnly && hasCompletedFirstRebuild}
					{#if currentTopology}
						<div class="card-divider-v self-stretch"></div>
					{/if}

					<div class="flex items-center gap-4 py-2">
						{#if currentTopology}
							<button class="btn-primary" onclick={handleEditTopology}>
								<Edit class="my-1 h-4 w-4" />
							</button>
						{/if}

						<button class="btn-primary" onclick={handleCreateTopology}>
							<Plus class="my-1 h-4 w-4" />
						</button>

						{#if currentTopology}
							<button class="btn-danger" onclick={handleDelete}>
								<Trash2 class="my-1 h-5 w-5" />
							</button>
						{/if}
					</div>
				{/if}
			</div>

			<!-- Contextual Info Banner -->
			{#if currentTopology && stateConfig}
				{#if stateConfig.type === 'locked'}
					<InlineInfo
						dismissableKey="topology-locked-info"
						title={topology_locked()}
						body={topology_lockedInfoBody()}
					/>
				{:else if stateConfig.type === 'stale_conflicts'}
					<InlineDanger
						dismissableKey="topology-conflict-info"
						title={topology_conflictsDetected()}
						body={topology_conflictsDetectedBody()}
					/>
				{:else if stateConfig.type === 'stale_safe'}
					<InlineWarning
						dismissableKey="topology-refresh-info"
						title={topology_staleData()}
						body={topology_staleDataBody()}
					/>
				{/if}
			{/if}

			{#if isLoading}
				<Loading />
			{:else if currentTopology}
				<div class="relative">
					<TopologyOptionsPanel
						{isReadOnly}
						onClearSelection={clearMultiSelect}
						onGroupCreated={() => {
							clearMultiSelect();
							handleRefresh();
						}}
					/>
					<TopologyViewer
						bind:this={topologyViewer}
						onToggleLock={handleToggleLock}
						onRebuild={handleRefresh}
						{isActive}
					/>
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

<TopologyModal
	name="topology-editor"
	bind:isOpen={isCreateEditOpen}
	{onSubmit}
	{onClose}
	topo={editingTopology}
/>

{#if currentTopology}
	<RefreshConflictsModal
		bind:isOpen={isRefreshConflictsOpen}
		topology={currentTopology}
		onConfirm={handleConfirmRefresh}
		onLock={handleLockFromConflicts}
		onCancel={() => (isRefreshConflictsOpen = false)}
	/>
	<ShareModal
		name="topology-share"
		isOpen={isShareModalOpen}
		topologyId={currentTopology.id}
		networkId={currentTopology.network_id}
		onClose={() => (isShareModalOpen = false)}
	/>
{/if}
