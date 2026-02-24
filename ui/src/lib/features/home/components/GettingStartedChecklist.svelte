<script lang="ts">
	import type { components } from '$lib/api/schema';
	import { Check, Circle } from 'lucide-svelte';
	import { onMount } from 'svelte';

	type TelemetryOperation = components['schemas']['TelemetryOperation'];

	let {
		onboarding,
		onNavigate
	}: {
		onboarding: TelemetryOperation[];
		onNavigate: (tab: string) => void;
	} = $props();

	const DISMISS_KEY = 'home-checklist-dismissed';

	let dismissed = $state(false);

	onMount(() => {
		dismissed = localStorage.getItem(DISMISS_KEY) === 'true';
	});

	interface ChecklistStep {
		id: string;
		milestone: TelemetryOperation;
		label: string;
		description: string;
		tab: string;
		actionLabel: string;
	}

	const steps: ChecklistStep[] = [
		{
			id: 'daemon',
			milestone: 'FirstDaemonRegistered',
			label: 'Add a Daemon',
			description: 'Install a daemon to start discovering your network.',
			tab: 'daemons',
			actionLabel: 'Go to Daemons'
		},
		{
			id: 'discovery',
			milestone: 'FirstDiscoveryCompleted',
			label: 'Run a Discovery',
			description: 'Discover hosts, services, and subnets on your network.',
			tab: 'discovery-sessions',
			actionLabel: 'Go to Discovery'
		},
		{
			id: 'topology',
			milestone: 'FirstTopologyRebuild',
			label: 'View your Topology',
			description: 'See your network visualized as an interactive map.',
			tab: 'topology',
			actionLabel: 'Go to Topology'
		}
	];

	let completedCount = $derived(steps.filter((s) => onboarding.includes(s.milestone)).length);

	let allComplete = $derived(completedCount === steps.length);

	function dismiss() {
		localStorage.setItem(DISMISS_KEY, 'true');
		dismissed = true;
	}
</script>

{#if !dismissed}
	<section>
		<div class="rounded-lg border border-blue-600/30 bg-blue-900/20 p-4">
			<div class="mb-3 flex items-center justify-between">
				<h3 class="text-primary text-base font-semibold">
					{#if allComplete}
						Setup complete
					{:else}
						Getting Started
					{/if}
				</h3>
				<div class="flex items-center gap-3">
					<span class="text-tertiary text-sm">{completedCount} of {steps.length} complete</span>
					{#if allComplete}
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
					{@const isComplete = onboarding.includes(step.milestone)}
					<div
						class="flex items-center justify-between rounded-lg px-3 py-2 {!isComplete
							? 'bg-gray-800/50'
							: ''}"
					>
						<div class="flex items-center gap-3">
							{#if isComplete}
								<Check class="h-5 w-5 flex-shrink-0 text-green-400" />
							{:else}
								<Circle class="text-tertiary h-5 w-5 flex-shrink-0" />
							{/if}
							<div>
								<span
									class="text-sm font-medium"
									class:text-primary={!isComplete}
									class:text-tertiary={isComplete}
									class:line-through={isComplete}
								>
									{step.label}
								</span>
								{#if !isComplete}
									<p class="text-tertiary text-xs">{step.description}</p>
								{/if}
							</div>
						</div>
						{#if !isComplete}
							<button
								onclick={() => onNavigate(step.tab)}
								class="text-sm font-medium text-blue-400 transition-colors hover:text-blue-300"
							>
								{step.actionLabel}
							</button>
						{/if}
					</div>
				{/each}
			</div>
		</div>
	</section>
{/if}
