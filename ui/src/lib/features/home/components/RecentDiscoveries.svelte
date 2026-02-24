<script lang="ts">
	import type { Discovery } from '$lib/features/discovery/types/base';
	import HomeDiscoveryDisplay from './HomeDiscoveryDisplay.svelte';

	let { discoveries }: { discoveries: Discovery[] } = $props();
</script>

<section>
	<h3 class="text-primary mb-3 text-base font-semibold">Recent Discoveries</h3>
	{#if discoveries.length === 0}
		<p class="text-tertiary text-sm">No discovery results yet.</p>
	{:else}
		<div class="sm:w-1/2">
			<div class="space-y-2">
				{#each discoveries as discovery (discovery.id)}
					<div class="card card-static">
						<HomeDiscoveryDisplay item={discovery} />
					</div>
				{/each}
			</div>
		</div>
	{/if}
</section>
