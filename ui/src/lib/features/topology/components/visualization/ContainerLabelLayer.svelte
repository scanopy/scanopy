<script lang="ts">
	import { untrack } from 'svelte';
	import throttle from 'just-throttle';
	import { ViewportPortal, useViewport, type Node } from '@xyflow/svelte';
	import { containerTypes } from '$lib/shared/stores/metadata';
	import { createColorHelper } from '$lib/shared/utils/styling';
	import { searchMatchContainerMap, searchMatchNodeIds } from '../../interactions';
	import {
		labelText,
		placeLabels,
		type LabelCandidate,
		type PlacedLabel
	} from '../../pipeline/label-placement';

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
		active,
		landmarks
	}: {
		nodes: Node[];
		/** Container headers cannot be read at this zoom, and nothing is being exported. */
		active: boolean;
		landmarks: boolean;
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
		return placeLabels(
			candidates,
			{ x: -x / zoom, y: -y / zoom, width: pane.clientWidth, height: pane.clientHeight, zoom },
			{ landmarks }
		);
	});

	let boxById = $derived(new Map(nodes.map((node) => [node.id, node])));

	// The name of whatever box the pointer is over, at any zoom: the cheapest answer to "what is
	// this one", and the only one where boxes are too dense for any name to be drawn.
	let hovered = $state<{ text: string; x: number; y: number } | null>(null);
	let hoveredId: string | null = null;

	function describe(id: string): { text: string; x: number; y: number } | null {
		const names: string[] = [];
		let x = 0;
		let y = 0;
		for (let node = boxById.get(id); node; node = boxById.get(node.parentId ?? '')) {
			const header = (node.data as Record<string, unknown>).header as string | undefined;
			if (header) names.unshift(labelText(header));
			// Positions are relative to the parent, so the chain sums to the absolute one.
			x += node.position.x;
			y += node.position.y;
		}
		return names.length > 0 ? { text: names.join(' › '), x, y } : null;
	}

	$effect(() => {
		if (!active || !pane) {
			hovered = null;
			hoveredId = null;
			return;
		}
		const onMove = (event: PointerEvent) => {
			const element = (event.target as Element | null)?.closest?.('.svelte-flow__node');
			const id = (element as HTMLElement | null)?.dataset.id ?? null;
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
		<div class="container-labels" style:--inv-zoom={inverseZoom}>
			{#each placed as label (label.id)}
				<div
					class="container-label"
					class:landmark={label.placement === 'landmark'}
					style:--label-color={label.color}
					style:transform="translate({label.x}px, {label.y}px) scale(var(--inv-zoom)){label.placement ===
					'landmark'
						? ' translateY(-100%)'
						: ''}"
				>
					{label.text}
				</div>
			{/each}
			{#if hovered}
				<div
					class="container-label hover"
					style:transform="translate({hovered.x}px, {hovered.y}px) scale(var(--inv-zoom)) translateY(-100%)"
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

	/* 16px tall and about 6.2px a character: `label-placement.ts` plans with the same figures. */
	.container-label {
		position: absolute;
		top: 0;
		left: 0;
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

	.container-label.hover {
		z-index: 1;
		font-weight: 600;
		border-left-color: var(--color-text-primary, #0f172a);
	}
</style>
