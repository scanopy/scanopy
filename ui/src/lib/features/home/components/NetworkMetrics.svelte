<script lang="ts">
	import type { components } from '$lib/api/schema';
	import NetworkCard from './NetworkCard.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';

	type NetworkSummary = components['schemas']['NetworkSummary'];
	type PlanUsage = components['schemas']['PlanUsage'];

	let {
		networks,
		planUsage
	}: {
		networks: NetworkSummary[];
		planUsage: PlanUsage;
	} = $props();

	let hostLimitWarning = $derived.by(() => {
		if (planUsage.host_limit == null) return null;
		const pct = planUsage.host_count / planUsage.host_limit;
		if (pct >= 1) return `Host limit reached (${planUsage.host_count}/${planUsage.host_limit})`;
		if (pct >= 0.8)
			return `Approaching host limit (${planUsage.host_count}/${planUsage.host_limit})`;
		return null;
	});
</script>

<section>
	<h3 class="text-primary mb-3 text-base font-semibold">Networks</h3>
	{#if hostLimitWarning}
		<div class="mb-3">
			<InlineWarning title={hostLimitWarning} />
		</div>
	{/if}
	<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
		{#each networks as network (network.id)}
			<NetworkCard {network} />
		{/each}
	</div>
</section>
