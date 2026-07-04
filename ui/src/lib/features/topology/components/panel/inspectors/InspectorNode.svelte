<script lang="ts">
	import { et } from '$lib/shared/utils/embed-i18n';
	import type { Node } from '@xyflow/svelte';
	import InspectorElementNode from './nodes/InspectorElementNode.svelte';
	import InspectorContainerNode from './nodes/InspectorContainerNode.svelte';

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
			<p class="text-tertiary text-sm">{et('Unable to display node details')}</p>
		</div>
	{/if}
</div>
