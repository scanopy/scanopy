<script lang="ts">
	import { et } from '$lib/shared/utils/embed-i18n';
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
	<h3 class="text-primary mb-3 text-base font-semibold">{et('Networks')}</h3>
	<div class="grid grid-cols-[repeat(auto-fill,minmax(360px,1fr))] gap-4">
		{#each networks as network (network.id)}
			<NetworkCard {network} />
		{/each}
	</div>
</section>
