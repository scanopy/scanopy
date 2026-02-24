<script lang="ts">
	import type { components } from '$lib/api/schema';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import { openModal } from '$lib/shared/stores/modal-registry';
	import { trackEvent } from '$lib/shared/utils/analytics';

	type PlanUsage = components['schemas']['PlanUsage'];

	let {
		planUsage,
		planType,
		isOwner
	}: {
		planUsage: PlanUsage;
		planType: string | null;
		isOwner: boolean;
	} = $props();

	// Only show for plans with limits
	let hasLimits = $derived(
		planUsage.host_limit != null || planUsage.network_limit != null || planUsage.seat_limit != null
	);

	let warnings = $derived.by(() => {
		const list: string[] = [];
		if (planUsage.host_limit != null) {
			const pct = planUsage.host_count / planUsage.host_limit;
			if (pct >= 1)
				list.push(`Host limit reached (${planUsage.host_count}/${planUsage.host_limit})`);
			else if (pct >= 0.8)
				list.push(`Approaching host limit (${planUsage.host_count}/${planUsage.host_limit})`);
		}
		if (planUsage.network_limit != null) {
			const pct = planUsage.network_count / planUsage.network_limit;
			if (pct >= 1)
				list.push(`Network limit reached (${planUsage.network_count}/${planUsage.network_limit})`);
			else if (pct >= 0.8)
				list.push(
					`Approaching network limit (${planUsage.network_count}/${planUsage.network_limit})`
				);
		}
		if (planUsage.seat_limit != null) {
			const pct = planUsage.seat_count / planUsage.seat_limit;
			if (pct >= 1)
				list.push(`Seat limit reached (${planUsage.seat_count}/${planUsage.seat_limit})`);
			else if (pct >= 0.8)
				list.push(`Approaching seat limit (${planUsage.seat_count}/${planUsage.seat_limit})`);
		}
		return list;
	});

	let showUpgrade = $derived(warnings.length > 0 && isOwner && planType === 'Free');
</script>

{#if hasLimits && warnings.length > 0}
	<section>
		<h3 class="text-primary mb-3 text-base font-semibold">Plan Usage</h3>
		<div class="space-y-2">
			{#each warnings as warning, i (i)}
				<InlineWarning title={warning} />
			{/each}
		</div>
		{#if showUpgrade}
			<button
				class="mt-3 rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-blue-500"
				onclick={() => {
					trackEvent('upgrade_button_clicked', { feature: 'home_plan_usage' });
					openModal('billing-plan');
				}}
			>
				Upgrade Plan
			</button>
		{/if}
	</section>
{/if}
