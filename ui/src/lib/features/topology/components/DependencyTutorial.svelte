<script lang="ts">
	import { SvelteFlow, SvelteFlowProvider, Background, BackgroundVariant } from '@xyflow/svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import { writable } from 'svelte/store';
	import { setContext } from 'svelte';
	import '@xyflow/svelte/dist/style.css';
	import './visualization/topology-viewer.css';
	import TopologyOverlay from './TopologyOverlay.svelte';
	import ChecklistItem from '$lib/shared/components/data/ChecklistItem.svelte';
	import ElementNode from './visualization/ElementNode.svelte';
	import CustomEdge from './visualization/CustomEdge.svelte';
	import { selectedNodes, previewEdges } from '../queries';
	import { dependencyTypes } from '$lib/shared/stores/metadata';
	import { browser } from '$app/environment';
	import {
		TUTORIAL_SERVICES,
		TUTORIAL_TOPOLOGY,
		TUTORIAL_XYFLOW_NODES
	} from './dependency-tutorial-data';
	import type { Node, Edge } from '@xyflow/svelte';
	import {
		topology_tutorialTitle,
		topology_tutorialStep1,
		topology_tutorialStep2,
		topology_tutorialStep3,
		topology_tutorialStep4,
		topology_tutorialSkip,
		topology_tutorialExplainer
	} from '$lib/paraglide/messages';

	let {
		onDismiss,
		dependencyTypeToggled = false
	}: {
		onDismiss: () => void;
		dependencyTypeToggled?: boolean;
	} = $props();

	const modifier = browser && navigator.platform.includes('Mac') ? '⌘' : 'Ctrl';

	// Provide tutorial topology via context so ElementNode resolves services
	const topologyStore = writable(TUTORIAL_TOPOLOGY);
	setContext('topology', topologyStore);

	// Also provide isolated selection context so ElementNode doesn't highlight from global state
	const localSelectedNode = writable<Node | null>(null);
	const localSelectedEdge = writable<Edge | null>(null);
	setContext('selectedNode', localSelectedNode);
	setContext('selectedEdge', localSelectedEdge);

	// Node/edge types for mini SvelteFlow
	const nodeTypes = { Element: ElementNode };
	const edgeTypes = { custom: CustomEdge };

	// Xyflow stores — must be writable stores (same as BaseTopologyViewer)
	const tutorialNodes = writable<Node[]>([...TUTORIAL_XYFLOW_NODES]);
	const tutorialEdges = writable<Edge[]>([]);
	previewEdges.subscribe((value) => {
		tutorialEdges.set(value);
	});

	// Track clicked nodes
	let clickedNodeIds = $state(new Set<string>());

	function handleNodeClick({ node }: { node: Node; event: MouseEvent | TouchEvent }) {
		if (clickedNodeIds.has(node.id)) return;
		const updated = new SvelteSet(clickedNodeIds);
		updated.add(node.id);
		clickedNodeIds = updated;

		const xyNode = TUTORIAL_XYFLOW_NODES.find((n) => n.id === node.id);
		if (xyNode) {
			selectedNodes.set([...$selectedNodes, xyNode]);
		}
	}

	let step1Done = $derived(clickedNodeIds.size >= 1);
	let step2Done = $derived(clickedNodeIds.size >= 3);
	let step3Done = $derived(dependencyTypeToggled);

	const RequestPathIcon = dependencyTypes.getIconComponent('RequestPath');
	const HubAndSpokeIcon = dependencyTypes.getIconComponent('HubAndSpoke');
	let selectionDots = $derived(TUTORIAL_SERVICES.map((n) => clickedNodeIds.has(n.id)));
</script>

<div class="tutorial-flow-border">
	<TopologyOverlay title={topology_tutorialTitle()} size="lg" fixedHeight offsetForPanel>
		<div class="flex min-h-0 flex-1 flex-col">
			<!-- Explainer -->
			<div class="flex-shrink-0 px-6 py-3">
				<p class="text-secondary text-sm">{topology_tutorialExplainer()}</p>
			</div>
			<!-- Mini SvelteFlow canvas with real ElementNode + edge rendering -->
			<div class="min-h-0 flex-1">
				<SvelteFlowProvider>
					<SvelteFlow
						nodes={$tutorialNodes}
						edges={$tutorialEdges}
						{nodeTypes}
						{edgeTypes}
						onnodeclick={handleNodeClick}
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
				</SvelteFlowProvider>
			</div>

			<!-- Checklist below the canvas -->
			<div class="flex-shrink-0 p-4">
				<div class="mb-2 flex items-center gap-0.5">
					{#each selectionDots as filled, i (i)}
						<span
							class="inline-block h-1.5 w-1.5 rounded-full {filled
								? 'bg-green-400'
								: 'bg-gray-300 dark:bg-gray-600'}"
						></span>
					{/each}
				</div>

				<div class="space-y-0">
					<ChecklistItem
						checked={step1Done}
						disabled={step1Done}
						label={topology_tutorialStep1()}
					/>
					<ChecklistItem
						checked={step2Done}
						disabled={step2Done}
						label={topology_tutorialStep2({ modifier })}
					/>
					<ChecklistItem
						checked={step3Done}
						disabled={step3Done || !step2Done}
						label={topology_tutorialStep3()}
					>
						{#snippet labelExtra()}
							<RequestPathIcon class="h-3.5 w-3.5" />
							<HubAndSpokeIcon class="h-3.5 w-3.5" />
						{/snippet}
					</ChecklistItem>
					<ChecklistItem checked={false} disabled={!step3Done} label={topology_tutorialStep4()} />
				</div>

				<div class="mt-3 text-center">
					<button class="text-secondary hover:text-primary text-xs underline" onclick={onDismiss}>
						{topology_tutorialSkip()}
					</button>
				</div>
			</div>
		</div>
	</TopologyOverlay>
</div>

<style>
	/* The tutorial embeds a live SvelteFlow inside the modal; the viewer's own border reads as a
	   frame around the miniature and does not belong there. */
	.tutorial-flow-border :global(.svelte-flow) {
		border: none;
	}
</style>
