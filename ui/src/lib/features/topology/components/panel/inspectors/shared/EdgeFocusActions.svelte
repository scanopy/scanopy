<script lang="ts">
	import type { Edge } from '@xyflow/svelte';
	import { useSvelteFlow } from '@xyflow/svelte';
	import { Crosshair } from '@lucide/svelte';
	import type { TopologyEdge } from '#lib/features/topology/types/base.ts';
	import { focusEdge, focusEdgeEnd } from '#lib/features/topology/viewport-fit.ts';
	import {
		common_both,
		common_source,
		common_target,
		topology_focusLink,
		topology_focusSource,
		topology_focusTarget
	} from '#lib/paraglide/messages.js';

	/**
	 * Fits the view to one end of the selected edge, or to all of its ends, for every edge type.
	 * An end is whatever the edge is drawn to: an element card, a container, or the collapsed
	 * container standing in for its members.
	 */
	let { edge, originals = [] }: { edge: Edge; originals?: readonly TopologyEdge[] } = $props();

	const flow = useSvelteFlow();

	const buttonClass = 'btn-secondary gap-1 rounded-sm px-1.5 py-0 text-xs font-medium';
</script>

<div class="flex items-center justify-end gap-1">
	<button
		type="button"
		class={buttonClass}
		onclick={() => focusEdgeEnd(flow, edge, 'source')}
		title={topology_focusSource()}
	>
		<Crosshair class="h-3 w-3" />
		{common_source()}
	</button>
	<button
		type="button"
		class={buttonClass}
		onclick={() => focusEdgeEnd(flow, edge, 'target')}
		title={topology_focusTarget()}
	>
		<Crosshair class="h-3 w-3" />
		{common_target()}
	</button>
	<button
		type="button"
		class={buttonClass}
		onclick={() => focusEdge(flow, edge, originals)}
		title={topology_focusLink()}
	>
		<Crosshair class="h-3 w-3" />
		{common_both()}
	</button>
</div>
