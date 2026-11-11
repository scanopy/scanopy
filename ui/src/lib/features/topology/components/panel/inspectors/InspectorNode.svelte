<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import InspectorElementNode from './nodes/InspectorElementNode.svelte';
	import InspectorContainerNode from './nodes/InspectorContainerNode.svelte';
	import { inspector_nodeDetailsUnavailable } from '#lib/paraglide/messages.js';

	let { node }: { node: Node } = $props();

	let isElementNode = $derived(node.type === 'Element');
	let isContainerNode = $derived(node.type === 'Container');
</script>

<div class="w-full space-y-4">
	{#if isElementNode}
		<InspectorElementNode {node} />
	{:else if isContainerNode}
		<InspectorContainerNode {node} />
	{:else}
		<div class="space-y-3">
			<p class="text-sm text-tertiary">{inspector_nodeDetailsUnavailable()}</p>
		</div>
	{/if}
</div>
