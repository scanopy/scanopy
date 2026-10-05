<script lang="ts">
	import type { components } from '$lib/api/schema';
	import SiteCard from './SiteCard.svelte';
	import { common_sites } from '$lib/paraglide/messages';

	type SiteSummary = components['schemas']['SiteSummary'];

	let {
		sites
	}: {
		sites: SiteSummary[];
	} = $props();
</script>

<section>
	<h3 class="text-primary mb-3 text-base font-semibold">{common_sites()}</h3>
	<div class="grid grid-cols-[repeat(auto-fill,minmax(360px,1fr))] gap-4">
		{#each sites as site (site.id)}
			<SiteCard {site} />
		{/each}
	</div>
</section>
