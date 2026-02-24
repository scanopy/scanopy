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
</script>

<section>
	<h3 class="text-primary mb-3 text-base font-semibold">Networks</h3>
	<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
		{#each networks as network (network.id)}
			<NetworkCard {network} />
		{/each}
	</div>
</section>
