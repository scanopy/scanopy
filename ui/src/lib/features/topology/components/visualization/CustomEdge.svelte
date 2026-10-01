<script lang="ts">
	import {
		type EdgeProps,
		getSmoothStepPath,
		BaseEdge,
		EdgeLabel,
		getBezierPath,
		getStraightPath,
		EdgeReconnectAnchor
	} from '@xyflow/svelte';
	import { topologyOptions } from '../../queries';
	import { useTopology, selectedTopologyId } from '../../context';
	import { edgeTypes } from '$lib/shared/stores/metadata';
	import { createColorHelper, type Color } from '$lib/shared/utils/styling';
	import type { TopologyEdge, RenderableTopology } from '../../types/base';
	import { isExporting, detailSimplified, hoveredEdgeType } from '../../interactions';
	import {
		getLinkEvidenceTag,
		isDottedEdge,
		isOverlayEdge
	} from '../../layout/edge-classification';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { ELEMENT_TEXT_WIDTH_PX } from '../../pipeline/build-flow-nodes';

	let {
		id,
		sourceX,
		sourceY,
		sourcePosition,
		targetX,
		targetY,
		targetPosition,
		sourceHandleId,
		targetHandleId,
		label,
		data,
		interactionWidth
	}: EdgeProps = $props();

	const topo = useTopology();
	const topoStore = topo.fromContext ? topo.store : null;
	let topology = $derived(
		topoStore
			? $topoStore
			: (topo.query?.data?.find((t) => t.id === $selectedTopologyId) as
					RenderableTopology | undefined)
	);

	const nodes = $derived(topology?.nodes ?? []);

	const edgeData = $derived(data as TopologyEdge | undefined);

	/**
	 * Half of what a name gets on an element card — an edge label carries two of them plus a
	 * separator, where a card carries one.
	 *
	 * Truncating by width rather than character count is what makes this match the card: both use
	 * the same CSS ellipsis at the same 12px, so a name that fits on a card fits here in the same
	 * proportion regardless of how wide its glyphs are.
	 */
	const ENDPOINT_MAX_WIDTH_PX = ELEMENT_TEXT_WIDTH_PX / 2;

	/**
	 * The two endpoint names in a `a ↔ b` label, or `null` for any other shape.
	 *
	 * Both endpoint-naming edge types build their label this way — `PhysicalLink` from two
	 * interface names, `NeighborLink` from two host names — and either half can be long on its own.
	 * Splitting lets each side truncate independently, so the separator survives even when both
	 * ends are cut; a single truncation across the whole string loses it and with it the only clue
	 * that the label names two things.
	 */
	let labelEndpoints = $derived.by((): [string, string] | null => {
		const parts = label?.split(' ↔ ');
		return parts?.length === 2 ? [parts[0], parts[1]] : null;
	});

	// Bundle detection
	const anyEdgeData = $derived(data as Record<string, unknown> | undefined);
	let isBundle = $derived(!!anyEdgeData?.isBundle);
	let bundleStrokeWidth = $derived((anyEdgeData?.bundleStrokeWidth as number) ?? 2);
	let bundleIsOverlay = $derived(!!anyEdgeData?.bundleIsOverlay);
	let hasFanOffset = $derived(anyEdgeData?.bundleFanTotal != null);
	let fanIndex = $derived((anyEdgeData?.bundleFanIndex as number) ?? 0);
	let fanTotal = $derived((anyEdgeData?.bundleFanTotal as number) ?? 0);
	// Endpoint hidden state — centralized in getEdgeDisplayState(), passed via edge data
	let isEndpointHiddenByTagFilter = $derived(
		(anyEdgeData?.isEndpointTagHidden as boolean) ?? false
	);
	let isEndpointHiddenBySearch = $derived(
		(anyEdgeData?.isEndpointSearchHidden as boolean) ?? false
	);
	const edgeTypeMetadata = $derived(edgeData ? edgeTypes.getMetadata(edgeData.edge_type) : null);

	// Whether the evidence for this link has gone stale while both its ports carry on being
	// scanned. Additive — an amber chip beside the label, matching the ruling the stale pill on a
	// node already follows: stroke, dash and opacity are all spoken for (dashed means
	// device-level in L2, opacity is the search/filter channel), so staleness gets its own mark
	// rather than overloading one of theirs.
	const linkEvidenceTag = $derived(
		edgeData
			? getLinkEvidenceTag(edgeData, topology?.interfaces ?? [], topology?.neighbours ?? [])
			: null
	);

	// Get dependency reactively - updates when dependencies store changes
	let group = $derived.by(() => {
		if (!topology?.dependencies || !edgeTypeMetadata || !edgeData) return null;
		if ('dependency_id' in edgeData) {
			return topology.dependencies.find((g) => g.id == edgeData.dependency_id) || null;
		}
		return null;
	});

	/**
	 * Edges go with the card detail below the zoom threshold — see `shouldSimplify`.
	 *
	 * At the zoom a large graph fits at, an edge is a hairline between two nodes that are
	 * themselves a couple of pixels wide: it conveys nothing and there are as many of them as
	 * there are links. Folded into the existing `hideEdge` gate rather than filtering the store,
	 * because the store is also what `getLayoutedEdges` reads to decide which endpoints to force
	 * into the mounted set — dropping edges from it would change what is culled, not just what is
	 * drawn.
	 */
	let simplified = $derived($detailSimplified);

	let isPreview = $derived(!!(edgeData as Record<string, unknown> | undefined)?.is_preview);

	/**
	 * A preview edge is never hidden.
	 *
	 * The exemption used to sit inside the hide-by-type arm alone, so that hiding `Dependency`
	 * edges did not also hide the one being drawn. Simplification was then added in front of it as
	 * `simplified || …`, which short-circuits: past the zoom threshold every edge was hidden and
	 * the exemption was never reached. `detailSimplified` is graph-wide and any estate large enough
	 * to fit below the threshold has it permanently on, so dependency previews stopped drawing at
	 * all on exactly the networks where dependencies get drawn.
	 */
	let hideEdge = $derived(
		!isPreview &&
			(simplified ||
				(edgeData ? $topologyOptions.local.hide_edge_types.includes(edgeData.edge_type) : false))
	);

	// Any non-solid stroke gets the overlay treatment (thinner, dimmed until highlighted); the
	// specific stroke only picks the dash pattern below.
	let isOverlay = $derived(isBundle ? bundleIsOverlay : edgeData ? isOverlayEdge(edgeData) : false);
	let isDotted = $derived(!isBundle && edgeData ? isDottedEdge(edgeData) : false);

	// Display state is passed as edge data from BaseTopologyViewer, which computes it
	// from selection/hover stores. This avoids store subscription issues inside SvelteFlow's
	// component tree — data props always propagate reliably.
	let shouldShowFull = $derived((anyEdgeData?.shouldShowFull as boolean) ?? false);
	let shouldAnimate = $derived((anyEdgeData?.shouldAnimate as boolean) ?? false);
	let isSelected = $derived((anyEdgeData?.isSelected as boolean) ?? false);

	// Calculate edge color - use group color if available, otherwise use edge type color
	let edgeColorHelper = $derived.by(() => {
		if (group?.color) {
			return createColorHelper(group.color);
		}
		// Preview edges carry their own color since they have no real group
		const anyData = edgeData as Record<string, unknown> | undefined;
		if (anyData?.is_preview && anyData?.preview_color) {
			return createColorHelper(anyData.preview_color as Color);
		}
		if (!edgeData) {
			return createColorHelper('Gray');
		}
		return edgeTypes.getColorHelper(edgeData.edge_type);
	});

	// Determine if this edge should use the two-color dashed effect
	let isGroupEdge = $derived(edgeTypeMetadata?.is_dependency_edge ?? false);
	let useMultiColorDash = $derived((isGroupEdge && shouldShowFull) || isPreview);

	// Edge type hover highlight
	let isEdgeTypeHovered = $derived(
		$hoveredEdgeType !== null &&
			edgeData?.edge_type != null &&
			$hoveredEdgeType.edgeTypes.includes(edgeData.edge_type)
	);
	let isAnotherEdgeTypeHovered = $derived($hoveredEdgeType !== null && !isEdgeTypeHovered);

	// Aggregated edge support
	let isAggregated = $derived(!!(edgeData as Record<string, unknown> | undefined)?.isAggregated);
	let aggregatedCount = $derived(
		((edgeData as Record<string, unknown> | undefined)?.aggregatedCount as number) ?? 1
	);

	// Calculate base edge properties
	let baseStrokeWidth = $derived.by(() => {
		if (isAggregated) return Math.min(2 + aggregatedCount, 8);
		if (isBundle) return bundleStrokeWidth;
		if (isEdgeTypeHovered) return 3;
		if (!$topologyOptions.local.no_fade_edges && (shouldShowFull || isPreview)) return 3;
		if (isOverlay) return 1.5;
		return 2;
	});
	let baseOpacity = $derived.by(() => {
		if ($isExporting) return 1;
		// Preview edges always full opacity
		if (isPreview) return 1;
		// Edge type hover: matching edges full opacity, non-matching fade
		if (isEdgeTypeHovered) return 1;
		if (isAnotherEdgeTypeHovered) return 0.2;
		// Fade if either endpoint is hidden by tag or search filter
		if (isEndpointHiddenByTagFilter) return 0.4;
		if (isEndpointHiddenBySearch) return 0.4;
		// Overlay edges: reduced opacity unless highlighted
		if (isOverlay && !shouldShowFull) return 0.5;
		// Fade based on selection state
		if (!$topologyOptions.local.no_fade_edges && !shouldShowFull) return 0.4;
		return 1;
	});
	// Labels follow the same fade behavior as their parent edges
	let labelOpacity = $derived.by(() => {
		if ($isExporting) return 1;
		if (isEdgeTypeHovered) return 1;
		if (isAnotherEdgeTypeHovered) return 0.2;
		if (isEndpointHiddenByTagFilter) return 0.4;
		if (isEndpointHiddenBySearch) return 0.4;
		const hasActiveSelection = !!(anyEdgeData?.hasActiveSelection as boolean);
		if (!$topologyOptions.local.no_fade_edges && hasActiveSelection && !shouldShowFull) return 0.4;
		return 1;
	});

	// Calculate edge style for primary layer (dashed white overlay for group edges, or normal edge)
	let edgeStyle = $derived.by(() => {
		// For group edges with multi-color dash: white dashes
		// For non-group dashed edges: use standard 5 5 pattern with edge color
		let strokeColor = edgeColorHelper.rgb;
		let dashArray = '';

		const isDark =
			typeof document !== 'undefined' && document.documentElement.classList.contains('dark');
		if (useMultiColorDash && isSelected) {
			// Group edge currently selected
			strokeColor = isDark ? 'rgba(0, 0, 0, 0.4)' : 'rgba(255, 255, 255, 0.6)';
		} else if (useMultiColorDash && !isSelected) {
			// Other group edges, subtler highlight
			strokeColor = isDark ? 'rgba(0, 0, 0, 0.15)' : 'rgba(255, 255, 255, 0.4)';
		} else if (!isGroupEdge && isOverlay) {
			dashArray = isDotted ? 'stroke-dasharray: 1 4;' : 'stroke-dasharray: 6 3;';
		}

		return `stroke: ${strokeColor}; stroke-width: ${baseStrokeWidth}px; opacity: ${baseOpacity}; ${dashArray} transition: opacity 0.2s ease-in-out, stroke-width 0.2s ease-in-out;`;
	});

	// Calculate edge style for secondary solid layer (only for group edges when shown full)
	let solidBaseStyle = $derived.by(() => {
		if (!useMultiColorDash) return '';
		// Solid base color underneath the white dashes
		return `stroke: ${edgeColorHelper.rgb}; stroke-width: ${baseStrokeWidth}px; opacity: ${baseOpacity}; transition: opacity 0.2s ease-in-out, stroke-width 0.2s ease-in-out;`;
	});

	// Calculate dynamic offset for multi-hop edges
	function calculateDynamicOffset(isMultiHop: boolean): number {
		if (!isMultiHop) {
			return 20; // Default offset for single-hop
		}

		// Determine routing direction from edge handles
		const routingLeft = sourceHandleId == 'Left' || targetHandleId == 'Left';

		// Find the bounding box of the edge path
		const minX = Math.min(sourceX, targetX);
		const maxX = Math.max(sourceX, targetX);
		const minY = Math.min(sourceY, targetY);
		const maxY = Math.max(sourceY, targetY);

		let maxOutcrop = 0;

		// Check all nodes to find intermediate subnets
		for (const node of nodes) {
			// Skip if node is outside the vertical range of the edge
			if (node.position.y <= minY || node.position.y >= maxY) {
				continue;
			}

			// Check if this node is a subnet in the path
			if (node.node_type == 'Container') {
				const nodeLeft = node.position.x;
				const nodeRight = node.position.x + (node.size.x || 0);

				if (routingLeft) {
					// Check how far left this node extends beyond our leftmost point
					const outcrop = minX - nodeLeft;
					maxOutcrop = Math.max(maxOutcrop, outcrop);
				} else {
					// Check how far right this node extends beyond our rightmost point
					const outcrop = nodeRight - maxX;
					maxOutcrop = Math.max(maxOutcrop, outcrop);
				}
			}
		}

		// Return calculated offset with padding, or minimum offset
		const padding = 50;
		const minimumOffset = 100;
		return Math.max(minimumOffset, maxOutcrop + padding);
	}

	// Helper function to get the path calculation function based on edge style
	function getPathFunction(edge_style: string) {
		const isMultiHop = (edgeData?.is_multi_hop as boolean) || false;
		const offset = calculateDynamicOffset(isMultiHop);

		// Apply fan offset for expanded bundle edges
		let fanOffsetX = 0;
		let fanOffsetY = 0;
		if (hasFanOffset && fanTotal > 1) {
			const spacing = 8;
			const fanOffset = (fanIndex - (fanTotal - 1) / 2) * spacing;
			// Offset perpendicular to edge direction
			const dx = targetX - sourceX;
			const dy = targetY - sourceY;
			const len = Math.sqrt(dx * dx + dy * dy) || 1;
			// Perpendicular unit vector
			fanOffsetX = (-dy / len) * fanOffset;
			fanOffsetY = (dx / len) * fanOffset;
		}

		const basePathProperties = {
			sourceX: sourceX + fanOffsetX,
			sourceY: sourceY + fanOffsetY,
			sourcePosition,
			targetX: targetX + fanOffsetX,
			targetY: targetY + fanOffsetY,
			targetPosition
		};

		switch (edge_style) {
			case 'Straight':
				return getStraightPath(basePathProperties);
			case 'Smoothstep':
			case 'SmoothStep':
			case 'Step':
				return getSmoothStepPath({
					...basePathProperties,
					borderRadius: 10,
					offset
				});
			case 'Bezier':
			case 'SimpleBezier':
				return getBezierPath(basePathProperties);
			default:
				return getSmoothStepPath({
					...basePathProperties,
					borderRadius: 10,
					offset
				});
		}
	}

	// Calculate edge path and label position - DRY approach
	let pathData = $derived.by(() => {
		// Use group edge_style if available, then preview edge style, otherwise edge type metadata
		const anyData = edgeData as Record<string, unknown> | undefined;
		const edge_style = group
			? group.edge_style
			: ((anyData?.preview_edge_style as string) ?? edgeTypeMetadata?.edge_style ?? 'SmoothStep');
		return getPathFunction(edge_style);
	});

	let edgePath = $derived(pathData[0]);
	let labelX = $derived(pathData[1]);
	let labelY = $derived(pathData[2]);

	let labelOffsetX = $state(0);
	let labelOffsetY = $state(0);
	let isDragging = $state(false);
	let dragStartX = 0;
	let dragStartY = 0;

	function onDragStart(event: DragEvent) {
		isDragging = true;
		dragStartX = event.clientX - labelOffsetX;
		dragStartY = event.clientY - labelOffsetY;
	}

	function onDrag(event: DragEvent) {
		if (event.clientX === 0 && event.clientY === 0) return; // Ignore end drag event
		labelOffsetX = event.clientX - dragStartX;
		labelOffsetY = event.clientY - dragStartY;
	}

	function onDragEnd() {
		isDragging = false;
	}

	let reconnecting = $state(false);
</script>

{#if edgeData}
	{#if isSelected}
		<EdgeReconnectAnchor
			bind:reconnecting
			type="source"
			position={{ x: sourceX, y: sourceY }}
			class={{}}
			style={!reconnecting
				? `background: ${edgeColorHelper.rgb}; border: 2px solid var(--color-border); border-radius: 100%; width: 12px; height: 12px;`
				: 'background: transparent; border: 2px solid var(--color-border); border-radius: 100%; width: 12px; height: 12px;'}
		/>
		<EdgeReconnectAnchor
			bind:reconnecting
			type="target"
			position={{ x: targetX, y: targetY }}
			style={!reconnecting
				? `background: ${edgeColorHelper.rgb}; border: 2px solid var(--color-border); border-radius: 100%; width: 12px; height: 12px;`
				: 'background: transparent; border: 2px solid var(--color-border); border-radius: 100%; width: 12px; height: 12px;'}
		/>
	{/if}

	{#if !hideEdge && !reconnecting}
		<!-- Solid base layer for group edges when shown full (rendered first, behind) -->
		{#if useMultiColorDash}
			<BaseEdge
				path={edgePath}
				style={solidBaseStyle}
				{id}
				interactionWidth={0}
				class="solid-base"
			/>
		{/if}

		<!-- Primary edge layer (white dashes for group edges when shown, normal for everything else) -->
		<BaseEdge
			path={edgePath}
			style="{edgeStyle}{shouldAnimate
				? ' stroke-dasharray: 5; animation: dashdraw 0.5s linear infinite;'
				: ''}"
			{id}
			interactionWidth={interactionWidth || 20}
			class={useMultiColorDash ? 'dashed-overlay' : ''}
		/>

		<!-- A stale link keeps the label card even where the label itself was stripped (an edge
		     wholly inside one container drops it), or the one link that has stopped being
		     evidenced would be the one carrying no mark. -->
		{#if !isBundle && (label || linkEvidenceTag)}
			<EdgeLabel
				x={labelX + labelOffsetX}
				y={labelY + labelOffsetY}
				style="background: none; pointer-events: none;"
			>
				<div
					class="card text-secondary nopan flex items-center gap-2"
					style="font-size: 12px; font-weight: 500; padding: 0.5rem 0.75rem; border-color: var(--color-border); cursor: {isDragging
						? 'grabbing'
						: 'grab'}; pointer-events: auto; opacity: {labelOpacity}; transition: opacity 0.2s ease-in-out;"
					draggable="true"
					role="button"
					tabindex="0"
					ondragstart={onDragStart}
					ondrag={onDrag}
					ondragend={onDragEnd}
				>
					{#if labelEndpoints}
						<span class="flex min-w-0 items-center gap-1" title={label}>
							<span class="truncate" style="max-width: {ENDPOINT_MAX_WIDTH_PX}px;"
								>{labelEndpoints[0]}</span
							>
							<span class="flex-shrink-0">↔</span>
							<span class="truncate" style="max-width: {ENDPOINT_MAX_WIDTH_PX}px;"
								>{labelEndpoints[1]}</span
							>
						</span>
					{:else if label}{label}{/if}
					{#if linkEvidenceTag}
						<Tag {...linkEvidenceTag} pill nativeTooltip />
					{/if}
				</div>
			</EdgeLabel>
		{/if}
	{/if}
{/if}

<style>
	/* We control animation ourselves via inline styles and shouldAnimate data prop.
	   Override SvelteFlow's .animated class to prevent double animation. */
	:global(.svelte-flow__edge.animated .svelte-flow__edge-path) {
		stroke-dasharray: initial !important;
		animation: none !important;
	}

	@keyframes dashdraw {
		from {
			stroke-dashoffset: 10;
		}
	}
</style>
