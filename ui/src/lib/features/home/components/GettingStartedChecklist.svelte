<script lang="ts">
	import type { components } from '$lib/api/schema';
	import { Check, Circle, Info, Loader2, ExternalLink } from '@lucide/svelte';
	import ChecklistItem from '$lib/shared/components/data/ChecklistItem.svelte';
	import { openModal } from '$lib/shared/stores/modal-registry';
	import { onMount } from 'svelte';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { queryKeys } from '$lib/api/query-client';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import confetti from 'canvas-confetti';
	import { daemonSetupState } from '$lib/features/daemons/stores/daemon-setup';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useActiveSessionsQuery } from '$lib/features/discovery/queries';
	import DaemonTroubleshootingModal from '$lib/shared/components/layout/DaemonTroubleshootingModal.svelte';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import DiscoveryEstimation from '$lib/features/discovery/components/DiscoveryEstimation.svelte';
	import { isFullScan } from '$lib/features/discovery/utils/fullScan';
	import StallTag from '$lib/features/discovery/components/StallTag.svelte';
	import {
		CHECKLIST_STEPS,
		isStepComplete as checkStepComplete,
		isStepEnabled as checkStepEnabled,
		getCompletedCount,
		isAllComplete,
		executeStepAction,
		trackChecklistStepClicked,
		type ChecklistStep
	} from '$lib/shared/onboarding/checklist';
	import {
		common_dismiss,
		gettingStarted_allSet,
		gettingStarted_allSetDescription,
		gettingStarted_complete,
		gettingStarted_emailNotification,
		gettingStarted_funCompanyReddit,
		gettingStarted_funCompanyRestart,
		gettingStarted_funDaydream,
		gettingStarted_funFallbackCoffee,
		gettingStarted_funFallbackStretch,
		gettingStarted_funHomelabReddit,
		gettingStarted_funHomelabUpgrade,
		gettingStarted_funMspReddit,
		gettingStarted_funMspScope,
		gettingStarted_havingTrouble,
		gettingStarted_inviteTeamMembers,
		gettingStarted_scanningNetwork,
		gettingStarted_setupInterfaceDiscovery,
		gettingStarted_createTags,
		gettingStarted_title,
		gettingStarted_whileYouWait
	} from '$lib/paraglide/messages';

	type SetupItemType = components['schemas']['SetupItemDiscriminants'];

	let {
		setup,
		organization,
		onNavigate,
		onDismiss,
		isActive = false
	}: {
		setup: SetupItemType[];
		organization: {
			use_case?: string | null;
			plan?: { included_seats?: number | null; seat_cents?: number | null } | null;
		};
		onNavigate: (tab: string) => void;
		onDismiss?: () => void;
		isActive: boolean;
	} = $props();

	const DISMISS_KEY = 'home-checklist-dismissed';

	let dismissed = $state(false);
	let showCelebration = $state(false);
	let celebrationDone = $state(false);
	let showTroubleshootingModal = $state(false);

	// Subscribe to daemon setup state
	let daemonStatus = $state<'idle' | 'waiting' | 'connected' | 'trouble'>('idle');
	const unsubscribe = daemonSetupState.subscribe((s) => {
		daemonStatus = s.connectionStatus;
	});

	// Poll org query while daemon is waiting/trouble
	const organizationQuery = useOrganizationQuery();
	const queryClient = useQueryClient();

	// Active sessions query
	const sessionsQuery = useActiveSessionsQuery();

	const configQuery = useConfigQuery();
	let hasEmail = $derived(configQuery.data?.has_email_service ?? false);

	let activeScanSession = $derived(
		(sessionsQuery.data ?? []).find(
			(s) => s.discovery_type?.type === 'Network' || s.discovery_type?.type === 'Unified'
		)
	);
	let isDiscoveryActive = $derived(!!activeScanSession);

	// Fun suggestion localStorage state
	let funChecked = $state<Record<string, boolean>>({});

	onMount(() => {
		dismissed = localStorage.getItem(DISMISS_KEY) === 'true';

		// Load fun item states from localStorage
		for (let i = 0; i < localStorage.length; i++) {
			const key = localStorage.key(i);
			if (key?.startsWith('waiting-fun:')) {
				funChecked[key.replace('waiting-fun:', '')] = true;
			}
		}

		return unsubscribe;
	});

	// Clear fun item states when discovery completes
	$effect(() => {
		if (setup.includes('RunFirstScan')) {
			for (let i = localStorage.length - 1; i >= 0; i--) {
				const key = localStorage.key(i);
				if (key?.startsWith('waiting-fun:')) {
					localStorage.removeItem(key);
				}
			}
			funChecked = {};
		}
	});

	$effect(() => {
		if (daemonStatus === 'waiting' || daemonStatus === 'trouble') {
			const interval = setInterval(() => {
				organizationQuery.refetch();
			}, 5000);
			return () => clearInterval(interval);
		}
	});

	// Detect connection via setup prop updates
	$effect(() => {
		if (
			(daemonStatus === 'waiting' || daemonStatus === 'trouble') &&
			setup.includes('InstallDaemon')
		) {
			daemonSetupState.set({ connectionStatus: 'connected' });
			// The Daemons table reads the list pages, which still hold the daemon as never seen.
			queryClient.invalidateQueries({ queryKey: queryKeys.daemons.all });
		}
	});

	$effect(() => {
		if (allComplete && !dismissed && !celebrationDone && isActive) {
			showCelebration = true;
			confetti({ particleCount: 100, spread: 70, origin: { y: 0.6 } });
			setTimeout(() => {
				showCelebration = false;
				celebrationDone = true;
				localStorage.setItem(DISMISS_KEY, 'true');
			}, 4000);
		}
	});

	const steps = CHECKLIST_STEPS;

	let completedCount = $derived(getCompletedCount(setup));

	let allComplete = $derived(isAllComplete(setup));

	function handleStepClick(step: ChecklistStep) {
		trackChecklistStepClicked(step.id, 'home');
		executeStepAction(step, onNavigate);
	}

	let showDaemonTroubleTag = $derived(
		(daemonStatus === 'waiting' || daemonStatus === 'trouble') && !setup.includes('InstallDaemon')
	);

	function handleTroubleTagClick(e: MouseEvent) {
		e.stopPropagation();
		showTroubleshootingModal = true;
		trackEvent('checklist_trouble_tag_clicked');
	}

	function dismiss() {
		trackEvent('checklist_dismissed', { completed_count: completedCount });
		localStorage.setItem(DISMISS_KEY, 'true');
		dismissed = true;
		onDismiss?.();
	}

	// Waiting suggestions — show all applicable, mark completed ones
	function getInAppSuggestions(): Array<{ label: string; action: () => void; completed: boolean }> {
		const suggestions: Array<{ label: string; action: () => void; completed: boolean }> = [];
		suggestions.push({
			label: gettingStarted_setupInterfaceDiscovery(),
			action: () => {
				onNavigate('credentials');
				openModal('credential-editor');
			},
			completed: setup.includes('InterfaceCredential')
		});
		suggestions.push({
			label: gettingStarted_createTags(),
			action: () => {
				onNavigate('tags');
				openModal('tag-editor');
			},
			completed: setup.includes('Tags')
		});
		if (
			organization?.plan &&
			((organization.plan.included_seats ?? 0) > 1 || (organization.plan.seat_cents ?? 0) > 0)
		) {
			suggestions.push({
				label: gettingStarted_inviteTeamMembers(),
				action: () => {
					onNavigate('users');
					openModal('invite-user');
				},
				completed: setup.includes('InviteTeam')
			});
		}
		return suggestions;
	}

	function getFunSuggestions(): Array<{ id: string; label: string; url?: string }> {
		const useCase = organization?.use_case;
		if (useCase === 'homelab') {
			return [
				{
					id: 'homelab-reddit',
					label: gettingStarted_funHomelabReddit(),
					url: 'https://reddit.com/r/homelab'
				},
				{ id: 'homelab-upgrade', label: gettingStarted_funHomelabUpgrade() },
				{ id: 'homelab-daydream', label: gettingStarted_funDaydream() }
			];
		}
		if (useCase === 'msp') {
			return [
				{
					id: 'msp-reddit',
					label: gettingStarted_funMspReddit(),
					url: 'https://reddit.com/r/msp'
				},
				{ id: 'msp-scope', label: gettingStarted_funMspScope() },
				{ id: 'msp-daydream', label: gettingStarted_funDaydream() }
			];
		}
		if (useCase === 'internal_it') {
			return [
				{
					id: 'company-reddit',
					label: gettingStarted_funCompanyReddit(),
					url: 'https://reddit.com/r/sysadmin'
				},
				{ id: 'company-restart', label: gettingStarted_funCompanyRestart() },
				{ id: 'company-daydream', label: gettingStarted_funDaydream() }
			];
		}
		return [
			{ id: 'fallback-coffee', label: gettingStarted_funFallbackCoffee() },
			{ id: 'fallback-stretch', label: gettingStarted_funFallbackStretch() },
			{ id: 'fallback-daydream', label: gettingStarted_funDaydream() }
		];
	}

	function toggleFunItem(id: string) {
		if (funChecked[id]) {
			delete funChecked[id];
			localStorage.removeItem(`waiting-fun:${id}`);
		} else {
			funChecked[id] = true;
			localStorage.setItem(`waiting-fun:${id}`, 'true');
		}
		funChecked = { ...funChecked };
	}

	let inAppSuggestions = $derived(getInAppSuggestions());
	let funSuggestions = $derived(getFunSuggestions());
</script>

{#if showCelebration}
	<section>
		<div
			class="rounded-lg border border-green-300 bg-green-50 p-6 text-center dark:border-green-600/30 dark:bg-green-900/20"
		>
			<h3 class="text-base font-semibold text-primary">{gettingStarted_allSet()}</h3>
			<p class="mt-1 text-sm text-secondary">
				{gettingStarted_allSetDescription()}
			</p>
		</div>
	</section>
{:else if !allComplete && !dismissed}
	<section>
		<div class="card">
			<div class="mb-3 flex items-center justify-between">
				<h3 class="text-base font-semibold text-primary">{gettingStarted_title()}</h3>
				<div class="flex items-center gap-3">
					<span class="text-sm text-tertiary"
						>{gettingStarted_complete({
							completed: String(completedCount),
							total: String(steps.length)
						})}</span
					>
					{#if completedCount > 0}
						<button
							onclick={dismiss}
							class="text-sm text-tertiary transition-colors hover:text-secondary"
						>
							{common_dismiss()}
						</button>
					{/if}
				</div>
			</div>

			<div class="space-y-2">
				{#each steps as step (step.id)}
					{@const complete = checkStepComplete(step, setup)}
					{@const enabled = checkStepEnabled(step, setup)}
					{@const isAccountStep = step.id === 'account'}
					{@const isActiveDiscoveryStep = step.id === 'discovery' && isDiscoveryActive && !complete}
					<ChecklistItem
						checked={complete}
						disabled={complete || !enabled || isAccountStep}
						onToggle={() => handleStepClick(step)}
						label={isActiveDiscoveryStep ? gettingStarted_scanningNetwork() : step.label()}
					>
						{#snippet icon()}
							{#if complete}
								<Check class="h-5 w-5 shrink-0 text-green-400" />
							{:else if isActiveDiscoveryStep}
								<Loader2 class="h-5 w-5 shrink-0 animate-spin text-blue-500" />
							{:else}
								<Circle class="h-5 w-5 shrink-0 {enabled ? 'text-tertiary' : 'text-disabled'}" />
							{/if}
						{/snippet}
						{#snippet labelExtra()}
							{#if step.id === 'daemon' && showDaemonTroubleTag}
								<!-- svelte-ignore a11y_click_events_have_key_events -->
								<!-- svelte-ignore a11y_no_static_element_interactions -->
								<span
									class="inline-flex cursor-pointer items-center gap-1 rounded-sm bg-yellow-100 px-2 py-0.5 text-xs font-medium text-yellow-700 transition-colors hover:bg-yellow-200 dark:bg-yellow-900/30 dark:text-yellow-400 dark:hover:bg-yellow-900/50"
									onclick={handleTroubleTagClick}
								>
									<Info class="h-3 w-3" />
									{gettingStarted_havingTrouble()}
								</span>
							{/if}
						{/snippet}
						{#snippet subContent()}
							{#if !complete && enabled && !isAccountStep && !isActiveDiscoveryStep}
								<p class="text-xs text-tertiary">{step.description()}</p>
							{:else if isActiveDiscoveryStep && activeScanSession}
								<div class="mt-0.5 flex items-center gap-2">
									<DiscoveryEstimation
										phase={activeScanSession.phase}
										hosts_discovered={activeScanSession.hosts_discovered}
										estimated_remaining_secs={activeScanSession.estimated_remaining_secs}
										is_full_scan={isFullScan(activeScanSession.discovery_type)}
									/>
									<StallTag
										session_id={activeScanSession.session_id}
										phase={activeScanSession.phase}
									/>
								</div>
							{/if}
						{/snippet}
						{#snippet detail()}
							{#if isActiveDiscoveryStep}
								{#if hasEmail}
									<p class="mt-1 text-xs text-secondary">
										{gettingStarted_emailNotification()}
									</p>
								{/if}

								<p class="mt-2 mb-1 text-xs font-medium text-tertiary">
									{gettingStarted_whileYouWait()}
								</p>

								<!-- In-app suggestions -->
								{#each inAppSuggestions as suggestion (suggestion.label)}
									<button
										class="flex w-full items-center gap-2 rounded-sm px-2 py-1 text-left text-xs text-blue-600 transition-colors hover:bg-blue-50 dark:text-blue-400 dark:hover:bg-blue-900/20"
										onclick={() => suggestion.action()}
										disabled={suggestion.completed}
									>
										{#if suggestion.completed}
											<Check class="h-3 w-3 shrink-0 text-green-400" />
										{:else}
											<Circle class="h-3 w-3 shrink-0" />
										{/if}
										<span class:line-through={suggestion.completed}>{suggestion.label}</span>
									</button>
								{/each}

								<!-- Fun suggestions -->
								{#each funSuggestions as item (item.id)}
									<button
										class="flex w-full items-center gap-2 rounded-sm px-2 py-1 text-left text-xs text-blue-600 transition-colors hover:bg-blue-50 dark:text-blue-400 dark:hover:bg-blue-900/20"
										onclick={() => {
											if (item.url) {
												window.open(item.url, '_blank', 'noopener,noreferrer');
											}
											toggleFunItem(item.id);
										}}
									>
										{#if funChecked[item.id]}
											<Check class="h-3 w-3 shrink-0 text-green-400" />
										{:else}
											<Circle class="h-3 w-3 shrink-0" />
										{/if}
										<span class:line-through={funChecked[item.id]}>{item.label}</span>
										{#if item.url}
											<ExternalLink class="h-3 w-3 shrink-0 opacity-50" />
										{/if}
									</button>
								{/each}
							{/if}
						{/snippet}
					</ChecklistItem>
				{/each}
			</div>
		</div>
	</section>
{/if}

<DaemonTroubleshootingModal
	isOpen={showTroubleshootingModal}
	onClose={() => (showTroubleshootingModal = false)}
/>
