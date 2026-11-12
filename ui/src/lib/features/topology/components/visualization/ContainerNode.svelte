<script lang="ts">
	import { type NodeProps, type ResizeDragEvent, type ResizeParams } from '@xyflow/svelte';
	import NodeHandles from './NodeHandles.svelte';
	// NodeResizeControl — unused while the resize controls below are commented out.
	// import { NodeResizeControl } from '@xyflow/svelte';
	import { createColorHelper } from '#lib/shared/utils/styling.ts';
	import type { ColorStyle } from '#lib/shared/utils/styling.ts';
	import { containerTypes } from '#lib/shared/stores/metadata.ts';
	import {
		// useUpdateNodeResizeMutation — DISABLED (container resize is not persisted)
		topologyOptions,
		activeView,
		selectedNode as globalSelectedNode,
		selectedEdge as globalSelectedEdge
	} from '../../queries';
	import { useTopology, selectedTopologyId } from '../../context';
	import type { RenderableTopology, TopologyNode } from '../../types/base';
	import { buildContainerRender } from '../../container-render-data';
	import { topologyLookup } from '../../resolvers';
	import { entityTooltip } from '#lib/shared/entity-tooltip.ts';
	import type { TooltipOptions } from '#lib/shared/actions/tooltip.ts';
	import { useSitesQuery } from '#lib/features/sites/queries.ts';
	import type { Writable } from 'svelte/store';
	import { getContext } from 'svelte';
	import { editModeEnabled } from '../../state';
	import { matchesHoveredMetadata, UNTAGGED_SENTINEL } from '../../interactions';
	import * as sharedStores from '../../reactive-stores.svelte';
	import { toggleCollapse } from '../../collapse';
	import type { Node, Edge } from '@xyflow/svelte';
	import type { IconComponent } from '#lib/shared/utils/types.ts';
	import ContainerHeader from './ContainerHeader.svelte';
	import { CONTAINER_HANDLE_SIZE_PX } from '../../pipeline/build-flow-nodes';
	import { nodeDetail } from '../../pipeline/render-mode';
	import { useDetailStaging } from '../../detail-staging.svelte';

	// Shared, refcounted views over the module-level stores — see
	// `reactive-stores.svelte.ts`. One subscription serves every node component.
	let connectedNodes = $derived(sharedStores.connectedNodes.current);
	let edgeHandles = $derived(sharedStores.edgeHandles.current);
	let isExportingValue = $derived(sharedStores.exporting.current);

	/**
	 * Drop the header chrome once it is too small to read — see `shouldSimplify`.
	 *
	 * The container's own box is drawn by the expanded-state div below and by the collapsed
	 * variants, so hiding the title costs no geometry: the header is a pill positioned outside the
	 * container, or padding-area text inside it, and neither is what ELK sized.
	 *
	 * Note the comment further down about resize controls, which once tested `viewport.zoom > 0.5`
	 * and were changed because it "subscribed every container node to every pan/zoom frame". This
	 * reads the shared zoom instead of the viewport: a pan changes x and y only, so it reaches no
	 * container, and a zoom recomputes each tier once. Reading `useViewport()` here recomputed
	 * every mounted container on every pan frame, 39 ms per frame at 3,532 containers.
	 */
	let searchHiddenNodes = $derived(sharedStores.searchHiddenNodes.current);
	let searchContainerMap = $derived(sharedStores.searchContainerMatches.current);
	let currentHoveredTag = $derived(sharedStores.currentHoveredTag.current);
	let currentHoveredMetadata = $derived(sharedStores.currentHoveredMetadata.current);
	let collapsedNodes = $derived(sharedStores.collapsedNodes.current);

	// `selected` is read only by the commented-out resize controls.
	let { id, data, width, height }: NodeProps = $props();

	const topo = useTopology();
	const topoStore = topo.fromContext ? topo.store : null;
	let topology = $derived(
		topoStore
			? $topoStore
			: (topo.query?.data?.find((t) => t.id === $selectedTopologyId) as
					RenderableTopology | undefined)
	);
	// Container resize is DISABLED — size changes are no longer persisted (the
	// graph builds on request and ELK re-lays out every render, so there's no
	// mechanism to save them). Kept commented for revival:
	// const updateNodeResizeMutation = useUpdateNodeResizeMutation();

	// Try to get selection from context (for share/embed pages), fallback to global store
	const selectedNodeContext = getContext<Writable<Node | null> | undefined>('selectedNode');
	const selectedEdgeContext = getContext<Writable<Edge | null> | undefined>('selectedEdge');
	let selectedNode = $derived(
		selectedNodeContext ? $selectedNodeContext : $globalSelectedNode
	) as Node | null;
	let selectedEdge = $derived(
		selectedEdgeContext ? $selectedEdgeContext : $globalSelectedEdge
	) as Edge | null;

	// Search match state for collapsed containers
	let searchMatchCount = $derived(searchContainerMap.get(id)?.length ?? 0);
	let hasSearchMatches = $derived(searchMatchCount > 0);
	let searchHighlightRingStyle = $derived(
		hasSearchMatches ? 'box-shadow: 0 0 0 2px rgb(59 130 246);' : ''
	);

	// Fade signals "focus elsewhere" (search, selection) — not filter state.
	// Filter hides remove containers structurally upstream of this component.
	let shouldFadeOut = $derived.by(() => {
		if (isExportingValue) return false;
		if (searchHiddenNodes.has(id) && !hasSearchMatches) return true;
		if (!selectedNode && !selectedEdge) return false;
		return !connectedNodes.has(id);
	});

	let nodeOpacity = $derived(shouldFadeOut ? 0.3 : 1);

	// Container metadata (drives all rendering decisions)
	let containerType = $derived(
		((data as Record<string, unknown>)?.container_type as string) ?? 'Subnet'
	);
	let containerMeta = $derived(containerTypes.getMetadata(containerType));
	let titleStyle = $derived(containerMeta.title_style);
	let isSubcontainer = $derived(containerMeta.is_subcontainer);
	let fillIcon = $derived(containerMeta.fill_icon ?? false);
	let isCollapsible = $derived(containerMeta.is_collapsible && !$editModeEnabled);
	let hasBorder = $derived(containerMeta.has_border);

	let isCollapsed = $derived(collapsedNodes.has(id));
	let childCount = $derived(((data as Record<string, unknown>)?.childCount as number) ?? 0);

	// Everything the header shows is computed by one function in `container-render-data.ts`,
	// shared with the diagram exports. Do not derive header content locally; add to that function.
	const sitesQuery = useSitesQuery();
	let render = $derived(
		topology
			? buildContainerRender({
					nodeId: id,
					node: data as TopologyNode,
					topology,
					activeView: $activeView,
					options: $topologyOptions,
					sites: sitesQuery.data ?? [],
					isCollapsed
				})
			: null
	);
	let containerTags = $derived(render?.tags ?? []);
	// The entity this container stands for. Grouping containers (categories, tags, stacks) have
	// none, so they get no stale pill and no filter-hover ring.
	let entity = $derived(render?.entity);
	let entityType = $derived(render?.entityType);
	let entitySite = $derived(render?.entitySite);
	let entityColor = $derived(render?.entityColor ?? null);
	let chipIcon = $derived(render?.chipIcon ?? null);
	let staleTag = $derived(render?.staleTag ?? null);
	let childSummary = $derived(render?.childSummary ?? '');
	let ungroupedSummary = $derived(render?.ungroupedSummary ?? '');
	let headerText = $derived(render?.headerText ?? '');
	let iconComponent: IconComponent | null = $derived(render?.iconComponent ?? null);
	let serviceDefId = $derived(render?.serviceDefinition ?? undefined);
	let deviceType = $derived(render?.deviceType ?? null);
	let colorHelper: ColorStyle = $derived(
		render?.colorHelper ?? containerTypes.getColorHelper(containerType)
	);
	let isInfraRule = $derived(render?.isInfraRule ?? false);
	let groupLabels = $derived(render?.groupLabels ?? []);
	let resolvedSubgroups = $derived(render?.subgroups ?? []);

	let nodeStyle = $derived.by(() => {
		const parts = [
			width != null ? `width: ${width}px` : '',
			height != null ? `height: ${height}px` : ''
		].filter(Boolean);
		return parts.length > 0 ? parts.join('; ') + ';' : '';
	});

	// Hover text for the header: the entity the container stands for, through the same tooltip
	// its card would show; a grouping container with no entity names itself and what it holds.
	let headerTooltip: TooltipOptions['content'] = $derived.by(() => {
		if (topology && entity && entityType) {
			const subject = { entityType, entity };
			const lookup = topologyLookup(topology);
			const name = headerText || undefined;
			return () => entityTooltip(subject.entityType, subject.entity, { lookup, name });
		}
		if (!headerText) return null;
		return { title: headerText, sections: childSummary ? [{ text: childSummary }] : [] };
	});

	/**
	 * What this container draws — see `nodeDetail`.
	 *
	 * Whether the graph is past full detail is decided once in the viewer (`detailSimplified`), since
	 * it depends on the size of the whole graph. What this container then *draws* is keyed on its own
	 * on-screen box, not on zoom alone. At the zoom a large L2 graph
	 * fits at, a switch is 211px and the port-status group inside a host is 2.7px; the first cut at
	 * this used one global boolean and gave both of them the same 12px label, so a grouping box
	 * ended up wearing a name four times wider than itself.
	 *
	 * Nothing is named below full detail — see `nodeDetail` for why a container cannot draw its own
	 * label inside its box. Names return with the real header at `DETAIL_ZOOM`.
	 */
	// Detail is staged on a large graph: this container asks for its full markup when it mounts and
	// draws as a box until it is granted. See `detail-staging.svelte.ts`.
	const detailStaging = useDetailStaging();
	let rootElement = $state<HTMLElement>();
	$effect(() => {
		const nodeId = id;
		detailStaging?.request(nodeId, rootElement);
		return () => detailStaging?.release(nodeId);
	});
	let detailMounted = $derived(
		detailStaging
			? detailStaging.hasDetail(id, sharedStores.simplified.current)
			: !sharedStores.simplified.current
	);
	let detailTier = $derived(
		nodeDetail({
			detail: detailMounted,
			screenWidth: (width ?? 0) * sharedStores.zoom.current,
			isSubcontainer
		})
	);
	let simplified = $derived(detailTier !== 'full');
	// Full markup that is not shown yet: mounted ahead of the detail zoom, drawn as the box.
	let veiled = $derived(!simplified && sharedStores.simplified.current);

	let tagHoverRingStyle = $derived.by(() => {
		// Filter-value hover: ring the box when the entity it stands for carries the hovered value,
		// the same ring element cards get.
		if (
			currentHoveredMetadata &&
			entityType &&
			matchesHoveredMetadata(entity, entityType, currentHoveredMetadata, entitySite)
		) {
			const ch = createColorHelper(
				currentHoveredMetadata.color as Parameters<typeof createColorHelper>[0]
			);
			return `box-shadow: 0 0 0 3px ${ch.rgb};`;
		}
		if (!currentHoveredTag) return '';
		// An Application box stands for the app tag it is named after, so hovering that tag's chip
		// rings the box as well as the services it rings inside it.
		if (
			containerType === 'Application' &&
			currentHoveredTag.tagId !== null &&
			(data as TopologyNode).node_type === 'Container' &&
			currentHoveredTag.tagId ===
				(data as Extract<TopologyNode, { node_type: 'Container' }>).entity_id &&
			currentHoveredTag.color
		) {
			const ch = createColorHelper(
				currentHoveredTag.color as Parameters<typeof createColorHelper>[0]
			);
			return `box-shadow: 0 0 0 3px ${ch.rgb};`;
		}
		// Only highlight when the hovered entity type matches this container's
		// entity type. containerType here is the container_type discriminant
		// (e.g. 'Subnet', 'Host'), which matches EntityDiscriminants casing.
		if (currentHoveredTag.entityType !== containerType) return '';
		const { tagId, color } = currentHoveredTag;
		// Entity-type-wide hover (tagId === null): subdued gray outline.
		if (tagId === null) {
			return 'box-shadow: 0 0 0 2px rgb(156, 163, 175);';
		}
		const isUntagged = containerTags.length === 0;
		const hasTag = tagId === UNTAGGED_SENTINEL ? isUntagged : containerTags.includes(tagId);
		if (!hasTag) return '';
		const ch = createColorHelper(color as Parameters<typeof createColorHelper>[0]);
		return `box-shadow: 0 0 0 3px ${ch.rgb};`;
	});

	// Whether the hovered entity type matches this container — drives the
	// dotted-underline on the header text.
	let isEntityTypeHover = $derived(
		currentHoveredTag?.tagId === null && currentHoveredTag?.entityType === containerType
	);

	// Only the commented-out resize controls used this.
	// const grayColorHelper = createColorHelper('Gray');

	function handleChevronClick(event: MouseEvent | KeyboardEvent) {
		if ($editModeEnabled) return;
		event.stopPropagation();
		toggleCollapse(id, topology?.nodes);
	}

	// Only the commented-out resize controls call this.
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	async function onResize(event: ResizeDragEvent, params: ResizeParams) {
		if (!topology) return;
		let node = topology.nodes.find((n) => n.id == id);
		if (node && params.width && params.height) {
			let roundedWidth = Math.round(params.width / 25) * 25;
			let roundedHeight = Math.round(params.height / 25) * 25;
			let roundedX = Math.round(params.x / 25) * 25;
			let roundedY = Math.round(params.y / 25) * 25;

			node.size.x = roundedWidth;
			node.size.y = roundedHeight;
			node.position.x = roundedX;
			node.position.y = roundedY;

			// DISABLED: no mechanism to persist container resize.
			// await updateNodeResizeMutation.mutateAsync({
			// 	topologyId: topology.id,
			// 	siteId: topology.site_id,
			// 	view: $activeView,
			// 	nodeId: node.id,
			// 	size: { x: roundedWidth, y: roundedHeight },
			// 	position: { x: roundedX, y: roundedY }
			// });
		}
	}
</script>

<!-- A container is collapsed by its title's chevron and selected by its title chip, the way a card
     is selected by clicking it. The body holds the children, so a click on it lands on the canvas. -->
<div
	class="relative"
	class:entity-type-hover-active={isEntityTypeHover}
	class:lod-veil={veiled}
	bind:this={rootElement}
	style="{nodeStyle} opacity: {nodeOpacity}; --lod-veil-edge: {colorHelper.rgb};"
>
	<!-- TITLE: External (card/pill above container) -->
	<!-- Rendered in both states: the pill is where a root container is named, collapsed or not. -->
	{#if !simplified && titleStyle === 'External' && (headerText || isCollapsible)}
		<ContainerHeader
			variant="external"
			{isCollapsed}
			{isCollapsible}
			{headerText}
			{iconComponent}
			logoDefinition={serviceDefId ?? null}
			{deviceType}
			{fillIcon}
			{colorHelper}
			{groupLabels}
			{childCount}
			{childSummary}
			tooltip={headerTooltip}
			{entityColor}
			{chipIcon}
			{staleTag}
			onToggleCollapse={handleChevronClick}
		/>
	{/if}

	<!-- TITLE: Inline (inside container top padding) -->
	{#if !simplified && titleStyle === 'Inline' && !isCollapsed && (headerText || groupLabels.length > 0)}
		<ContainerHeader
			variant="inline"
			{isCollapsed}
			{isCollapsible}
			{headerText}
			{iconComponent}
			logoDefinition={serviceDefId ?? null}
			{deviceType}
			{fillIcon}
			{colorHelper}
			{groupLabels}
			{childCount}
			{childSummary}
			tooltip={headerTooltip}
			{entityColor}
			{chipIcon}
			{staleTag}
			onToggleCollapse={handleChevronClick}
		/>
	{/if}

	<!-- COLLAPSED STATE -->
	{#if detailTier === 'hidden'}
		<!-- Nothing: a grouping box a few pixels across is noise inside its parent. The root div
		     stays so the node keeps its box and its handles — dropping those would strand every
		     edge that names them (`NodeHandles.svelte`). -->
	{:else if isCollapsed && simplified}
		<!--
			A collapsed container draws its own box through `ContainerHeader`, so unlike the expanded
			case there is no separate body div to fall back on — dropping the header would leave
			nothing at all. This is that box.

			Outlined rather than filled with the usual surface: `--color-topology-node-bg` is #ffffff
			against a #f8fafc canvas, which is 1.03:1 — at this size the box simply is not there. The
			outline carries the shape and the type colour still marks the top edge.
		-->
		<div
			class="lod-box rounded-lg"
			class:lod-search-match={hasSearchMatches}
			style="width: 100%; height: 100%; border-top: 2px solid {colorHelper.rgb};"
		></div>
	{:else if isCollapsed}
		{#if isSubcontainer}
			<ContainerHeader
				variant="collapsed-sub"
				{isCollapsed}
				{isCollapsible}
				{headerText}
				{iconComponent}
				logoDefinition={serviceDefId ?? null}
				{deviceType}
				{fillIcon}
				{colorHelper}
				{groupLabels}
				{childCount}
				{childSummary}
				tooltip={headerTooltip}
				{entityColor}
				{chipIcon}
				{staleTag}
				countOnly={isInfraRule}
				onToggleCollapse={handleChevronClick}
				{searchMatchCount}
				{searchHighlightRingStyle}
			/>
		{:else}
			<ContainerHeader
				variant="collapsed-root"
				{isCollapsed}
				{isCollapsible}
				{headerText}
				{iconComponent}
				logoDefinition={serviceDefId ?? null}
				{deviceType}
				{fillIcon}
				{colorHelper}
				{groupLabels}
				{childCount}
				{childSummary}
				tooltip={headerTooltip}
				{ungroupedSummary}
				onToggleCollapse={handleChevronClick}
				subgroupSummaries={resolvedSubgroups}
				tagHoverRingStyle={[tagHoverRingStyle, searchHighlightRingStyle].filter(Boolean).join(' ')}
				{searchMatchCount}
			/>
		{/if}
	{:else}
		<!-- EXPANDED STATE -->
		{#if isSubcontainer}
			{#if hasBorder}
				<div
					class="rounded-lg border border-dashed border-gray-300 transition-all duration-200 dark:border-gray-600"
					class:lod-box={simplified}
					style="background: var(--color-topology-subgroup-bg); width: 100%; height: 100%; position: relative; overflow: hidden;"
				></div>
			{/if}
		{:else}
			<div
				class="rounded-xl text-center text-sm font-semibold shadow-lg transition-all duration-200"
				class:lod-box={simplified}
				style="background: var(--color-topology-node-bg); width: 100%; height: 100%; position: relative; overflow: hidden; transition: box-shadow 0.15s ease-in-out; border-top: 2px solid {colorHelper.rgb}; {tagHoverRingStyle}"
			></div>

			<!-- Resize handles are gated on edit mode alone. They used to also test
			     `viewport.zoom > 0.5`, but reading the viewport here subscribed every
			     container node to every pan/zoom frame — a per-frame invalidation of
			     the whole graph to hide controls that are already hidden. -->
			<!--
			Container resize controls, commented out rather than deleted.

			Already unreachable — `editModeEnabled` is only ever set to false — and each one is DOM
			and listeners on every mounted container. Topology editing is disabled; restore these
			with `onResize` below if it is revived, and re-measure the tab's memory when you do.
			{#if $editModeEnabled}
			<NodeResizeControl
			position="bottom-right"
			onResizeEnd={onResize}
			style="z-index: 100; border: none; width: 20px; height: 20px;"
			>
			<svg
			xmlns="http://www.w3.org/2000/svg"
			width="20"
			height="20"
			viewBox="0 0 20 20"
			style="position: absolute; right: 10px; bottom: 10px;"
			>
			<path
			d="M20 7.5 L20 20 L7.5 20 Z"
			fill={selected ? colorHelper.rgb : grayColorHelper.rgb}
			style="transition: fill 200ms ease-in-out;"
			/>
			<line x1="11.667" y1="20" x2="20" y2="11.667" stroke="#374151" stroke-width="1" />
			<line x1="16.333" y1="20" x2="20" y2="16.333" stroke="#374151" stroke-width="1" />
			</svg>
			</NodeResizeControl>
			<NodeResizeControl
			position="top-left"
			onResizeEnd={onResize}
			style="z-index: 100; border: none; width: 20px; height: 20px;"
			>
			<svg
			xmlns="http://www.w3.org/2000/svg"
			width="20"
			height="20"
			viewBox="0 0 20 20"
			style="position: absolute; left: 10px; top: 10px;"
			>
			<path
			d="M0 12.5 L0 0 L12.5 0 Z"
			fill={selected ? colorHelper.rgb : grayColorHelper.rgb}
			style="transition: fill 200ms ease-in-out;"
			/>
			<line x1="8.333" y1="0" x2="0" y2="8.333" stroke="#374151" stroke-width="1" />
			<line x1="3.667" y1="0" x2="0" y2="3.667" stroke="#374151" stroke-width="1" />
			</svg>
			</NodeResizeControl>
			<NodeResizeControl
			position="top-right"
			onResizeEnd={onResize}
			style="z-index: 100; border: none; width: 20px; height: 20px;"
			>
			<svg
			xmlns="http://www.w3.org/2000/svg"
			width="20"
			height="20"
			viewBox="0 0 20 20"
			style="position: absolute; right: 10px; top: 10px;"
			>
			<path
			d="M7.5 0 L20 0 L20 12.5 Z"
			fill={selected ? colorHelper.rgb : grayColorHelper.rgb}
			style="transition: fill 200ms ease-in-out;"
			/>
			<line x1="11.667" y1="0" x2="20" y2="8.333" stroke="#374151" stroke-width="1" />
			<line x1="16.333" y1="0" x2="20" y2="3.667" stroke="#374151" stroke-width="1" />
			</svg>
			</NodeResizeControl>
			<NodeResizeControl
			position="bottom-left"
			onResizeEnd={onResize}
			style="z-index: 100; border: none; width: 20px; height: 20px;"
			>
			<svg
			xmlns="http://www.w3.org/2000/svg"
			width="20"
			height="20"
			viewBox="0 0 20 20"
			style="position: absolute; left: 10px; bottom: 10px;"
			>
			<path
			d="M0 7.5 L12.5 20 L0 20 Z"
			fill={selected ? colorHelper.rgb : grayColorHelper.rgb}
			style="transition: fill 200ms ease-in-out;"
			/>
			<line x1="0" y1="11.667" x2="8.333" y2="20" stroke="#374151" stroke-width="1" />
			<line x1="0" y1="16.333" x2="3.667" y2="20" stroke="#374151" stroke-width="1" />
			</svg>
			</NodeResizeControl>
			{/if}
			-->
		{/if}
	{/if}
</div>

<!-- Only the handles an edge on this node actually names; see `edgeHandlesByNode`. -->
{#if edgeHandles.get(id)}
	<NodeHandles size={CONTAINER_HANDLE_SIZE_PX} used={edgeHandles.get(id)!} />
{/if}

<style>
	/*
	 * A container's box at reduced detail.
	 *
	 * The full-detail surfaces are chosen to sit under text, so they are a hair off the canvas:
	 * #ffffff on #f8fafc is 1.03:1, and the dark pair is no better. That is fine behind a card and
	 * useless when the box *is* the node. This overrides the fill with a faint tint and, more
	 * importantly, an outline — a shape with an edge reads at three pixels where a fill does not.
	 * Both values are theme tokens; see `app.css`.
	 */
	.lod-box {
		background: var(--color-topology-lod-container) !important;
		outline: 1px solid var(--color-topology-lod-stroke);
		outline-offset: -1px;
		box-shadow: none !important;
	}

	/* A search match inside a boxed container: the ring the full-detail card draws with a shadow,
	 * drawn as the outline the box already has. */
	.lod-box.lod-search-match {
		outline: 2px solid rgb(59 130 246);
	}

	div {
		word-wrap: break-word;
		overflow-wrap: break-word;
	}

	:global(.svelte-flow__resize-control) {
		background-color: transparent !important;
	}
</style>
