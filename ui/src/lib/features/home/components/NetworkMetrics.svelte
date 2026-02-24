<script lang="ts">
	import type { components } from '$lib/api/schema';
	import NetworkCard from './NetworkCard.svelte';

	type NetworkSummary = components['schemas']['NetworkSummary'];

	let {
		networks
	}: {
		networks: NetworkSummary[];
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
