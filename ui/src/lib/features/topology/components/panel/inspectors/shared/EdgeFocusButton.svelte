<script lang="ts">
	import type { Edge } from '@xyflow/svelte';
	import { useSvelteFlow } from '@xyflow/svelte';
	import { Crosshair } from '@lucide/svelte';
	import type { TopologyEdge } from '#lib/features/topology/types/base.ts';
	import { focusEdge } from '#lib/features/topology/viewport-fit.ts';
	import { topology_focusLink } from '#lib/paraglide/messages.js';

	/** Fits the view to every end of the selected edge, for every edge type. */
	let { edge, originals = [] }: { edge: Edge; originals?: readonly TopologyEdge[] } = $props();

	const flow = useSvelteFlow();
</script>

<button
	type="button"
	class="btn-icon p-0.5"
	onclick={() => focusEdge(flow, edge, originals)}
	title={topology_focusLink()}
>
	<Crosshair class="h-3.5 w-3.5" />
</button>
