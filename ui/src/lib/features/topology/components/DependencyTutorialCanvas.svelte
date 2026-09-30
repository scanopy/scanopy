<script lang="ts">
	import { SvelteFlow, Background, BackgroundVariant } from '@xyflow/svelte';
	import type { Node } from '@xyflow/svelte';
	import ElementNode from './visualization/ElementNode.svelte';
	import CustomEdge from './visualization/CustomEdge.svelte';
	import { TUTORIAL_XYFLOW_NODES } from './dependency-tutorial-data';
	import { useRevealedPreviewEdges } from '../preview-reveal.svelte';

	let {
		onnodeclick
	}: {
		onnodeclick: (args: { node: Node; event: MouseEvent | TouchEvent }) => void;
	} = $props();

	const nodeTypes = { Element: ElementNode };
	const edgeTypes = { custom: CustomEdge };
	const nodes = [...TUTORIAL_XYFLOW_NODES];

	// Called here rather than in `DependencyTutorial` so it runs inside the tutorial's own
	// `SvelteFlowProvider` and measures this canvas's nodes.
	const revealedPreview = useRevealedPreviewEdges();
</script>

<SvelteFlow
	{nodes}
	edges={revealedPreview.current}
	{nodeTypes}
	{edgeTypes}
	{onnodeclick}
	fitView={true}
	fitViewOptions={{ padding: 0.3 }}
	minZoom={0.5}
	maxZoom={1.5}
	nodesDraggable={false}
	nodesConnectable={false}
	elementsSelectable={true}
	selectionOnDrag={false}
	panOnDrag={true}
	zoomOnScroll={false}
	zoomOnDoubleClick={false}
>
	<Background
		variant={BackgroundVariant.Dots}
		bgColor="var(--color-topology-bg)"
		gap={50}
		size={1}
	/>
</SvelteFlow>
