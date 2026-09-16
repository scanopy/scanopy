<script lang="ts">
	import { writable, derived, get, type Writable } from 'svelte/store';
	import {
		SvelteFlow,
		MiniMap,
		Background,
		BackgroundVariant,
		useNodesInitialized,
		useViewport,
		type Connection,
		useSvelteFlow,
		useUpdateNodeInternals
	} from '@xyflow/svelte';
	import {
		common_collapse,
		common_expand,
		topology_levelFullyCollapsed,
		topology_levelContainersExpanded,
		topology_levelSubcontainersExpanded,
		topology_levelFullyExpanded,
		topology_parseFailed,
		topology_detailSimplified,
		common_rendering
	} from '$lib/paraglide/messages';
	import { type Node, type Edge } from '@xyflow/svelte';
	import { getViewportForBounds, type Rect } from '@xyflow/system';
	import { shouldSimplify } from '../../pipeline/render-mode';
	import {
		ABSOLUTE_MIN_ZOOM,
		DEFAULT_MIN_ZOOM,
		boundsOfNodes,
		zoomFloorFor
	} from '../../viewport-fit';
	import '@xyflow/svelte/dist/style.css';
	import './topology-viewer.css';
	import { pushError } from '$lib/shared/stores/feedback';
	import {
		previewEdges,
		baseFlowEdges,
		selectedNodes,
		selectedEdge as selectedEdgeStore,
		selectedNode as selectedNodeStore,
		topologyOptions,
		optionsPanelExpanded,
		editingDependencyId,
		OPTIONS_PANEL_FITVIEW_PADDING_PX,
		MINIMAP_WIDTH_PX,
		MINIMAP_HEIGHT_PX,
		MINIMAP_OFFSET_PX,
		aggregatedEdgeOriginals,
		getInfrastructureRuleIdForTopology,
		topologyReadOnly,
		topologyOptionsHydrated,
		activeView
	} from '../../queries';
	import {
		isExporting,
		isMeasuring,
		detailSimplified,
		isRenderingTopology,
		expandedPortNodeIds
	} from '../../interactions';

	// Import custom node/edge components
	import ContainerNode from './ContainerNode.svelte';
	import ElementNode from './ElementNode.svelte';
	import CustomEdge from './CustomEdge.svelte';
	import TopologySidebarControls from './TopologySidebarControls.svelte';
	import type { RenderableTopology } from '../../types/base';
	import {
		collapsedContainers,
		collapseLevel,
		stepExpand,
		stepCollapse,
		nextEffectiveLevel
	} from '../../collapse';
	import type { CollapseLevel } from '../../collapse';
	import {
		updateConnectedNodes,
		setEdgeHover,
		clearEdgeHoverState,
		expandedBundles,
		collapseAllBundles,
		collectEdgeHandles,
		edgeHandlesByNode,
		mergeEdgeHandles,
		previewEdgeHandlesByNode,
		searchHiddenNodeIds,
		tagHiddenNodeIds,
		hiddenEntityIds
	} from '../../interactions';
	import {
		selectNode,
		selectEdge,
		clearSelection,
		handleModifierNodeClick,
		handleBoxSelect,
		type SelectionStores
	} from '../../selection';
	import { onMount, tick, setContext, getContext } from 'svelte';
	import { writable as svelteWritable } from 'svelte/store';
	import { themeStore } from '$lib/shared/stores/theme.svelte';
	import { containerTypes } from '$lib/shared/stores/metadata';

	// Pipeline imports
	import { createInitialState, type XY } from '../../pipeline/types';
	import { prepareTopologyData, hiddenMetadataKey } from '../../pipeline/prepare';
	import { resolveNodeSizes } from '../../pipeline/measure';
	import { executeLayout, handlePortExpansion } from '../../pipeline/execute-layout';
	import { preloadElk } from '../../layout/elk-layout';
	import { buildFlowNodes, sortFlowNodes, stripSizeSeed } from '../../pipeline/build-flow-nodes';
	import { buildFlowEdges } from '../../pipeline/build-flow-edges';
	import { cacheCollapsedSizes, reconcileMeasuredSizes } from '../../pipeline/post-render';
	import throttle from 'just-throttle';
	import { computeEdgeDisplayUpdates } from '../../pipeline/sync-edge-display';
	import { shouldCull } from '../../pipeline/render-mode';
	import {
		installDiagnostics,
		noteNodeStoreWrite,
		noteRunDetail,
		noteRunEnd,
		noteRunStart,
		noteRunSuperseded,
		recordAfterRun,
		recordAfterViewportMove
	} from '../../diagnostics';
	import * as perf from '../../perf';
	import {
		reloadInputsDiff,
		snapshotReloadInputs,
		type ReloadInputs
	} from '../../pipeline/reload-guard';

	// Props
	let {
		topology,
		readonly = false,
		showControls = true,
		isEmbed = false,
		hideAttribution = false,
		showBranding = false,
		showMinimap = undefined,
		onNodeDragStop = null,
		onReconnect = null,
		onOpenShortcuts = null,
		onOpenSearch = null,
		editMode = false,
		onToggleEditMode = null,
		sidebarCollapsed = false
	}: {
		topology: RenderableTopology;
		readonly?: boolean;
		showControls?: boolean;
		isEmbed?: boolean;
		hideAttribution?: boolean;
		showBranding?: boolean;
		showMinimap?: boolean | undefined;
		onNodeDragStop?: ((node: Node) => void) | null;
		onReconnect?: ((edge: Edge, newConnection: Connection) => void) | null;
		onOpenShortcuts?: (() => void) | null;
		onOpenSearch?: (() => void) | null;
		editMode?: boolean;
		onToggleEditMode?: (() => void) | null;
		sidebarCollapsed?: boolean;
	} = $props();

	// Create a context store for the topology so child nodes can access it.
	// The effect below keeps the store in sync with the prop across updates;
	// the initial read of `topology` here is just seeding the store.
	// svelte-ignore state_referenced_locally
	const topologyContext = svelteWritable<RenderableTopology>(topology);
	setContext('topology', topologyContext);
	$effect(() => {
		topologyContext.set(topology);
	});

	// Resolve selection stores from context (share/embed) or fall back to global stores.
	// We pass the store *reference* through, not its value, so $/get() don't apply.
	/* eslint-disable svelte/require-store-reactive-access */
	const selNodeStore = getContext<Writable<Node | null>>('selectedNode') ?? selectedNodeStore;
	const selEdgeStore = getContext<Writable<Edge | null>>('selectedEdge') ?? selectedEdgeStore;
	const selNodesStore = getContext<Writable<Node[]>>('selectedNodes') ?? selectedNodes;
	/* eslint-enable svelte/require-store-reactive-access */
	const selectionStores: SelectionStores = {
		selectedNode: selNodeStore,
		selectedEdge: selEdgeStore,
		selectedNodes: selNodesStore
	};

	/**
	 * Node count at or above which the collapse fade is skipped in favour of a single store write.
	 *
	 * Set well above the graphs where the animation reads as polish and well below the scale where a
	 * second full node adoption costs hundreds of megabytes. The culling threshold (150) is a
	 * different question — that one is about what is mounted, this one about how many times the set
	 * is handed over — so it is deliberately not reused.
	 */
	const ANIMATE_COLLAPSE_MAX_NODES = 600;

	/** Empty handle map for measurement builds, whose handles `stripSizeSeed` removes anyway. */
	const NO_HANDLES: Map<string, Set<string>> = new Map();

	/**
	 * SvelteFlow's own default, stated so the fit measurement and the canvas use one number.
	 *
	 * Passed to the flow as well as to `getViewportForBounds` below. Left implicit, the two could
	 * be given different ceilings and the measured "required zoom" would stop describing the fit
	 * the canvas actually performs.
	 */
	const MAX_ZOOM = 2;

	// Track viewport panning state
	let viewportMoved = false;
	let viewportMoveTimer: ReturnType<typeof setTimeout> | null = null;

	const { fitView, setViewport, getNodes, getInternalNode, getViewport } = useSvelteFlow();
	const viewerViewport = useViewport();
	let containerElement: HTMLDivElement;

	/**
	 * Bounds of the graph the pipeline last wrote, in graph coordinates.
	 *
	 * Read from `$nodes` rather than `getNodes()`: on a view switch SvelteFlow has not adopted the
	 * new set yet, and the bounds are wanted exactly then.
	 */
	function getLayoutBounds(): Rect | null {
		return boundsOfNodes($nodes);
	}

	/**
	 * The viewport that frames `bounds`, at a given zoom floor.
	 *
	 * One helper for both the fit and the measurement of what the fit wanted, so the two cannot
	 * disagree about padding, pane size or what "fits" means — they are the same call with a
	 * different floor.
	 */
	function viewportFor(bounds: Rect, minZoom: number) {
		return getViewportForBounds(
			bounds,
			containerElement?.clientWidth ?? 0,
			containerElement?.clientHeight ?? 0,
			minZoom,
			MAX_ZOOM,
			getFitViewPadding()
		);
	}

	/**
	 * The zoom a fit needs, measured at the absolute floor so no clamp can hide the answer.
	 *
	 * `null` until the pane has a size. `containerElement` is a `bind:this`, so for the first
	 * render or two it is unbound and reads 0×0 — against which every graph "needs" the absolute
	 * floor, which would drop the zoom floor to its minimum on a graph that fits comfortably and
	 * report a required zoom that describes nothing.
	 */
	function requiredFitZoom(bounds: Rect): number | null {
		const paneWidth = containerElement?.clientWidth ?? 0;
		const paneHeight = containerElement?.clientHeight ?? 0;
		if (paneWidth <= 0 || paneHeight <= 0) return null;
		return viewportFor(bounds, ABSOLUTE_MIN_ZOOM).zoom;
	}

	/**
	 * Returns fitView padding that accounts for overlays (options panel, minimap).
	 *
	 * The minimap occupies a rectangle in the bottom-left corner. Rather than
	 * reserving an entire row or column, we simulate fitView with uniform padding,
	 * project each node into viewport coordinates, and check if any actually
	 * overlap the minimap region. Only adds padding if real overlap is detected,
	 * and picks the direction (left or bottom) that requires the smallest shift.
	 */
	function getFitViewPadding(): import('@xyflow/system').Padding {
		const minimapVisible =
			showMinimap !== undefined ? showMinimap : get(topologyOptions).local.show_minimap;
		const hasPanel = get(optionsPanelExpanded);

		if (!hasPanel && !minimapVisible) return 0.2;

		const BASE_PAD = 0.2;
		type Pad = number | `${number}px` | `${number}%`;
		let extraBottom: Pad = BASE_PAD;
		let extraLeft: Pad = BASE_PAD;

		if (minimapVisible && containerElement) {
			const cw = containerElement.clientWidth;
			const ch = containerElement.clientHeight;
			const allNodes = getNodes();

			if (allNodes.length > 0 && cw > 0 && ch > 0) {
				// 1. Compute topology bounding box
				let minX = Infinity,
					maxX = -Infinity,
					minY = Infinity,
					maxY = -Infinity;
				for (const n of allNodes) {
					const x = n.position.x;
					const y = n.position.y;
					const w = n.measured?.width ?? n.width ?? 0;
					const h = n.measured?.height ?? n.height ?? 0;
					if (x < minX) minX = x;
					if (x + w > maxX) maxX = x + w;
					if (y < minY) minY = y;
					if (y + h > maxY) maxY = y + h;
				}
				const topoW = maxX - minX || 1;
				const topoH = maxY - minY || 1;

				// 2. Simulate fitView with uniform base padding
				const availW = cw * (1 - 2 * BASE_PAD);
				const availH = ch * (1 - 2 * BASE_PAD);
				const zoom = Math.min(availW / topoW, availH / topoH);

				// Center offset: maps topology coords → viewport coords
				const cx = cw / 2 - (minX + topoW / 2) * zoom;
				const cy = ch / 2 - (minY + topoH / 2) * zoom;

				// 3. Minimap rectangle in viewport coords (with breathing room)
				const GAP = 8;
				const mmLeft = MINIMAP_OFFSET_PX - GAP;
				const mmTop = ch - MINIMAP_OFFSET_PX - MINIMAP_HEIGHT_PX - GAP;
				const mmRight = MINIMAP_OFFSET_PX + MINIMAP_WIDTH_PX + GAP;
				const mmBottom = ch - MINIMAP_OFFSET_PX + GAP;

				// 4. Check if any node overlaps the minimap region
				let hasOverlap = false;
				let maxNodeRight = 0; // rightmost edge of overlapping nodes (for left shift calc)
				let maxNodeBottom = 0; // bottommost edge of overlapping nodes (for bottom shift calc)

				for (const n of allNodes) {
					const nw = n.measured?.width ?? n.width ?? 0;
					const nh = n.measured?.height ?? n.height ?? 0;
					const vx = n.position.x * zoom + cx;
					const vy = n.position.y * zoom + cy;
					const vr = vx + nw * zoom;
					const vb = vy + nh * zoom;

					// Rectangle intersection test
					if (vx < mmRight && vr > mmLeft && vy < mmBottom && vb > mmTop) {
						hasOverlap = true;
						if (vr > maxNodeRight) maxNodeRight = vr;
						if (vb > maxNodeBottom) maxNodeBottom = vb;
					}
				}

				// 5. If overlap, compute minimum shift in each direction and pick the smaller
				if (hasOverlap) {
					const shiftRight = mmRight - mmLeft + GAP; // push content right past minimap
					const shiftUp = mmBottom - mmTop + GAP; // push content up past minimap

					if (shiftRight <= shiftUp) {
						extraLeft = `${MINIMAP_WIDTH_PX + MINIMAP_OFFSET_PX + GAP * 2}px`;
					} else {
						extraBottom = `${MINIMAP_HEIGHT_PX + MINIMAP_OFFSET_PX + GAP * 2}px`;
					}
				}
				// No overlap → extraLeft and extraBottom stay at BASE_PAD
			}
		}

		return {
			top: BASE_PAD,
			right: BASE_PAD,
			bottom: extraBottom,
			left: hasPanel ? `${OPTIONS_PANEL_FITVIEW_PADDING_PX}px` : extraLeft
		};
	}

	/**
	 * Fit the whole graph, without waiting for SvelteFlow to decide the nodes are ready.
	 *
	 * `fitView()` does not move the viewport. It sets `fitViewQueued` and the fit is applied from
	 * the `nodesInitialized` derived, which is false while *any* node lacks `measured` — and the
	 * only other way through is `updateNodeInternals`, which needs a node to mount and measure.
	 * With culling on, a transform left over from the previous view can select nothing at all, and
	 * then nothing mounts, so nothing measures, so the queued fit never runs and the transform
	 * never moves. That is a closed loop: a blank canvas where `F` does nothing, which is how this
	 * was reported.
	 *
	 * Computing the viewport and pushing it through `setViewport` is what `fitView` would
	 * eventually have done, minus the wait for a condition the blank state prevents. We already
	 * know the bounds — the pipeline just laid them out.
	 */
	export async function triggerFitView() {
		const bounds = getLayoutBounds();
		if (!bounds) return;
		await setViewport(viewportFor(bounds, derivedMinZoom));
	}

	export function fitViewToNodes(nodeIds: string[]) {
		requestAnimationFrame(() =>
			fitView({ nodes: nodeIds.map((id) => ({ id })), padding: 0.5, duration: 300 })
		);
	}

	onMount(() => {
		const { fitView } = useSvelteFlow();
		const observer = new IntersectionObserver(
			(entries) => {
				if (entries[0].isIntersecting) {
					requestAnimationFrame(() => fitView());
					observer.disconnect();
				}
			},
			{ threshold: 0.1 }
		);
		if (containerElement) {
			observer.observe(containerElement);
		}
		return () => observer.disconnect();
	});

	// Define node types
	const nodeTypes = { Container: ContainerNode, Element: ElementNode };
	const customEdgeTypes = { custom: CustomEdge };

	// Refit viewport when panel expands/collapses (after 300ms CSS transition)
	let panelInitialized = false;
	$effect(() => {
		if ($optionsPanelExpanded !== undefined) {
			if (panelInitialized) {
				setTimeout(() => fitView({ padding: getFitViewPadding() }), 300);
			}
			panelInitialized = true;
		}
	});

	// Stores for SvelteFlow
	let nodes = writable<Node[]>([]);
	let edges = writable<Edge[]>([]);
	const nodesInitialized = useNodesInitialized();
	let pendingEdges: Edge[] = [];

	// Pipeline state
	const layoutState = createInitialState();

	/**
	 * A DOM measurement pass is running.
	 *
	 * Culling must stay off for its duration: the pass mounts every node and reads its height, and
	 * a culled node never mounts, so measurement would silently return sizes for the on-screen
	 * subset and hand ELK fallbacks for the rest. Set on *every* measure pass, not just the first.
	 */
	// A store, not component state: node components need to see it too — a card simplified for
	// low zoom would otherwise be measured at its pinned height instead of its content's.
	let measurePassActive = $derived($isMeasuring);

	/**
	 * The cold load is measuring, which additionally suppresses the expand animation.
	 *
	 * Split from `measurePassActive` because one flag used to do both jobs, and only ever on the
	 * first render (`lastRenderedTopoKey === ''`). Later measure passes still have to suspend
	 * culling, and with a single flag they did not — harmless only while culling was inert.
	 *
	 * Both flags now hide the pane. A measure pass mounts every node with no container sizes, so
	 * containers render as 2px slivers with their contents outside for as long as it runs; at a
	 * few thousand nodes that is a frame or more, and it was plainly visible because only the cold
	 * load hid anything. This flag still exists on its own because `shouldAnimate` means "not a
	 * cold load", which is a different question from "is something being measured".
	 */
	let coldLoadMeasure = $state(false);
	let animatingCollapse = $state(false);

	/** Minimum gap between collapsed-size refreshes while panning. */
	const COLLAPSED_SIZE_REFRESH_MS = 500;

	/** Both flags down: no pass is measuring and the pane may show. */
	function endMeasurePass(): void {
		isMeasuring.set(false);
		coldLoadMeasure = false;
	}

	/**
	 * The zoom floor, derived from the graph rather than fixed.
	 *
	 * A hard-coded `0.1` is what made a large estate unfittable: every fit clamped, so `F` appeared
	 * to do nothing and the canvas showed a fraction of the graph at a tenth scale. Recomputed as
	 * the store changes, since the graph a collapse press produces is a different size.
	 */
	let derivedMinZoom = $derived.by(() => {
		const bounds = boundsOfNodes($nodes);
		if (!bounds) return DEFAULT_MIN_ZOOM;
		const required = requiredFitZoom(bounds);
		return required === null ? DEFAULT_MIN_ZOOM : zoomFloorFor(required);
	});

	// Cull off-screen nodes once the graph is big enough — see
	// `pipeline/render-mode.ts` for why measuring and exporting must suspend it.
	/**
	 * Whether the canvas is currently drawing boxes instead of contents.
	 *
	 * Read once here for the badge below, rather than inferred from anything the nodes do. A view
	 * that silently stops showing what it was showing reads as a bug — the original report this
	 * whole line of work came from was someone describing exactly that — so the drop in detail is
	 * stated on screen rather than left to be discovered.
	 *
	 * Reading the viewport reactively costs one component here, and the value is a boolean, so the
	 * badge appears and disappears on threshold crossings rather than on every pan frame.
	 */
	let detailHidden = $derived(
		shouldSimplify({
			zoom: viewerViewport.current.zoom,
			nodeCount: $nodes.length,
			measuring: measurePassActive,
			exporting: $isExporting
		})
	);

	// Publish rather than let each node decide: the size term above is not visible to a node, and
	// one refcounted subscription beats a viewport read per node. See `detailSimplified`.
	$effect(() => detailSimplified.set(detailHidden));

	let cullOffscreen = $derived(
		shouldCull({
			renderedCount: $nodes.length,
			measuring: measurePassActive,
			exporting: $isExporting
		})
	);

	/// Everything the blank-canvas diagnostic cannot read from the DOM. Gathered here so the
	/// sample sees the same numbers the culling gate above was given, not a re-derived guess.
	const diagnosticInputs = () => ({
		storeNodes: $nodes.length,
		storeEdges: $edges.length,
		measuring: measurePassActive,
		coldLoadMeasure,
		exporting: $isExporting,
		culling: cullOffscreen,
		container: containerElement,
		// Read lazily: the diagnostic decides whether it wants all of them or a sample, and at
		// this customer's node count the difference matters on the throttled viewport path.
		internalNodes: () => $nodes.map((n) => getInternalNode(n.id)),
		// The payload as the server sent it, so a report can tell "few edges arrived" from "many
		// arrived and were dropped in rendering" — the two causes `store.edges` cannot separate.
		payload: () => topology ?? null,
		// The zoom a fit wants against the zoom it is allowed. Lazy, because it walks the store.
		fitZoom: () => {
			const bounds = boundsOfNodes($nodes);
			if (!bounds) return null;
			const required = requiredFitZoom(bounds);
			if (required === null) return null;
			return {
				required,
				applied: getViewport().zoom,
				clampedAtFloor: required < derivedMinZoom
			};
		}
	});
	installDiagnostics(diagnosticInputs);

	/**
	 * Single point of write for the node store.
	 *
	 * Every mount of the graph goes through here, so counting the writes distinguishes one large
	 * allocation from a remount loop — the question the customer's 247 out-of-memory throws left
	 * open, and one the per-sample ring buffer could not answer.
	 */
	/**
	 * Every write to the node store, timed and heap-measured.
	 *
	 * The per-run heap ledger placed 88% of a run's growth — 357MB of 407MB — between `build-edges`
	 * finishing and `post-render` starting, with the whole ELK layout accounting for only 50MB. The
	 * writes in that span are the candidates: each hands the full node set to SvelteFlow, which
	 * builds an internal representation of all 2,890 nodes per write, and the collapse animation
	 * performs up to three of them for one press. Instrumented here rather than at each call site so
	 * every branch is covered; repeated entries appear in the ledger in order.
	 */
	function setStoreNodes(next: Node[]): void {
		const writeDone = perf.stage('render.store-write');
		noteNodeStoreWrite(next.length);
		nodes.set(next);
		writeDone();
	}

	// --- Reactive triggers ---

	// Clear expanded bundles when bundling is toggled off
	$effect(() => {
		if (!$topologyOptions.local.bundle_edges) {
			collapseAllBundles();
		}
	});

	// Trigger loadTopologyData on topology or store changes
	const bundleEdgesStore = derived(topologyOptions, (o) => o.local.bundle_edges ?? false);
	const hideEdgeTypesStore = derived(topologyOptions, (o) =>
		(o.local.hide_edge_types ?? []).join(',')
	);
	// Metadata-value filters (e.g. hiding the OpenPorts service category) are read at render
	// time straight from the options, so no hidden-id store ever fires for them. Without this
	// the cards resize under a layout that never re-runs, and they overlap.
	const hiddenMetadataStore = derived([topologyOptions, activeView], ([, view]) =>
		hiddenMetadataKey(view)
	);

	// Infra rule id derived from the topology bundle being rendered (not the
	// global options store, which hydrates out-of-band and lags a network
	// switch). Keeps auto-collapse of the infra subcontainer correct on switch.
	const getInfrastructureRuleId = () => getInfrastructureRuleIdForTopology(topology);

	/**
	 * Whether a pipeline run is in flight. Deliberately *not* `$state`.
	 *
	 * `triggerLoad` both reads this and writes it, and it is called from an `$effect` (the topology
	 * prop watcher below). As a rune that read is tracked, so the write re-invalidates the effect,
	 * which calls `triggerLoad` again — an unbounded loop that hangs the page before the first
	 * layout, on a graph of any size. It was introduced and reverted once; do not make it reactive.
	 *
	 * The UI still needs to know, so `setLoadInProgress` mirrors it into a store. The store is only
	 * ever read by the template and by other components, never by the control flow here, which is
	 * what keeps the cycle broken.
	 */
	let loadInProgress = false;

	function setLoadInProgress(value: boolean): void {
		loadInProgress = value;
		isRenderingTopology.set(value);
	}
	let pendingReload = false;
	/**
	 * Escape hatch for the hydration gate below.
	 *
	 * If `hydrateStoresFromTopology` never runs on some path, the view must still
	 * render — a blank topology is a far worse failure than one wasted layout. The
	 * timeout only ever costs something when hydration is genuinely absent.
	 */
	let hydrationWaivedAt = $state(false);
	/**
	 * The store values the in-flight run actually consumed, snapshotted once
	 * `prepare` has returned. A pending reload is honoured only if the current
	 * values differ from these — see `pipeline/reload-guard.ts`.
	 */
	let inFlightInputs: ReloadInputs | null = null;

	function currentReloadInputs(): ReloadInputs {
		return {
			collapsed: get(collapsedContainers),
			expandedBundles: get(expandedBundles),
			expandedPorts: get(expandedPortNodeIds),
			bundleEdges: get(topologyOptions).local.bundle_edges ?? false,
			hiddenEdgeTypes: (get(topologyOptions).local.hide_edge_types ?? []).join(','),
			tagHidden: get(tagHiddenNodeIds),
			hiddenEntities: get(hiddenEntityIds),
			hiddenMetadata: hiddenMetadataKey(get(activeView)),
			topology
		};
	}
	function triggerLoad(source = 'unknown') {
		// Hold every entry point, not just the topology effect: the option stores
		// fire during hydration too, and any one of them starting the pipeline
		// early would defeat the gate. The effect below re-triggers once hydration
		// lands, so nothing is lost by returning here.
		if (topology && !loadInProgress && !$topologyOptionsHydrated && !hydrationWaivedAt) {
			perf.count(`deferred-until-hydration:${source}`);
			return;
		}
		if (!topology || loadInProgress) {
			if (topology && loadInProgress) {
				// A store wrote while the pipeline was mid-flight, so the whole run
				// will be repeated. Attributed by source because each one costs a
				// full re-layout (two elk.layout() calls).
				perf.count(`pending-reload:${source}`);
				pendingReload = true;
				// Abandon the run in flight the moment we can prove its result is already obsolete.
				//
				// A press arrives roughly every 2s and a run takes 1.8-3.7s, so the first run
				// reliably finished a full layout that the queued run replaced on arrival — two ELK
				// passes and ~600MB of garbage for one visible result. Bumping the generation makes
				// `isStale()` true for the in-flight run, which bails at the check before ELK.
				//
				// Only when the inputs demonstrably differ: `inFlightInputs` is snapshotted once
				// `prepare` returns, so a null here means the run has not yet fixed its inputs and
				// cancelling would be guesswork.
				if (inFlightInputs && reloadInputsDiff(inFlightInputs, currentReloadInputs()).length > 0) {
					layoutState.layoutGeneration += 1;
					perf.count(`superseded:${source}`);
					noteRunSuperseded();
				}
			}
			return;
		}
		// Counted at the point a run actually begins (as opposed to being queued),
		// so each full pipeline execution — two elk.layout() calls — is attributable
		// to what started it.
		perf.count(`run-start:${source}`);
		// Always-on twin of the counter above: `perf` records nothing in a customer's build, and
		// which trigger started a run is the missing half of the zero-sized-container reports.
		noteRunStart(source);
		setLoadInProgress(true);
		pendingReload = false;
		// Give the browser one frame to draw the indicator before the run starts.
		//
		// Svelte flushes the DOM on a microtask, but painting needs a frame, and a run reaches its
		// synchronous ELK without yielding one. ~16ms against the eighteen to twenty-two seconds of
		// layout that are the reason for having an indicator at all.
		//
		// Not the fix for the spinner sitting still — that was the icon being an inline SVG, which
		// the compositor will not drive, so the animation depended on the very thread ELK was
		// holding. An earlier version of this comment claimed the indicator "never rendered", from a
		// sampler that ran in-page and was therefore blocked alongside everything else: it recorded
		// an absence it could not tell apart from the element not existing. It rendered fine.
		requestAnimationFrame(() => {
			void runPipeline();
		});
	}

	function runPipeline(): Promise<void> {
		return loadTopologyData()
			.catch((err) => {
				endMeasurePass();
				pushError(topology_parseFailed({ error: String(err) }));
			})
			.finally(() => {
				noteRunEnd();
				setLoadInProgress(false);
				if (pendingReload) {
					pendingReload = false;
					// Only re-run if an input actually differs from what the run
					// consumed. Most mid-run writes are the pipeline's own (prepare
					// seeding collapse state) or derived stores re-emitting an
					// identical value during option hydration — re-running for those
					// costs two elk.layout() calls and changes nothing.
					const consumed = inFlightInputs;
					inFlightInputs = null;
					if (consumed) {
						const changed = reloadInputsDiff(consumed, currentReloadInputs());
						if (changed.length === 0) {
							perf.count('reload-suppressed');
							return;
						}
						for (const field of changed) perf.count(`reload-cause:${field}`);
					}
					triggerLoad('pending');
				}
				inFlightInputs = null;
			});
	}

	let storesInitialized = false;

	/**
	 * Start a run *after* the caller that wrote the store has finished.
	 *
	 * Svelte notifies subscribers synchronously from `set`, and `loadTopologyData` runs as far as
	 * `prepare` before its first `await` — so the whole first half of a pipeline run executes
	 * inside the `.set()` call, before the writing function's next statement. Anything a caller
	 * does after writing collapse state therefore lands too late to affect the run it just caused.
	 *
	 * `stepExpand` was one such caller: it marked the auto-collapse containers seen after writing,
	 * so `applyAutoCollapse` re-collapsed the very containers it was about to be told to leave
	 * alone. The level advanced, the collapsed set did not, `collapseChanged` came out false, no
	 * layout ran, and the diagram stayed on the previous level with anything new to it unsized.
	 * That call site is now ordered correctly, but the hazard belongs to every writer of these
	 * stores — a container chevron, the level buttons, a filter — and each would have to remember.
	 *
	 * Deferring by a microtask makes the ordering safe by construction: the caller always
	 * completes first, and the run still starts in the same frame.
	 */
	const deferTriggerLoad = (source: string) => {
		queueMicrotask(() => {
			if (storesInitialized) triggerLoad(source);
		});
	};

	collapsedContainers.subscribe(() => {
		if (storesInitialized) deferTriggerLoad('collapsed');
	});
	expandedBundles.subscribe(() => {
		if (storesInitialized) triggerLoad('bundles');
	});
	expandedPortNodeIds.subscribe(() => {
		if (storesInitialized) triggerLoad('ports');
	});
	bundleEdgesStore.subscribe(() => {
		if (storesInitialized) triggerLoad('bundle-option');
	});
	hideEdgeTypesStore.subscribe(() => {
		if (storesInitialized) triggerLoad('hidden-edge-types');
	});
	// Filter changes must re-run the pipeline so ELK sees the new node set and containers
	// reflow around the removed cards. Three stores, because a filter can change the layout
	// without removing a node: an entity shown inline on another node's card resizes that card,
	// and a metadata-value filter never reaches either hidden-id store at all.
	tagHiddenNodeIds.subscribe(() => {
		if (storesInitialized) triggerLoad('tag-filter');
	});
	hiddenEntityIds.subscribe(() => {
		if (storesInitialized) triggerLoad('entity-filter');
	});
	hiddenMetadataStore.subscribe(() => {
		if (storesInitialized) triggerLoad('metadata-filter');
	});
	storesInitialized = true;

	onMount(() => {
		if (get(topologyOptionsHydrated)) return;
		const timer = setTimeout(() => {
			// Re-check rather than waive blindly: on a slow first load hydration may
			// simply not have arrived yet when the timer was armed, and waiving then
			// would reintroduce the pre-hydration layout this gate exists to avoid.
			if (get(topologyOptionsHydrated)) return;
			perf.count('hydration-gate-waived');
			hydrationWaivedAt = true;
		}, 2000);
		return () => clearTimeout(timer);
	});

	// Wait for options to hydrate before the first layout. Without this the
	// pipeline races hydration and discards its first full run — see
	// `topologyOptionsHydrated`.
	$effect(() => {
		if (topology && ($topologyOptionsHydrated || hydrationWaivedAt)) triggerLoad('topology');
	});

	// Update edges when selection or search/tag filter changes
	$effect(() => {
		const curSelectedNode = $selNodeStore;
		const curSelectedEdge = $selEdgeStore;
		const multiSelected = $selNodesStore;
		const searchHidden = $searchHiddenNodeIds;
		const tagHidden = $tagHiddenNodeIds;

		if (topology && (topology.edges || topology.nodes)) {
			const currentBaseEdges = get(baseFlowEdges);
			const currentNodes = get(nodes);
			const opts = get(topologyOptions);

			updateConnectedNodes(
				curSelectedNode,
				curSelectedEdge,
				currentBaseEdges,
				currentNodes,
				topology,
				multiSelected,
				opts.local.hide_edge_types ?? []
			);
			const updatedEdges = computeEdgeDisplayUpdates(
				currentBaseEdges,
				curSelectedNode,
				curSelectedEdge,
				searchHidden,
				tagHidden
			);
			if (updatedEdges !== currentBaseEdges) baseFlowEdges.set(updatedEdges);
		}
	});

	// Add edges when nodes are ready
	$effect(() => {
		if (nodesInitialized.current && pendingEdges.length > 0) {
			baseFlowEdges.set(pendingEdges);
			pendingEdges = [];
		}
	});

	// --- Main layout pipeline ---

	async function loadTopologyData() {
		// Wait for containerElement to be available (bind:this fires after mount)
		if (!containerElement) {
			await tick();
			if (!containerElement) return;
		}
		const thisGeneration = ++layoutState.layoutGeneration;
		const isStale = (): boolean => thisGeneration !== layoutState.layoutGeneration;

		if (!topology || (!topology.edges && !topology.nodes)) return;
		perf.beginRun();
		// Fire-and-forget: elkjs is a large module and the measure pass below takes
		// far longer than loading it, so the two should overlap rather than run
		// back to back.
		preloadElk();

		const prepareDone = perf.stage('prepare');
		const prep = prepareTopologyData(topology, layoutState, getInfrastructureRuleId);
		prepareDone();
		// Inputs are fixed once prepare has run — it is the last stage that writes
		// to the watched stores as part of its own work. Snapshot here so those
		// self-writes don't read as external change at the end of the run.
		inFlightInputs = snapshotReloadInputs(currentReloadInputs());
		noteRunDetail({ isNewStructure: prep?.isNewStructure, needsElk: prep?.needsElk });
		if (!prep) return;
		const { needsElk, collapsed, visibleNodes: initialVisibleNodes } = prep;
		let visibleNodes = initialVisibleNodes;

		// Sizes measured by *this* run, once it has measured. Preferred over the view cache so the
		// same frame that lays a node out can also size it — which is what lets culling work on
		// the first render after an expand rather than only after everything has mounted once.
		const viewCacheKey = `${prep.currentView}:${prep.topologyId}`;
		let runSizes: Map<string, XY> | null = null;

		// Handles the run's edges name, filled in before nodes are built. Null until then, which
		// makes `buildFlowNodes` declare all eight per node — the measurement pass builds nodes
		// before any edges exist and must not be narrowed against a stale set.
		let runUsedHandles: Map<string, Set<string>> | null = null;

		// Helper: build positioned flow nodes (called multiple times with different useGraph)
		//
		// `forMeasurement` suppresses handle synthesis entirely: `buildMeasureNodes` pipes the result
		// through `stripSizeSeed`, which deletes `handles` again, so synthesizing eight per node
		// first was building 23,120 objects to throw all of them away.
		const makeNodes = (useGraph: boolean, forMeasurement = false) =>
			sortFlowNodes(
				buildFlowNodes({
					visibleNodes,
					collapsed,
					topology,
					useGraph,
					layoutGraph: layoutState.layoutGraph,
					isNewStructure: prep.isNewStructure,
					liveNodes: getNodes(),
					infraRuleId: getInfrastructureRuleId(),
					editMode: editMode ?? false,
					sizeHints: runSizes ?? layoutState.viewSizeCache.get(viewCacheKey) ?? null,
					usedHandles: forMeasurement ? NO_HANDLES : runUsedHandles
				})
			);

		if (needsElk) {
			const measureDone = perf.stage('measure');
			const elementNodeSizes = await resolveNodeSizes(
				layoutState,
				prep,
				topology,
				containerElement,
				isStale,
				{
					setMeasuring: (v) => {
						// Culling is suspended for every measurement pass, unconditionally —
						// the pass reads heights out of the DOM, so a culled node measures as
						// absent and ELK gets a fallback size for it.
						isMeasuring.set(v);
						// Only hide viewport during measurement for initial load
						// (no nodes on screen). For subsequent measurements (e.g.
						// cacheMisses on collapse), nodes keep their current positions
						// so hiding is unnecessary — and skipping it lets shouldAnimate
						// fire normally.
						if (layoutState.lastRenderedTopoKey === '') {
							coldLoadMeasure = v;
						}
					},
					setNodes: (n) => setStoreNodes(n),
					setEdges: (e) => baseFlowEdges.set(e),
					buildMeasureNodes: (onlyIds?: Set<string>) => {
						// Strip the seeded sizes. The pass exists to learn what a card actually
						// renders as, and a node carrying `measured` + `handles` reads as already
						// initialised — so it would be presented at the size we guessed and could
						// only ever confirm that guess. Dropping both also re-attaches
						// `NodeWrapper`'s ResizeObserver, which is otherwise never reattached.
						let measureNodes = stripSizeSeed(makeNodes(false, true));
						if (onlyIds) {
							// Ancestors come too, whether or not they need measuring: SvelteFlow
							// resolves a child's position against its parent and drops a node whose
							// `parentId` is absent from the set it was given.
							// Not `SvelteSet`: this is a local computed inside a callback and never read
							// reactively — the lint rule targets reactive state, and a reactive Set here
							// would add tracking overhead for a value that is discarded on return.
							// eslint-disable-next-line svelte/prefer-svelte-reactivity
							const keep = new Set(onlyIds);
							const parentOf = new Map(
								measureNodes.map((n) => [n.id, n.parentId as string | undefined])
							);
							for (const id of onlyIds) {
								let parent = parentOf.get(id);
								while (parent && !keep.has(parent)) {
									keep.add(parent);
									parent = parentOf.get(parent);
								}
							}
							measureNodes = measureNodes.filter((n) => keep.has(n.id));
						}
						// Preserve current positions during measurement — DOM
						// measurement only needs element presence, not positions.
						// This prevents nodes from jumping to (0,0) while visible.
						const currentPositions = new Map(getNodes().map((n) => [n.id, n.position]));
						if (currentPositions.size === 0) return measureNodes;
						return measureNodes.map((n) => ({
							...n,
							position: currentPositions.get(n.id) ?? n.position
						}));
					},
					waitForNodesRendered: async (expectedIds?: Set<string>) => {
						// Wait for SvelteFlow to render node DOM elements.
						// We only need DOM presence for measurement, not full initialization.
						await tick();
						// Poll for DOM nodes with a short timeout — nodesInitialized
						// can hang indefinitely for large topologies.
						const start = performance.now();
						const expectedCount = expectedIds?.size ?? 0;
						while (performance.now() - start < 2000) {
							const nodeEls = containerElement?.querySelectorAll('.svelte-flow__node');
							if (nodeEls && nodeEls.length > 0) {
								if (expectedCount === 0) break;
								// Cheap gate first. Verifying every expected id means building
								// a Set of all rendered ids, which is O(nodes) — doing that on
								// every frame of a wait that can span a hundred frames is
								// itself a meaningful cost at a thousand-plus nodes. The DOM
								// can only hold every expected node once it holds at least
								// that many, so the count check rules out almost every frame
								// for the price of a property read.
								if (nodeEls.length >= expectedCount) {
									// Require every expected id to be present before breaking.
									// Breaking on the first node (old-render leftovers) lets a
									// newly-added SSE host miss measurement, so ELK falls back
									// to metadata defaults and positions siblings too close.
									const present = new Set(
										Array.from(nodeEls)
											.map((el) => (el as HTMLElement).dataset.id)
											.filter((id): id is string => !!id)
									);
									let allPresent = true;
									for (const id of expectedIds!) {
										if (!present.has(id)) {
											allPresent = false;
											break;
										}
									}
									if (allPresent) break;
								}
							}
							await new Promise((r) => requestAnimationFrame(r));
						}
					}
				}
			);
			measureDone();
			// The measurement pass is over the moment `resolveNodeSizes` returns — it has already
			// read every height it needed out of the DOM — so culling resumes here rather than at
			// the end of the render.
			//
			// It must be cleared on this path, not only in the branches below. `resolveNodeSizes`
			// calls `setMeasuring(false)` only when it goes stale, and of the render branches only
			// the cold-load one ends with `endMeasurePass()`. A measure pass on an already-rendered
			// graph — every expand at scale — therefore left the flag set for the rest of the
			// session, suspending culling permanently and mounting the whole graph on every run.
			isMeasuring.set(false);
			if (!elementNodeSizes) {
				endMeasurePass();
				return;
			}
			runSizes = elementNodeSizes;

			const layoutDone = perf.stage('layout');
			const layoutResult = await executeLayout(
				topology,
				layoutState,
				prep,
				elementNodeSizes,
				isStale
			);
			layoutDone();
			if (!layoutResult) {
				endMeasurePass();
				return;
			}
			visibleNodes = layoutResult.visibleNodes;
		}

		// Port expansion handling (no full ELK re-layout).
		//
		// This is the pipeline's second DOM measurement site: it mounts the rebuilt nodes and reads
		// `offsetWidth`/`offsetHeight` for the cards whose port list changed. Culling is
		// deliberately *not* suspended for it, unlike the full measurement pass. Suspending it
		// would mount every node in the graph to re-measure a handful of cards, which at this
		// customer's scale is the out-of-memory failure by another route. A card whose ports the
		// user just toggled is on screen by construction, so it is mounted and measures correctly;
		// `handlePortExpansion` drops the cached size of any id it could not find in the DOM, so a
		// card toggled and then scrolled away from re-measures for real the next time it mounts
		// rather than keeping a stale height.
		const currentExpandedPorts = get(expandedPortNodeIds);
		const portsChanged = await handlePortExpansion(
			layoutState,
			currentExpandedPorts,
			containerElement,
			() => makeNodes(false),
			(n) => setStoreNodes(n),
			isStale,
			needsElk,
			viewCacheKey
		);

		// Build final nodes and edges. Edge handles are computed inside
		// buildFlowEdges against final post-layout positions (from layoutGraph)
		// rather than being precomputed by the layout engines.
		const needsLayout = needsElk || portsChanged || prep.collapseChanged;

		// Edges first, then nodes.
		//
		// `buildFlowEdges` takes none of the flow nodes — only the layout graph and the topology —
		// so the order is free, and reversing it lets the nodes be built already knowing which
		// handles their edges name. Emitting all eight per node cost a `handleBounds` object each in
		// `parseHandles`, on every node, on every adoption.
		const buildEdgesDone = perf.stage('build-edges');
		const { flowEdges, originalsMap } = buildFlowEdges({
			elevatedEdges: prep.elevatedEdges,
			collapsed,
			elementToContainer: prep.elementToContainer,
			aggregatedEdges: prep.aggregatedEdges,
			hiddenEdgeTypes: prep.hiddenEdgeTypes,
			layoutNodes: prep.layoutNodes,
			view: prep.currentView,
			layoutGraph: layoutState.layoutGraph,
			bundleEnabled: $topologyOptions.local.bundle_edges ?? false,
			currentExpandedBundles: get(expandedBundles),
			selectionStores
		});
		buildEdgesDone();
		aggregatedEdgeOriginals.set(originalsMap);

		const makeNodesDone = perf.stage('render.make-nodes');
		const realHandles = collectEdgeHandles(flowEdges);
		// Node `handles` data also declares the dependency preview's handles. Writing the node store
		// re-adopts every node and rebuilds its handle bounds from this data, so a rebuild during a
		// preview would otherwise drop the preview edge until it was re-measured.
		runUsedHandles = mergeEdgeHandles(realHandles, get(previewEdgeHandlesByNode));
		const allNodes = makeNodes(needsLayout);
		makeNodesDone();

		// Publish the handles each node's edges name, before the nodes that reference them. Node
		// components render only these; SvelteFlow reads handle boxes out of the DOM, and only for
		// a handle an edge names, so a node whose handles arrive a frame after its edge has that
		// edge dropped.
		const handlesDone = perf.stage('render.publish-handles');
		edgeHandlesByNode.set(realHandles);
		handlesDone();

		// Render
		//
		// The phased fade is skipped on a large graph, where it is the single most expensive thing a
		// collapse press does. Each node-store write costs 130-230MB *downstream* of `nodes.set()` —
		// SvelteFlow adopts the whole node set and Svelte flushes the reactive graph after the
		// synchronous call returns, which is why the write itself times at 1ms and shows no growth
		// while the heap climbs between stages. The phased path performs two or three writes for one
		// press; the direct path performs one. Measured on a 2,890-node graph, that difference was
		// ~350MB of a 392MB run, against an ELK layout costing 40MB.
		//
		// Below the threshold the animation is worth its cost and stays: it is what makes a collapse
		// legible rather than a jump, and on a small graph a second adoption is cheap.
		const shouldAnimate =
			needsElk &&
			!coldLoadMeasure &&
			layoutState.lastRenderedTopoKey !== '' &&
			!prep.viewChanged &&
			allNodes.length < ANIMATE_COLLAPSE_MAX_NODES;

		if (shouldAnimate) {
			animatingCollapse = true;
			const previousNodeIds = new Set(get(nodes).map((n) => n.id));
			const phase1Nodes = allNodes.filter((n) => previousNodeIds.has(n.id));
			setStoreNodes(phase1Nodes);
			baseFlowEdges.set(flowEdges);

			const fullNodes = [...allNodes];
			const fullEdges = [...flowEdges];
			// Await phase 2 + one rAF so new nodes are in the DOM with final
			// sizes before the post-render measurement below runs. Without
			// this, cacheCollapsedSizes would either measure pre-phase-2 state
			// (no new nodes) or be skipped entirely — letting ELK's fallback
			// sizes for fresh SSE hosts persist and produce the overlaps.
			await new Promise<void>((resolve) => {
				setTimeout(() => {
					animatingCollapse = false;
					const newNodeIds = new Set(
						fullNodes.filter((n) => !previousNodeIds.has(n.id)).map((n) => n.id)
					);
					if (newNodeIds.size > 0) {
						// Copies the whole node set to restyle the new arrivals — one full duplicate of
						// every node object, on top of the store write that follows it.
						const fadeCopyDone = perf.stage('render.animate-fade-copy');
						const fadingNodes = fullNodes.map((n) =>
							newNodeIds.has(n.id)
								? { ...n, style: 'opacity: 0; transition: opacity 0.3s ease-in-out;' }
								: n
						);
						fadeCopyDone();
						setStoreNodes(fadingNodes);
						baseFlowEdges.set(fullEdges);
						requestAnimationFrame(() => {
							setStoreNodes(fullNodes);
							baseFlowEdges.set(fullEdges);
							requestAnimationFrame(() => resolve());
						});
					} else {
						setStoreNodes(fullNodes);
						baseFlowEdges.set(fullEdges);
						requestAnimationFrame(() => resolve());
					}
				}, 350);
			});
			if (isStale()) return;
		} else if (!coldLoadMeasure) {
			setStoreNodes(allNodes);
			baseFlowEdges.set(flowEdges);
		} else {
			baseFlowEdges.set([]);
			setStoreNodes(allNodes);
			pendingEdges = flowEdges;
			await tick();
			if (isStale()) {
				endMeasurePass();
				return;
			}
			if (pendingEdges.length > 0) {
				baseFlowEdges.set(pendingEdges);
				pendingEdges = [];
			}
			await tick();
			await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
			if (isStale()) {
				endMeasurePass();
				return;
			}
			endMeasurePass();
		}

		// Post-render: measure collapsed containers at their natural content
		// size (width:auto / height:auto) and trigger a corrective re-layout
		// when new entries are found. This self-heals any case where ELK's
		// first pass used stale/fallback sizes — e.g. a fresh SSE host whose
		// DOM wasn't reconciled in time for the measurement pass. Runs in
		// every branch; the animation branch above awaits phase 2 so new
		// nodes are in the DOM by the time we measure.
		if (containerElement && layoutState.layoutGraph) {
			await tick();
			const cacheSizesDone = perf.stage('post-render.cache-collapsed');
			const newEntries = cacheCollapsedSizes(
				containerElement,
				layoutState.layoutGraph,
				collapsed,
				layoutState.containerSizeCache
			);

			// Correct any cached card height the seeded sizes have drifted from. Nodes built with
			// `measured` never get a ResizeObserver, so this is the only thing that notices — see
			// `reconcileMeasuredSizes`. Bounded by the mounted set, and it converges, so a drift
			// costs one extra layout rather than repeating.
			const viewCache = layoutState.viewSizeCache.get(viewCacheKey);
			const drifted = viewCache
				? reconcileMeasuredSizes(
						containerElement,
						viewCache,
						layoutState.layoutGraph,
						layoutState.driftCorrectedIds
					)
				: 0;
			if (drifted > 0) perf.count('post-render.size-drift');

			cacheSizesDone();
			// Read the layout *model*, not the DOM: this separates "the graph lost its sizes" from
			// "the render has not caught up", which the per-sample degenerate count cannot.
			noteRunDetail({
				// Expanded containers only. A collapsed one has never been laid out expanded, so a
				// zero there is normal and would drown the signal — it is the *expanded* container
				// with no size that renders as its borders with its contents outside.
				containersZeroSizedAfter: [...layoutState.layoutGraph.containers.values()].filter(
					(c) => !c.collapsed && c.expandedSize.width === 0
				).length
			});
			if ((newEntries > 0 || drifted > 0) && !isStale()) {
				// Counted because on a cold load with many collapsed containers this
				// self-heal fires every time, and each recursion is a full pipeline
				// run including two more elk.layout() calls.
				perf.count('post-render-relayout');
				// Invalidate structureKey to force ELK re-run. Do NOT
				// invalidate baseKey — base structure hasn't changed, and
				// clearing it would delete viewSizeCache (element sizes).
				layoutState.sessionStructureKey = '';
				// Preserve fitView intent across the recursive call — the
				// re-run won't see viewChanged/topologyChanged since we
				// update the tracking keys here.
				if (prep.viewChanged || prep.topologyChanged) {
					layoutState.fitViewPending = true;
				}
				layoutState.lastRenderedTopoKey = prep.topoKey;
				layoutState.lastRenderedView = prep.currentView;
				// The recursion is a second full pipeline run — two more `elk.layout()` calls —
				// inside the run record the first one opened. Without its own record it wrote its
				// details over the outer run's, and the outer `durationMs` silently reported both
				// passes as one, while `pipelineRuns` climbed past `runs.length` with nothing to
				// say where the extra runs went.
				noteRunEnd();
				noteRunStart('post-render-relayout');
				await loadTopologyData();
				return;
			}
		}

		const isFirstRender = layoutState.lastRenderedTopoKey === '';
		layoutState.lastRenderedTopoKey = prep.topoKey;
		layoutState.lastRenderedView = prep.currentView;

		if (prep.viewChanged || prep.topologyChanged || isFirstRender || layoutState.fitViewPending) {
			layoutState.fitViewPending = false;
			// Double rAF: first lets SvelteFlow process node positions, second triggers the fit
			requestAnimationFrame(() =>
				requestAnimationFrame(async () => {
					await triggerFitView();
					// The fit is the last thing a cold load does, so this is the
					// point the harness treats as "interactive".
					perf.count('fit-view');
					perf.endRun();
					await sampleOnNextFrame();
				})
			);
		} else {
			perf.endRun();
			// Not awaited: the run is finished, and holding its promise open for a frame would add
			// that frame to the `durationMs` this record exists to report.
			void sampleOnNextFrame();
		}
	}

	/**
	 * Take a diagnostics sample once the frame the user is looking at has actually been drawn.
	 *
	 * Every field in a sample that matters for blankness — `mounted`, `withSize`, `bounds`,
	 * `intersectsPane`, `transform` — is read out of the DOM, and the DOM lags a viewport change by
	 * a frame. Sampling in the same turn as the fit therefore described the *previous* viewport
	 * applied to the new layout, which reads as a blank canvas: a capture showed `mounted: 0` on a
	 * 3,228-node store and 804 mounted 755ms later with nothing else having happened, and a blank
	 * row carrying a `transform` byte-identical to the previous view's. Worse, the first such row
	 * latched the one-shot `firstBlank` capture, so the report spent its evidence on a frame that
	 * was never on screen.
	 */
	async function sampleOnNextFrame(): Promise<void> {
		await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
		recordAfterRun(diagnosticInputs());
	}

	// --- Event handlers ---

	let ignoreNextSelectionChange = false;

	function handleNodeClick({ node, event }: { node: Node; event: MouseEvent | TouchEvent }) {
		if (viewportMoved) return;
		// Multi-select drives dependency creation, so it's an edit action: disable
		// it when read-only (snapshot / share). Single-select (read-only inspect)
		// still works.
		const isModifierClick =
			event instanceof MouseEvent && (event.ctrlKey || event.metaKey) && !$topologyReadOnly;
		if (isModifierClick) {
			handleModifierNodeClick(node, selectionStores);
			ignoreNextSelectionChange = true;
		} else {
			collapseAllBundles();
			selectNode(node, selectionStores);
			ignoreNextSelectionChange = true;
		}
	}

	function handleEdgeClick({ edge }: { edge: Edge; event: MouseEvent }) {
		if (viewportMoved) return;
		collapseAllBundles();
		selectEdge(edge, selectionStores);
		ignoreNextSelectionChange = true;
	}

	function handleMove() {
		viewportMoved = true;
		if (viewportMoveTimer) {
			clearTimeout(viewportMoveTimer);
			viewportMoveTimer = null;
		}
	}

	function handleMoveEnd() {
		viewportMoveTimer = setTimeout(() => {
			viewportMoved = false;
		}, 50);
		// Moving the viewport is what re-evaluates which nodes are inside it, so it is the other
		// moment a canvas can go blank — and the one a customer reported as "locking it in".
		recordAfterViewportMove(diagnosticInputs());
		refreshCollapsedSizesInView();
	}

	/**
	 * Record the real collapsed size of any container that has just come into view.
	 *
	 * `cacheCollapsedSizes` reads the DOM, so with culling on it only ever sees mounted containers
	 * — and at scale most collapsed containers are off screen, so they are laid out from their
	 * type's declared size instead (see `measure.ts`). Running it again as the user pans replaces
	 * those estimates with measurements without ever mounting the whole graph. Throttled because
	 * a pan fires move-end repeatedly, and deliberately silent: it fills the cache for the *next*
	 * layout rather than triggering one, since re-laying out mid-pan is exactly the jank the
	 * culling work exists to remove.
	 */
	const refreshCollapsedSizesInView = throttle(
		() => {
			if (!containerElement || !layoutState.layoutGraph) return;
			cacheCollapsedSizes(
				containerElement,
				layoutState.layoutGraph,
				get(collapsedContainers),
				layoutState.containerSizeCache
			);
		},
		COLLAPSED_SIZE_REFRESH_MS,
		{ leading: false, trailing: true }
	);

	function syncEdgeDisplayState() {
		const current = get(baseFlowEdges);
		const updated = computeEdgeDisplayUpdates(
			current,
			get(selectionStores.selectedNode),
			get(selectionStores.selectedEdge),
			get(searchHiddenNodeIds),
			get(tagHiddenNodeIds)
		);
		// Identity means nothing changed. Skipping the write matters on the hover
		// path, which calls this on every pointer enter/leave.
		if (updated !== current) baseFlowEdges.set(updated);
	}

	function handlePaneClick() {
		if (!viewportMoved) {
			clearSelection(selectionStores);
			clearEdgeHoverState();
			syncEdgeDisplayState();
		}
		viewportMoved = false;
		if (viewportMoveTimer) {
			clearTimeout(viewportMoveTimer);
			viewportMoveTimer = null;
		}
	}

	function handleEdgePointerEnter({ edge }: { edge: Edge }) {
		setEdgeHover(edge, true, get(edges));
		syncEdgeDisplayState();
	}

	function handleEdgePointerLeave({ edge }: { edge: Edge }) {
		setEdgeHover(edge, false, get(edges));
		syncEdgeDisplayState();
	}

	function handleSelectionChange({ nodes: selNodes }: { nodes: Node[]; edges: Edge[] }) {
		if (ignoreNextSelectionChange) {
			ignoreNextSelectionChange = false;
			return;
		}
		if (selNodes.length === 0 && !viewportMoved) {
			tick().then(() => {
				// Skip if a click handler has set an active selection, or a multi-selection is active
				if (
					get(selectionStores.selectedNode) ||
					get(selectionStores.selectedEdge) ||
					get(selectionStores.selectedNodes).length > 0
				)
					return;
				clearSelection(selectionStores);
				clearEdgeHoverState();
				syncEdgeDisplayState();
			});
			return;
		}
		handleBoxSelect(selNodes, selectionStores);
	}

	function handleNodeDragStop({
		targetNode
	}: {
		targetNode: Node | null;
		nodes: Node[];
		event: MouseEvent | TouchEvent;
	}) {
		if (onNodeDragStop && targetNode) onNodeDragStop(targetNode);
	}

	function handleReconnect(edge: Edge, newConnection: Connection) {
		if (onReconnect) onReconnect(edge, newConnection);
	}

	// --- Collapse controls ---

	function getCollapseLevelName(level: CollapseLevel): string {
		switch (level) {
			case 1:
				return topology_levelFullyCollapsed();
			case 2:
				return topology_levelContainersExpanded();
			case 3:
				return topology_levelSubcontainersExpanded();
			case 4:
				return topology_levelFullyExpanded();
		}
	}

	// Disabled when nothing further would change, not merely when the level is at its numeric
	// end. A view whose only root is collapsed_by_default has fewer distinct states than the
	// ladder has rungs, so the button can run out before the number does — and a button that
	// still looks live while doing nothing is worse than one that greys out.
	// `$collapsedContainers` is read so this re-evaluates when the rendered set changes.
	let expandDisabled = $derived(
		!!editMode ||
			($collapsedContainers &&
				nextEffectiveLevel('expand', topology.nodes, containerTypes, getInfrastructureRuleId()) ===
					null)
	);
	let collapseDisabled = $derived(
		!!editMode ||
			($collapsedContainers &&
				nextEffectiveLevel(
					'collapse',
					topology.nodes,
					containerTypes,
					getInfrastructureRuleId()
				) === null)
	);
	let collapseLevelTooltipCollapse = $derived(
		$collapseLevel > 1
			? `${common_collapse()}: ${getCollapseLevelName(($collapseLevel - 1) as CollapseLevel)}`
			: ''
	);
	let collapseLevelTooltipExpand = $derived(
		$collapseLevel < 4
			? `${common_expand()}: ${getCollapseLevelName(($collapseLevel + 1) as CollapseLevel)}`
			: ''
	);

	// Both step handlers only write the collapse stores; the relayout that
	// follows is a full asynchronous pipeline run (measure pass, ELK, and a
	// 350ms animation phase). Refitting on a fixed timer therefore fits the
	// *previous* graph — and nothing refits afterwards, because a collapse
	// change is neither `viewChanged` nor `topologyChanged` (`getStructureKey`
	// does not include collapse state). Hand the intent to the pipeline instead
	// and let the post-layout branch fit once the new layout exists.
	function handleStepCollapse() {
		if (editMode) return;
		clearSelection(selectionStores);
		stepCollapse(topology.nodes, containerTypes, getInfrastructureRuleId(), (idsLeftExpanded) => {
			for (const id of idsLeftExpanded) layoutState.seenAutoCollapseIds.add(id);
			layoutState.fitViewPending = true;
		});
	}

	function handleStepExpand() {
		if (editMode) return;
		clearSelection(selectionStores);
		// Marked seen before the collapse stores are written, not after: the write runs the
		// pipeline synchronously, so anything done afterwards is too late to affect the run it
		// caused. See `stepExpand`.
		stepExpand(topology.nodes, containerTypes, getInfrastructureRuleId(), (idsLeftExpanded) => {
			for (const id of idsLeftExpanded) layoutState.seenAutoCollapseIds.add(id);
			layoutState.fitViewPending = true;
		});
	}

	export function triggerStepExpand() {
		handleStepExpand();
	}
	export function triggerStepCollapse() {
		handleStepCollapse();
	}

	// Preview edges enter `edges` only once SvelteFlow has bounds for the handles they name.
	//
	// A new dependency's preview usually names a side no real edge on that node uses, so the node
	// never rendered that handle (`edgeHandlesByNode`) and SvelteFlow drops the edge with "Couldn't
	// create edge for source handle id". Publishing the handles renders their divs, but SvelteFlow
	// re-reads handle bounds from the DOM only when a node's size changes, and adding a handle does
	// not change it. So the endpoints are force-measured, and the preview is revealed on the frame
	// after that lands. `previewRun` discards a run the next preview has already superseded.
	const updateNodeInternals = useUpdateNodeInternals();
	let revealedPreview = $state<Edge[]>([]);
	let previewRun = 0;
	$effect(() => {
		const preview = $previewEdges;
		const run = ++previewRun;
		const handles = collectEdgeHandles(preview);
		previewEdgeHandlesByNode.set(handles);
		if (preview.length === 0) {
			revealedPreview = [];
			return;
		}
		void (async () => {
			await tick();
			if (run !== previewRun) return;
			updateNodeInternals([...handles.keys()]);
			// The hook measures on the next frame; this callback is queued after it.
			await new Promise((resolve) => requestAnimationFrame(resolve));
			if (run !== previewRun) return;
			revealedPreview = preview;
		})();
	});

	// Derive the xyflow `edges` store from three reactive sources:
	//   - baseFlowEdges: the real edges produced by the rebuild pipeline
	//   - revealedPreview: preview edges from the dependency editor, once measured (above)
	//   - editingDependencyId: when set, hide the real edge for that dep
	// This is the ONLY writer of `edges`. Exits and rebuilds are symmetric:
	// clearing editingDependencyId naturally restores the filtered real edge.
	//
	// Svelte's `$baseFlowEdges` auto-subscribe hits a compiler bug here, so
	// bridge the store into a $state and depend on that in the merge effect.
	let currentBaseFlowEdges = $state<Edge[]>([]);
	$effect(() => {
		return baseFlowEdges.subscribe((v) => {
			currentBaseFlowEdges = v;
		});
	});
	$effect(() => {
		const base = currentBaseFlowEdges;
		const preview = revealedPreview;
		const editingId = $editingDependencyId;
		const aggregatedOriginals = $aggregatedEdgeOriginals;

		// Hide any edge whose underlying dependency is being edited. An edge may
		// directly carry `data.dependency_id` (plain dep edge), be a bundled
		// representative with `data.bundleEdges` (an array of originals), or be
		// an aggregated collapse edge whose originals live in the
		// `aggregatedEdgeOriginals` store (keyed by edge.id).
		const matchesEditingDep = (e: Edge): boolean => {
			const data = e.data as
				| {
						dependency_id?: string;
						bundleEdges?: Array<{ dependency_id?: string }>;
				  }
				| undefined;
			if (data?.dependency_id === editingId) return true;
			if (data?.bundleEdges?.some((o) => o.dependency_id === editingId)) return true;
			const originals = aggregatedOriginals.get(e.id);
			if (originals?.some((o) => (o as { dependency_id?: string }).dependency_id === editingId))
				return true;
			return false;
		};

		const visibleReal = editingId ? base.filter((e) => !matchesEditingDep(e)) : base;
		edges.set([...visibleReal, ...preview]);
	});
</script>

<div
	class="h-full w-full overflow-hidden !p-0"
	class:card={!isEmbed}
	class:card-static={!isEmbed}
	class:collapse-transition={animatingCollapse}
	style:visibility={coldLoadMeasure ? 'hidden' : 'visible'}
	bind:this={containerElement}
>
	<SvelteFlow
		nodes={$nodes}
		edges={$edges}
		{nodeTypes}
		edgeTypes={customEdgeTypes}
		onpaneclick={handlePaneClick}
		onedgeclick={handleEdgeClick}
		onnodeclick={handleNodeClick}
		onedgepointerenter={handleEdgePointerEnter}
		onedgepointerleave={handleEdgePointerLeave}
		onnodedragstop={readonly ? undefined : handleNodeDragStop}
		onreconnect={readonly ? undefined : handleReconnect}
		onselectionchange={handleSelectionChange}
		onmove={handleMove}
		onmoveend={handleMoveEnd}
		fitView={true}
		onlyRenderVisibleElements={cullOffscreen}
		minZoom={derivedMinZoom}
		maxZoom={MAX_ZOOM}
		noPanClass="nopan"
		snapGrid={[25, 25]}
		nodesDraggable={!readonly}
		nodesConnectable={!readonly}
		elementsSelectable={true}
		selectionOnDrag={true}
		selectionKey="Shift"
		panOnDrag={true}
		zoomOnScroll={true}
		proOptions={{ hideAttribution }}
	>
		<Background
			variant={BackgroundVariant.Dots}
			bgColor="var(--color-topology-bg)"
			gap={50}
			size={1}
		/>

		{#if showControls}
			<TopologySidebarControls
				{editMode}
				{onToggleEditMode}
				{onOpenShortcuts}
				{onOpenSearch}
				{sidebarCollapsed}
				onStepExpand={handleStepExpand}
				onStepCollapse={handleStepCollapse}
				onFitView={() => triggerFitView()}
				{expandDisabled}
				{collapseDisabled}
				collapseLevel={$collapseLevel}
				{collapseLevelTooltipExpand}
				{collapseLevelTooltipCollapse}
			/>
		{/if}

		{#if (showMinimap !== undefined ? showMinimap : $topologyOptions.local.show_minimap) && !$isExporting}
			<MiniMap
				position="bottom-left"
				width={MINIMAP_WIDTH_PX}
				height={MINIMAP_HEIGHT_PX}
				bgColor={themeStore.resolvedTheme === 'dark' ? '#1f2937' : '#ffffff'}
				nodeColor={themeStore.resolvedTheme === 'dark' ? '#6b7280' : '#9ca3af'}
				maskColor={themeStore.resolvedTheme === 'dark'
					? 'rgba(17, 24, 39, 0.7)'
					: 'rgba(243, 244, 246, 0.7)'}
				maskStrokeColor={themeStore.resolvedTheme === 'dark' ? '#374151' : '#d1d5db'}
			/>
		{/if}

		{#if $isRenderingTopology || detailHidden}
			<!--
				One pill for what the view is doing to itself, in flow chrome rather than in the graph
				so it belongs to the view and not to any node.

				Both lines can be true at once — a large graph is usually the reason a run is slow and
				the reason detail is off — so they stack rather than compete. Absent during export,
				because `shouldSimplify` suspends there and a run is never in flight mid-capture.
			-->
			<div class="detail-hidden-badge">
				{#if $isRenderingTopology}
					<span class="detail-hidden-row">
						<span class="lod-spinner" aria-hidden="true"></span>
						{common_rendering()}
					</span>
				{/if}
				{#if detailHidden}
					<span class="detail-hidden-row">
						{topology_detailSimplified()}
					</span>
				{/if}
			</div>
		{/if}

		{#if showBranding}
			<a
				href="https://scanopy.net?utm_source={isEmbed
					? 'embed'
					: 'share'}&utm_medium=referral&utm_campaign=created_with"
				target="_blank"
				rel="noopener noreferrer"
				class="branding-badge"
			>
				<img src="/logos/scanopy-logo.png" alt="Scanopy" class="h-4 w-4" />
				<span>Powered by Scanopy</span>
			</a>
		{/if}
	</SvelteFlow>
</div>

<style>
	/*
	 * Says the canvas is drawing boxes rather than contents.
	 *
	 * Bottom-centre: out of the way of the minimap (bottom-left), the sidebar controls (right) and
	 * the branding badge, and in the one place nothing else claims. Non-interactive — it reports a
	 * state, it is not a control, and it must not eat pointer events over the graph.
	 */
	.detail-hidden-badge {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 2px;
		position: absolute;
		bottom: 12px;
		left: 50%;
		transform: translateX(-50%);
		z-index: 5;
		padding: 4px 10px;
		border-radius: 9999px;
		font-size: 0.75rem;
		line-height: 1.2;
		white-space: nowrap;
		pointer-events: none;
		color: var(--color-text-tertiary, #64748b);
		background: var(--color-topology-node-bg, #fff);
		border: 1px solid var(--color-border, #e2e8f0);
		box-shadow: 0 1px 3px rgb(0 0 0 / 0.08);
	}

	.detail-hidden-row {
		display: flex;
		align-items: center;
		gap: 5px;
	}

	/*
	 * A ring drawn in CSS, not an icon.
	 *
	 * The whole point of this indicator is to stay alive while the main thread is not: ELK is the
	 * bundled build and runs in-process, so for the eighteen to twenty-two seconds of a layout on a
	 * large estate, no frame callback fires and nothing scripted can advance. A transform animation
	 * can still run, but only from the compositor, and only if the element was promoted — and an
	 * inline SVG generally is not, which is why the lucide spinner rendered and then sat there.
	 *
	 * A single element with a border and a radius is the shape most reliably promoted, so the ring
	 * is borders rather than an icon. `will-change` asks for the layer explicitly; the animation is
	 * pure `transform`, which is the one property the compositor can drive on its own.
	 *
	 * This also matches what the caret did for the same reason — see `ContainerHeader` — and costs
	 * one element instead of the five a lucide icon brings.
	 */
	.lod-spinner {
		display: block;
		width: 11px;
		height: 11px;
		flex-shrink: 0;
		border-radius: 9999px;
		border: 1.5px solid var(--color-topology-lod-stroke);
		border-top-color: transparent;
		will-change: transform;
		animation: lod-spin 0.7s linear infinite;
	}

	@keyframes lod-spin {
		to {
			transform: rotate(360deg);
		}
	}

	.branding-badge {
		position: absolute;
		bottom: 10px;
		right: 10px;
		display: flex;
		align-items: center;
		gap: 6px;
		color: var(--color-text-muted);
		font-size: 12px;
		text-decoration: none;
		z-index: 5;
		transition: color 0.2s;
	}

	.branding-badge:hover {
		color: var(--color-text-secondary);
	}
</style>
