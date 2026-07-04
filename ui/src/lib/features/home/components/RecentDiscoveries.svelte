<script lang="ts">
	import { et } from '$lib/shared/utils/embed-i18n';
	import type { Discovery } from '$lib/features/discovery/types/base';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import type { components } from '$lib/api/schema';
	import HomeDiscoveryDisplay from './HomeDiscoveryDisplay.svelte';
	import { home_noDiscoveriesYet } from '$lib/paraglide/messages';

	type NetworkSummary = components['schemas']['NetworkSummary'];

	let {
		discoveries,
		daemons = [],
		networks = [],
		onNavigate
	}: {
		discoveries: Discovery[];
		daemons?: Daemon[];
		networks?: NetworkSummary[];
		onNavigate?: (discovery: Discovery) => void;
	} = $props();
</script>

<section>
	<h3 class="text-primary mb-3 text-base font-semibold">{et('Recent Discoveries')}</h3>
	{#if discoveries.length === 0}
		<p class="text-tertiary text-sm">{home_noDiscoveriesYet()}</p>
	{:else}
		<div class="grid grid-cols-[repeat(auto-fill,minmax(360px,1fr))] gap-4">
			{#each discoveries as discovery (discovery.id)}
				<button
					class="card card-static cursor-pointer text-left"
					onclick={() => onNavigate?.(discovery)}
				>
					<HomeDiscoveryDisplay item={discovery} context={{ daemons, networks }} />
				</button>
			{/each}
		</div>
	{/if}
</section>
