<script lang="ts">
	import { getContext, setContext } from 'svelte';
	import type { Writable } from 'svelte/store';
	import type { Node, Edge } from '@xyflow/svelte';
	import { ChevronLeft, ChevronRight, Info } from '@lucide/svelte';
	import InspectorNode from '#lib/features/topology/components/panel/inspectors/InspectorNode.svelte';
	import InspectorEdge from '#lib/features/topology/components/panel/inspectors/InspectorEdge.svelte';
	import {
		optionsPanelExpanded,
		MINIMAP_FITVIEW_BOTTOM_PX
	} from '#lib/features/topology/queries.ts';
	import { topology_collapsePanel, topology_expandPanel } from '#lib/paraglide/messages.js';

	interface Props {
		showMinimap?: boolean;
	}
	let { showMinimap = false }: Props = $props();

	setContext('staticTags', true);

	// Get selected node/edge from context (set by ReadOnlyTopologyViewer)
	const selectedNode = getContext<Writable<Node | null>>('selectedNode');
	const selectedEdge = getContext<Writable<Edge | null>>('selectedEdge');

	let expanded = $state(false);

	function setExpanded(value: boolean) {
		expanded = value;
		// Sync to global store so BaseTopologyViewer auto-fits viewport
		optionsPanelExpanded.set(value);
	}

	// Automatically expand when something is selected
	$effect(() => {
		if ($selectedNode || $selectedEdge) {
			setExpanded(true);
		}
	});
</script>

<!-- Floating Panel -->
<div
	class="topology-options absolute top-4 left-4 z-10 duration-300 {expanded ? 'w-80' : 'w-auto'}"
>
	<div class="card p-0 shadow-lg">
		{#if expanded}
			<!-- Header with title and collapse button -->
			<div class="flex items-center border-b border-gray-700">
				<!-- Collapse button -->
				<button
					class="btn-icon rounded-xl p-3"
					onclick={() => setExpanded(false)}
					aria-label={topology_collapsePanel()}
				>
					<ChevronLeft class="h-5 w-5 text-secondary" />
				</button>
				<!-- Title -->
				<div class="flex-1 px-4 py-3 text-sm font-medium text-primary">
					<Info class="mr-1 inline h-4 w-4" />
					Inspector
				</div>
			</div>

			<!-- Content -->
			<div
				class="overflow-y-auto p-3"
				style="max-height: calc(100vh - {showMinimap ? MINIMAP_FITVIEW_BOTTOM_PX + 20 : 180}px);"
			>
				{#if $selectedNode}
					{#key $selectedNode.id}
						<InspectorNode node={$selectedNode} />
					{/key}
				{:else if $selectedEdge}
					{#key $selectedEdge.id}
						<InspectorEdge edge={$selectedEdge} />
					{/key}
				{:else}
					<div class="py-8 text-center text-sm text-tertiary">
						Click on a node or edge to inspect it
					</div>
				{/if}
			</div>
		{:else}
			<!-- Collapsed toggle button -->
			<button
				class="btn-icon rounded-2xl p-3"
				onclick={() => setExpanded(true)}
				aria-label={topology_expandPanel()}
			>
				<ChevronRight class="h-5 w-5 text-secondary" />
			</button>
		{/if}
	</div>
</div>
