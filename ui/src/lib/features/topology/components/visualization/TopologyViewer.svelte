<script lang="ts">
	import { type Node, type Edge, type Connection } from '@xyflow/svelte';
	import {
		topologyReadOnly,
		selectedEdge,
		selectedNode,
		selectedNodes
		// Layout-override mutations are DISABLED — overrides aren't persisted
		// (no save mechanism); re-import useUpdateNodePositionMutation /
		// useUpdateEdgeHandlesMutation (and `activeView` for their payload) here
		// when reviving.
	} from '../../queries';
	import { type TopologyEdge, type RenderableTopology } from '../../types/base';
	import { searchOpen } from '../../interactions';
	import { editModeEnabled } from '../../state';
	import { createTopologyKeydownHandler } from '../../keyboard';
	import BaseTopologyViewer from './BaseTopologyViewer.svelte';
	import SearchOverlay from './SearchOverlay.svelte';
	import ShortcutsHelpOverlay from './ShortcutsHelpOverlay.svelte';
	import { onDestroy } from 'svelte';

	// Props for callbacks from parent
	let {
		topology,
		onToggleLock,
		onRebuild,
		isActive = false
	}: {
		topology: RenderableTopology | null | undefined;
		onToggleLock?: () => void;
		onRebuild?: () => void;
		isActive?: boolean;
	} = $props();

	// Layout-override mutations are DISABLED — node position / edge handle
	// changes are no longer persisted (the graph builds on request and ELK
	// re-lays out every render, so there's no mechanism to save them). Kept
	// commented for revival:
	// const updateNodePositionMutation = useUpdateNodePositionMutation();
	// const updateEdgeHandlesMutation = useUpdateEdgeHandlesMutation();

	let baseViewer: BaseTopologyViewer | null = $state(null);

	// Overlay state
	let shortcutsHelpOpen = $state(false);

	// Edit mode is disabled: topology editing (drag/resize/reconnect) is turned
	// off product-wide. `editMode` is permanently false — the edit toggle (button
	// + hotkey) is unwired below — but the state and read-only resets are kept so
	// `editModeEnabled` stays a coherent (always-false) signal for consumers.
	let editMode = $state(false);

	// Force view mode whenever the topology becomes read-only (e.g. selecting a
	// snapshot while in edit mode).
	$effect(() => {
		if ($topologyReadOnly && editMode) {
			editMode = false;
			editModeEnabled.set(false);
		}
	});

	// Sidebar buttons show labels briefly on first visit per session, then stay collapsed
	const SIDEBAR_SEEN_KEY = 'topology_sidebar_labels_shown';
	const alreadySeen =
		typeof sessionStorage !== 'undefined' && sessionStorage.getItem(SIDEBAR_SEEN_KEY) === '1';
	let sidebarCollapsed = $state(alreadySeen);

	$effect(() => {
		if (isActive && !alreadySeen && !sidebarCollapsed) {
			const timer = setTimeout(() => {
				sidebarCollapsed = true;
				sessionStorage.setItem(SIDEBAR_SEEN_KEY, '1');
			}, 2000);
			return () => clearTimeout(timer);
		}
	});

	// Reset edit mode when leaving this tab (tabs stay mounted, just hidden)
	$effect(() => {
		if (!isActive && editMode) {
			editMode = false;
			editModeEnabled.set(false);
		}
	});

	onDestroy(() => {
		editModeEnabled.set(false);
	});

	export function triggerFitView() {
		baseViewer?.triggerFitView();
	}

	// Drag/reconnect handlers are wired to BaseTopologyViewer but only fire in
	// edit mode, which is permanently disabled (above). Their persistence (the
	// node-position / edge-handle mutations) is commented out — overrides are
	// no longer saved. Kept for revival.
	async function handleNodeDragStop(targetNode: Node) {
		if (!topology) return;
		let movedNode = topology.nodes.find((node) => node.id == targetNode?.id);
		if (movedNode && targetNode && targetNode.position) {
			// Snap to 25px grid (matches SvelteFlow snapGrid and ELK post-layout snap)
			const SNAP = 25;
			const x = Math.round(targetNode.position.x / SNAP) * SNAP;
			const y = Math.round(targetNode.position.y / SNAP) * SNAP;
			// Update local state for immediate feedback
			movedNode.position.x = x;
			movedNode.position.y = y;
			// DISABLED: no mechanism to persist position changes.
			// await updateNodePositionMutation.mutateAsync({
			// 	topologyId: topology.id,
			// 	siteId: topology.site_id,
			// 	view: $activeView,
			// 	nodeId: movedNode.id,
			// 	position: { x, y }
			// });
		}
	}

	async function handleReconnect(edge: Edge, newConnection: Connection) {
		if (!topology) return;
		const edgeData = edge.data as TopologyEdge;

		if ($selectedEdge && edge.id === $selectedEdge.id) {
			let topologyEdge = topology.edges.find((e) => e.id == edgeData.id);
			if (
				topologyEdge &&
				newConnection.source == topologyEdge.source &&
				newConnection.target == topologyEdge.target &&
				newConnection.sourceHandle &&
				newConnection.targetHandle
			) {
				// Update local state for immediate feedback
				topologyEdge.source_handle = newConnection.sourceHandle as TopologyEdge['source_handle'];
				topologyEdge.target_handle = newConnection.targetHandle as TopologyEdge['target_handle'];
				// DISABLED: no mechanism to persist edge handle changes.
				// await updateEdgeHandlesMutation.mutateAsync({
				// 	topologyId: topology.id,
				// 	siteId: topology.site_id,
				// 	view: $activeView,
				// 	edgeId: topologyEdge.id,
				// 	sourceHandle: newConnection.sourceHandle as 'Top' | 'Bottom' | 'Left' | 'Right',
				// 	targetHandle: newConnection.targetHandle as 'Top' | 'Bottom' | 'Left' | 'Right'
				// });
			}
		}
	}

	const handleKeydown = createTopologyKeydownHandler({
		getBaseViewer: () => baseViewer,
		getShortcutsHelpOpen: () => shortcutsHelpOpen,
		setShortcutsHelpOpen: (open) => (shortcutsHelpOpen = open),
		selectionStores: { selectedNode, selectedEdge, selectedNodes },
		isEnabled: () => isActive,
		onToggleLock: () => onToggleLock?.(),
		onRebuild: () => onRebuild?.()
	});
</script>

<svelte:window onkeydown={handleKeydown} />

{#if topology}
	<div class="relative h-[calc(100vh-120px)] w-full">
		<BaseTopologyViewer
			bind:this={baseViewer}
			{topology}
			readonly={!editMode || $topologyReadOnly}
			showControls={true}
			{editMode}
			{sidebarCollapsed}
			onToggleEditMode={null}
			onNodeDragStop={handleNodeDragStop}
			onReconnect={handleReconnect}
			onOpenShortcuts={() => (shortcutsHelpOpen = true)}
			onOpenSearch={() => searchOpen.set(true)}
		/>
		<SearchOverlay />
		<ShortcutsHelpOverlay bind:isOpen={shortcutsHelpOpen} />
	</div>
{/if}
