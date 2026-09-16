<script lang="ts">
	/**
	 * The active view has nothing left to draw and the user's own filters are why.
	 *
	 * View-agnostic: every view ships with `Service/Category = [OpenPorts]` hidden and L2 adds
	 * `Interface/LinkState = [Unlinked]`, so any of the four can filter itself to nothing. The
	 * alternative is the view's setup prompt claiming discovery found nothing, which sent two
	 * separate investigations after data that was there the whole time.
	 */
	import { Funnel } from 'lucide-svelte';
	import TopologyOverlay from './TopologyOverlay.svelte';
	import type { ActiveFilterSummary } from '../interactions';
	import { showEverythingIn, activeView } from '../queries';
	import {
		topology_emptyByFilterTitle,
		topology_emptyByFilterDescription,
		topology_emptyByFilterShowAll,
		topology_nHidden
	} from '$lib/paraglide/messages';

	let {
		filters,
		viewName,
		isReadOnly = false
	}: {
		filters: ActiveFilterSummary[];
		viewName: string;
		/** A snapshot or share cannot persist an options change, so offer no button it can't honour. */
		isReadOnly?: boolean;
	} = $props();
</script>

<TopologyOverlay title={topology_emptyByFilterTitle()} size="sm" offsetForPanel>
	<div class="flex flex-col items-center gap-4 p-6 text-center">
		<div class="rounded-full bg-amber-500/10 p-3">
			<Funnel class="h-8 w-8 text-amber-500" />
		</div>
		<p class="text-secondary text-sm">
			{topology_emptyByFilterDescription({ viewName })}
		</p>

		<ul class="border-primary/10 w-full border-t pt-3 text-left">
			{#each filters as filter (filter.label)}
				<li class="flex items-baseline justify-between gap-3 py-1 text-xs">
					<span>
						<span class="font-medium">{filter.label}</span>
						{#if filter.values.length}
							<span class="text-secondary">{filter.values.join(', ')}</span>
						{/if}
					</span>
					{#if filter.count}
						<span class="text-secondary/70 shrink-0"
							>{topology_nHidden({ count: filter.count })}</span
						>
					{/if}
				</li>
			{/each}
		</ul>

		{#if !isReadOnly}
			<button class="btn btn-sm btn-primary" onclick={() => showEverythingIn($activeView)}>
				{topology_emptyByFilterShowAll()}
			</button>
		{/if}
	</div>
</TopologyOverlay>
