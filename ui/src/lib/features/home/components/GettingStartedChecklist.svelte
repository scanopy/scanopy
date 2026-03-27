<script lang="ts">
	import type { components } from '$lib/api/schema';
	import { Check, Circle, Info, Loader2, ExternalLink } from 'lucide-svelte';
	import { openModal } from '$lib/shared/stores/modal-registry';
	import { onMount } from 'svelte';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import confetti from 'canvas-confetti';
	import { daemonSetupState } from '$lib/features/daemons/stores/daemon-setup';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useActiveSessionsQuery } from '$lib/features/discovery/queries';
	import DaemonTroubleshootingModal from '$lib/shared/components/layout/DaemonTroubleshootingModal.svelte';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import DiscoveryEstimation from '$lib/features/discovery/components/DiscoveryEstimation.svelte';
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

	type OnboardingOperation = components['schemas']['OnboardingOperation'];

	let {
		onboarding,
		organization,
		onNavigate,
		isActive = false
	}: {
		onboarding: OnboardingOperation[];
		organization: {
			use_case?: string | null;
			plan?: { included_seats?: number | null; seat_cents?: number | null } | null;
		};
		onNavigate: (tab: string) => void;
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

	// Active sessions query
	const sessionsQuery = useActiveSessionsQuery();

	const configQuery = useConfigQuery();
	let hasEmail = $derived(configQuery.data?.has_email_service ?? false);

	let activeNetworkSession = $derived(
		(sessionsQuery.data ?? []).find(
			(s) => s.discovery_type?.type === 'Network' || s.discovery_type?.type === 'Unified'
		)
	);
	let isDiscoveryActive = $derived(!!activeNetworkSession);

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
		if (onboarding.includes('FirstDiscoveryCompleted')) {
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

	// Detect connection via onboarding prop updates
	$effect(() => {
		if (
			(daemonStatus === 'waiting' || daemonStatus === 'trouble') &&
			onboarding.includes('FirstDaemonRegistered')
		) {
			daemonSetupState.set({ connectionStatus: 'connected' });
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

	let completedCount = $derived(getCompletedCount(onboarding));

	let allComplete = $derived(isAllComplete(onboarding));

	function handleStepClick(step: ChecklistStep) {
		trackChecklistStepClicked(step.id, 'home');
		executeStepAction(step, onNavigate);
	}

	let showDaemonTroubleTag = $derived(
		(daemonStatus === 'waiting' || daemonStatus === 'trouble') &&
			!onboarding.includes('FirstDaemonRegistered')
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
	}

	// Waiting suggestions — show all applicable, mark completed ones
	function getInAppSuggestions(): Array<{ label: string; action: () => void; completed: boolean }> {
		const suggestions: Array<{ label: string; action: () => void; completed: boolean }> = [];
		suggestions.push({
			label: 'Set up SNMP credentials',
			action: () => {
				onNavigate('credentials');
				openModal('credential-editor');
			},
			completed: onboarding.includes('FirstSnmpCredentialCreated')
		});
		suggestions.push({
			label: 'Create tags to organize hosts',
			action: () => {
				onNavigate('tags');
				openModal('tag-editor');
			},
			completed: onboarding.includes('FirstTagCreated')
		});
		if (
			organization?.plan &&
			((organization.plan.included_seats ?? 0) > 1 || (organization.plan.seat_cents ?? 0) > 0)
		) {
			suggestions.push({
				label: 'Invite team members',
				action: () => {
					onNavigate('users');
					openModal('invite-user');
				},
				completed: onboarding.includes('InviteSent')
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
					label: 'Browse r/homelab for inspiration',
					url: 'https://reddit.com/r/homelab'
				},
				{ id: 'homelab-upgrade', label: 'Convince yourself you do need another Raspberry Pi' },
				{ id: 'homelab-daydream', label: 'Daydream about a fully documented network' }
			];
		}
		if (useCase === 'msp') {
			return [
				{
					id: 'msp-reddit',
					label: 'Commiserate on r/msp for a few minutes',
					url: 'https://reddit.com/r/msp'
				},
				{ id: 'msp-scope', label: "Practice saying 'that's out of scope' in the mirror" },
				{ id: 'msp-daydream', label: 'Daydream about a fully documented network' }
			];
		}
		if (useCase === 'company') {
			return [
				{
					id: 'company-reddit',
					label: "Confirm you're not alone on r/sysadmin",
					url: 'https://reddit.com/r/sysadmin'
				},
				{ id: 'company-restart', label: "Rehearse your 'have you tried restarting it?' delivery" },
				{ id: 'company-daydream', label: 'Daydream about a fully documented network' }
			];
		}
		return [
			{ id: 'fallback-coffee', label: 'Grab a coffee' },
			{ id: 'fallback-stretch', label: 'Take a quick stretch break' },
			{ id: 'fallback-daydream', label: 'Daydream about a fully documented network' }
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
			<h3 class="text-primary text-base font-semibold">You're all set!</h3>
			<p class="text-secondary mt-1 text-sm">
				Your network is mapped. Explore your topology and discover what Scanopy can do.
			</p>
		</div>
	</section>
{:else if !allComplete && !dismissed}
	<section>
		<div class="card">
			<div class="mb-3 flex items-center justify-between">
				<h3 class="text-primary text-base font-semibold">Getting Started</h3>
				<div class="flex items-center gap-3">
					<span class="text-tertiary text-sm">{completedCount} of {steps.length} complete</span>
					{#if completedCount > 0}
						<button
							onclick={dismiss}
							class="text-tertiary hover:text-secondary text-sm transition-colors"
						>
							Dismiss
						</button>
					{/if}
				</div>
			</div>

			<div class="space-y-2">
				{#each steps as step (step.id)}
					{@const complete = checkStepComplete(step, onboarding)}
					{@const enabled = checkStepEnabled(step, onboarding)}
					{@const isAccountStep = step.id === 'account'}
					{@const isActiveDiscoveryStep = step.id === 'discovery' && isDiscoveryActive && !complete}
					<div>
						<button
							class="flex w-full items-center justify-between rounded-lg px-3 py-2 text-left transition-colors {!complete &&
							!isAccountStep &&
							enabled
								? 'bg-gray-100 hover:bg-gray-200 dark:bg-gray-800/50 dark:hover:bg-gray-700/50'
								: ''} {!enabled ? 'opacity-50' : ''}"
							disabled={complete || !enabled || isAccountStep}
							onclick={() => handleStepClick(step)}
						>
							<div class="flex items-center gap-3">
								{#if complete}
									<Check class="h-5 w-5 flex-shrink-0 text-green-400" />
								{:else if isActiveDiscoveryStep}
									<Loader2 class="h-5 w-5 flex-shrink-0 animate-spin text-blue-500" />
								{:else}
									<Circle
										class="h-5 w-5 flex-shrink-0 {enabled ? 'text-tertiary' : 'text-disabled'}"
									/>
								{/if}
								<div>
									<div class="flex items-center gap-2">
										<span
											class="text-sm font-medium"
											class:text-primary={!complete && enabled}
											class:text-tertiary={complete}
											class:text-disabled={!complete && !enabled}
											class:line-through={complete}
										>
											{isActiveDiscoveryStep ? 'Scanning your network' : step.label}
										</span>
										{#if step.id === 'daemon' && showDaemonTroubleTag}
											<!-- svelte-ignore a11y_click_events_have_key_events -->
											<!-- svelte-ignore a11y_no_static_element_interactions -->
											<span
												class="inline-flex cursor-pointer items-center gap-1 rounded bg-yellow-100 px-2 py-0.5 text-xs font-medium text-yellow-700 transition-colors hover:bg-yellow-200 dark:bg-yellow-900/30 dark:text-yellow-400 dark:hover:bg-yellow-900/50"
												onclick={handleTroubleTagClick}
											>
												<Info class="h-3 w-3" />
												Having trouble?
											</span>
										{/if}
									</div>
									{#if isActiveDiscoveryStep && activeNetworkSession}
										<DiscoveryEstimation
											phase={activeNetworkSession.phase}
											hosts_discovered={activeNetworkSession.hosts_discovered}
											estimated_remaining_secs={activeNetworkSession.estimated_remaining_secs}
											class="mt-0.5"
										/>
									{:else if !complete && enabled && !isAccountStep}
										<p class="text-tertiary text-xs">{step.description}</p>
									{/if}
								</div>
							</div>
						</button>

						<!-- Waiting suggestions shown below discovery step when active -->
						{#if isActiveDiscoveryStep}
							<div class="ml-11 mt-1 space-y-1">
								{#if hasEmail}
									<p class="text-secondary mt-1 text-xs">
										We'll email you when your scan is complete — feel free to leave and come back
										later.
									</p>
								{/if}

								<p class="text-tertiary mb-1 mt-2 text-xs font-medium">While you wait:</p>

								<!-- In-app suggestions -->
								{#each inAppSuggestions as suggestion (suggestion.label)}
									<button
										class="flex w-full items-center gap-2 rounded px-2 py-1 text-left text-xs text-blue-600 transition-colors hover:bg-blue-50 dark:text-blue-400 dark:hover:bg-blue-900/20"
										onclick={() => suggestion.action()}
										disabled={suggestion.completed}
									>
										{#if suggestion.completed}
											<Check class="h-3 w-3 flex-shrink-0 text-green-400" />
										{:else}
											<Circle class="h-3 w-3 flex-shrink-0" />
										{/if}
										<span class:line-through={suggestion.completed}>{suggestion.label}</span>
									</button>
								{/each}

								<!-- Fun suggestions -->
								{#each funSuggestions as item (item.id)}
									<button
										class="flex w-full items-center gap-2 rounded px-2 py-1 text-left text-xs text-blue-600 transition-colors hover:bg-blue-50 dark:text-blue-400 dark:hover:bg-blue-900/20"
										onclick={() => {
											if (item.url) {
												window.open(item.url, '_blank', 'noopener,noreferrer');
											}
											toggleFunItem(item.id);
										}}
									>
										{#if funChecked[item.id]}
											<Check class="h-3 w-3 flex-shrink-0 text-green-400" />
										{:else}
											<Circle class="h-3 w-3 flex-shrink-0" />
										{/if}
										<span class:line-through={funChecked[item.id]}>{item.label}</span>
										{#if item.url}
											<ExternalLink class="h-3 w-3 flex-shrink-0 opacity-50" />
										{/if}
									</button>
								{/each}
							</div>
						{/if}
					</div>
				{/each}
			</div>
		</div>
	</section>
{/if}

<DaemonTroubleshootingModal
	isOpen={showTroubleshootingModal}
	onClose={() => (showTroubleshootingModal = false)}
/>
