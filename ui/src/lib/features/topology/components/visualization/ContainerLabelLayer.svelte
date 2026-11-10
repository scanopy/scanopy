<script lang="ts">
	import { untrack } from 'svelte';
	import throttle from 'just-throttle';
	import { ViewportPortal, useViewport, type Node } from '@xyflow/svelte';
	import { containerTypes } from '$lib/shared/stores/metadata';
	import { createColorHelper } from '$lib/shared/utils/styling';
	import { searchMatchContainerMap, searchMatchNodeIds } from '../../interactions';
	import { placeLabels, type LabelCandidate, type PlacedLabel } from '../../pipeline/label-placement';

	/**
	 * Container names over a zoomed-out canvas: see `label-placement.ts` for which are drawn and
	 * why they live on a layer of their own.
	 *
	 * The layer sits inside the viewport, so a pan moves it with the nodes at no cost. Each name is
	 * scaled by the inverse of the zoom to stay one size on screen; that is one CSS variable on the
	 * layer, written as the zoom changes. Which names are drawn is recomputed only when the
	 * viewport comes to rest or the nodes change, never per frame.
	 */
	let {
		nodes,
		active
	}: {
		nodes: Node[];
		/** Container headers cannot be read at this zoom, and nothing is being exported. */
		active: boolean;
	} = $props();

	// The canvas this layer belongs to, found from a marker left where the layer is declared: the
	// labels themselves are moved into the viewport by the portal.
	let marker = $state<HTMLElement>();
	let pane = $derived(marker?.closest<HTMLElement>('.svelte-flow') ?? undefined);

	const viewport = useViewport();
	let inverseZoom = $derived(1 / viewport.current.zoom);

	// Bumped a moment after the viewport last moved, by the user or by a fit, and at most a few
	// times a second while it is moving.
	let settled = $state(0);
	const noteSettled = throttle(() => settled++, 150, { leading: false, trailing: true });
	$effect(() => {
		void viewport.current.x;
		void viewport.current.y;
		void viewport.current.zoom;
		noteSettled();
		return () => noteSettled.cancel();
	});

	const SEARCH_PRIORITY = 1e12;
	/** Width of a label's coloured left border, in pixels. Matches the style below. */
	const LABEL_BORDER_PX = 2;

	function colorOf(node: Node): string {
		const data = node.data as Record<string, unknown>;
		return typeof data.color === 'string' && data.color
			? createColorHelper(data.color as Parameters<typeof createColorHelper>[0]).rgb
			: containerTypes.getColorHelper((data.container_type as string) ?? null).rgb;
	}

	let placed = $derived.by((): PlacedLabel[] => {
		// Read so a settled viewport recomputes; the viewport itself is read untracked below, or
		// every pan frame would.
		void settled;
		if (!active || !pane) return [];
		const matches = new Set([...$searchMatchNodeIds, ...$searchMatchContainerMap.keys()]);
		const candidates: LabelCandidate[] = [];
		for (const node of nodes) {
			if (node.type !== 'Container' || node.parentId) continue;
			const width = node.width ?? node.measured?.width ?? 0;
			const height = node.height ?? node.measured?.height ?? 0;
			const text = ((node.data as Record<string, unknown>).header as string | undefined) ?? '';
			if (!text || width <= 0 || height <= 0) continue;
			candidates.push({
				id: node.id,
				text,
				color: colorOf(node),
				x: node.position.x,
				y: node.position.y,
				width,
				height,
				priority: (matches.has(node.id) ? SEARCH_PRIORITY : 0) + width * height
			});
		}
		const { x, y, zoom } = untrack(() => viewport.current);
		return placeLabels(candidates, {
			x: -x / zoom,
			y: -y / zoom,
			width: pane.clientWidth,
			height: pane.clientHeight,
			zoom
		});
	});

	let boxById = $derived(new Map(nodes.map((node) => [node.id, node])));

	// The whole name of whatever box the pointer is over, at any zoom, on that box's corner. It is
	// how a name cut short to its box is read in full (the labels take no pointer events, so the
	// pointer over a label is over its box), and the only name there is where boxes are too small
	// to carry one.
	//
	// A box that carries a label keeps that label: it only grows to the name's full width, and
	// nothing else about it changes. A box with none (too small, or a card inside a container)
	// gets one for as long as the pointer is on it.
	interface Hovered {
		id: string;
		text: string;
		color: string;
		x: number;
		y: number;
		/** Width of the box's own label showing its whole name, when it has one. */
		fullWidth: number | null;
	}
	let hovered = $state<Hovered | null>(null);
	let layer = $state<HTMLElement>();

	function describe(id: string): Hovered | null {
		const names: string[] = [];
		let x = 0;
		let y = 0;
		let root: Node | undefined;
		for (let node = boxById.get(id); node; node = boxById.get(node.parentId ?? '')) {
			const header = (node.data as Record<string, unknown>).header as string | undefined;
			if (header) names.unshift(header);
			// Positions are relative to the parent, so the chain sums to the absolute one.
			x += node.position.x;
			y += node.position.y;
			root = node;
		}
		if (names.length === 0 || !root) return null;
		const own = layer?.querySelector<HTMLElement>(`[data-label-for="${CSS.escape(id)}"]`);
		return {
			id,
			text: names.join(' › '),
			color: colorOf(root),
			x,
			y,
			// `scrollWidth` is the text and padding, rounded down; the left border is outside it. The
			// spare pixels keep a name that just fits from being cut again by that rounding.
			fullWidth: own ? own.scrollWidth + LABEL_BORDER_PX + 3 : null
		};
	}

	$effect(() => {
		if (!active || !pane) {
			hovered = null;
			return;
		}
		let hoveredId: string | null = null;
		// A label takes no pointer events, so the pane and the nodes under it still pan, select and
		// click. Whether the pointer is on one is read from its rectangle.
		const labelUnder = (event: PointerEvent): string | null => {
			for (const label of layer?.querySelectorAll<HTMLElement>('[data-label-for]') ?? []) {
				const rect = label.getBoundingClientRect();
				if (
					event.clientX >= rect.left &&
					event.clientX <= rect.right &&
					event.clientY >= rect.top &&
					event.clientY <= rect.bottom
				) {
					return label.dataset.labelFor ?? null;
				}
			}
			return null;
		};
		const onMove = (event: PointerEvent) => {
			const element = (event.target as Element | null)?.closest?.('.svelte-flow__node');
			// On a label, the label's own container; otherwise whatever node is under the pointer.
			const id = labelUnder(event) ?? (element as HTMLElement | null)?.dataset.id ?? null;
			if (id === hoveredId) return;
			hoveredId = id;
			hovered = id ? describe(id) : null;
		};
		const onLeave = () => {
			hoveredId = null;
			hovered = null;
		};
		pane.addEventListener('pointermove', onMove);
		pane.addEventListener('pointerleave', onLeave);
		return () => {
			pane.removeEventListener('pointermove', onMove);
			pane.removeEventListener('pointerleave', onLeave);
		};
	});
</script>

<span bind:this={marker} hidden></span>
{#if active}
	<ViewportPortal target="front">
		<div class="container-labels" style:--inv-zoom={inverseZoom} bind:this={layer}>
			{#each placed as label (label.id)}
				{@const open = hovered?.id === label.id && hovered.fullWidth !== null}
				<div
					class="container-label"
					class:open
					data-label-for={label.id}
					style:--label-color={label.color}
					style:max-width="{open ? Math.max(hovered?.fullWidth ?? 0, label.maxWidth) : label.maxWidth}px"
					style:transform="translate({label.x}px, {label.y}px) scale(var(--inv-zoom))"
				>
					{label.text}
				</div>
			{/each}
			{#if hovered && hovered.fullWidth === null}
				<div
					class="container-label open"
					style:--label-color={hovered.color}
					style:transform="translate({hovered.x}px, {hovered.y}px) scale(var(--inv-zoom))"
				>
					{hovered.text}
				</div>
			{/if}
		</div>
	</ViewportPortal>
{/if}

<style>
	/* Above every node. Nodes carry their own z-index (a child above its parent, a selected node
	 * above both) and share a stacking context with this layer. */
	.container-labels {
		position: absolute;
		top: 0;
		left: 0;
		z-index: 100000;
		pointer-events: none;
	}

	/* 16px tall, as `label-placement.ts` plans for. Never wider than its box (`max-width`, set per
	 * label): a longer name is cut with an ellipsis. On hover the width alone grows to the whole
	 * name, and eases there; colour, weight and size stay as they are. */
	.container-label {
		position: absolute;
		top: 0;
		left: 0;
		box-sizing: border-box;
		overflow: hidden;
		text-overflow: ellipsis;
		transition: max-width 0.15s ease-out;
		transform-origin: 0 0;
		height: 16px;
		padding: 0 6px;
		border-radius: 4px;
		border-left: 2px solid var(--label-color, var(--color-topology-lod-stroke));
		font-size: 11px;
		line-height: 16px;
		white-space: nowrap;
		color: var(--color-text-primary, #0f172a);
		background: var(--color-topology-node-bg, #fff);
		box-shadow: 0 0 0 1px var(--color-border, #e2e8f0);
	}

	/* Above its neighbours while it is wider than its box. */
	.container-label.open {
		z-index: 1;
	}
</style>
