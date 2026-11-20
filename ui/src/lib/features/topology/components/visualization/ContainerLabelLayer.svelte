<script lang="ts">
	import { untrack } from 'svelte';
	import throttle from 'just-throttle';
	import { ViewportPortal, useViewport, type Node } from '@xyflow/svelte';
	import { containerTypes } from '#lib/shared/stores/metadata.ts';
	import { createColorHelper } from '#lib/shared/utils/styling.ts';
	import {
		searchHiddenNodeIds,
		searchMatchContainerMap,
		searchMatchNodeIds
	} from '../../interactions';
	import {
		nameLevels,
		nameMatches,
		onPane,
		zoomStep,
		type LabelCandidate,
		type PlacedLabel
	} from '../../pipeline/label-placement';

	/**
	 * Container names over a zoomed-out canvas: see `label-placement.ts` for which are drawn and
	 * why they live on a layer of their own.
	 *
	 * The layer sits inside the viewport, so a pan moves it with the nodes at no cost. Each name is
	 * scaled by the inverse of the zoom to stay one size on screen; that is one CSS variable on the
	 * layer, written as the zoom changes. Which boxes are named is placed once per zoom step and
	 * kept until the nodes change; when the viewport comes to rest, the kept names are filtered to
	 * the pane. Nothing is computed per frame, and a pan places nothing.
	 */
	let {
		nodes,
		active,
		namesShown,
		keptIds
	}: {
		nodes: Node[];
		/** Container headers cannot be read at this zoom, and nothing is being exported. */
		active: boolean;
		/** The user shows container names (`show_container_names`). Off, only search matches are
		 * named, and hovering a box still names it. */
		namesShown: boolean;
		/** Nodes drawn in full at any zoom (`detailKeptIds`): they show their own header. */
		keptIds: ReadonlySet<string>;
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

	/** Width of a label's coloured left border, in pixels. Matches the style below. */
	const LABEL_BORDER_PX = 2;

	// A name's drawn width, measured once per name at the label's font. Text, padding (6px each
	// side), the left border, and a pixel so a name that just fits is not cut by rounding.
	const LABEL_PADDING_PX = 12;
	let measureContext: CanvasRenderingContext2D | null = null;
	// A cache, read and written outside any reactive scope.
	// eslint-disable-next-line svelte/prefer-svelte-reactivity
	const textWidths = new Map<string, number>();
	function textWidthOf(text: string, font: string): number {
		const key = `${font}\u0000${text}`;
		const cached = textWidths.get(key);
		if (cached !== undefined) return cached;
		measureContext ??= document.createElement('canvas').getContext('2d');
		// No 2D canvas: about 7px a character at 11px.
		if (!measureContext) return text.length * 7 + LABEL_PADDING_PX + LABEL_BORDER_PX;
		measureContext.font = font;
		const width =
			Math.ceil(measureContext.measureText(text).width) + LABEL_PADDING_PX + LABEL_BORDER_PX + 1;
		textWidths.set(key, width);
		return width;
	}

	function colorOf(node: Node): string {
		const data = node.data as Record<string, unknown>;
		return typeof data.color === 'string' && data.color
			? createColorHelper(data.color as Parameters<typeof createColorHelper>[0]).rgb
			: containerTypes.getColorHelper((data.container_type as string) ?? null).rgb;
	}

	let parentOf = $derived(new Map(nodes.map((node) => [node.id, node.parentId])));

	function rootOf(id: string): string {
		let root = id;
		for (let parent = parentOf.get(root); parent; parent = parentOf.get(root)) root = parent;
		return root;
	}

	// Every root container with a name, and the boxes with none that a name must stay off.
	let candidates = $derived.by((): LabelCandidate[] => {
		if (!active || !pane) return [];
		const font = `11px ${getComputedStyle(pane).fontFamily}`;
		const built: LabelCandidate[] = [];
		for (const node of nodes) {
			if (node.type !== 'Container' || node.parentId) continue;
			const width = node.width ?? node.measured?.width ?? 0;
			const height = node.height ?? node.measured?.height ?? 0;
			if (width <= 0 || height <= 0) continue;
			const data = node.data as Record<string, unknown>;
			const text = (data.header as string | undefined) ?? '';
			built.push({
				id: node.id,
				text,
				textWidth: text ? textWidthOf(text, font) : 0,
				color: colorOf(node),
				x: node.position.x,
				y: node.position.y,
				width,
				height,
				level: (data.container_type as string | undefined) ?? '',
				priority: width * height
			});
		}
		return built;
	});
	let candidateById = $derived(new Map(candidates.map((candidate) => [candidate.id, candidate])));

	// The names placed at each zoom step, for the current nodes: a pan and a return to a step
	// already visited place nothing. A cache, written while deriving `placed`.
	let namedByStep = $derived.by(() => {
		void candidates;
		// eslint-disable-next-line svelte/prefer-svelte-reactivity
		return new Map<number, PlacedLabel[]>();
	});

	let placed = $derived.by((): PlacedLabel[] => {
		// Read so a settled viewport recomputes; the viewport itself is read untracked below, or
		// every pan frame would.
		void settled;
		if (!active || !pane || candidates.length === 0) return [];
		const { x, y, zoom } = untrack(() => viewport.current);
		const paneView = {
			x: -x / zoom,
			y: -y / zoom,
			width: pane.clientWidth,
			height: pane.clientHeight,
			zoom
		};
		// The root containers a search points at: one that matches, and one holding a match at any
		// depth. A match inside a collapsed container is not a node here, and the search reports
		// the collapsed container in its place.
		// eslint-disable-next-line svelte/prefer-svelte-reactivity
		const matches = new Set<string>();
		for (const id of [...$searchMatchNodeIds, ...$searchMatchContainerMap.keys()]) {
			if (parentOf.has(id)) matches.add(rootOf(id));
		}
		// A query with no match hides every node, and names none.
		const searching = matches.size > 0 || $searchHiddenNodeIds.size > 0;
		let labels: PlacedLabel[];
		if (searching) {
			// While searching, the names left on the canvas are where the matches are, each named
			// however small its box.
			labels = nameMatches(
				candidates.filter((candidate) => matches.has(candidate.id)),
				paneView
			);
		} else if (!namesShown) {
			labels = [];
		} else {
			const step = zoomStep(zoom);
			let named = namedByStep.get(step);
			if (!named) {
				named = nameLevels(candidates, step);
				namedByStep.set(step, named);
			}
			labels = onPane(named, candidateById, paneView);
		}
		return labels.filter((label) => !keptIds.has(label.id));
	});
	let placedIds = $derived(new Set(placed.map((label) => label.id)));

	// The label the pointer is on, widened to its whole name. Only the width changes.
	//
	// A boxed card or group shows no tooltip: the way to learn about one is to zoom in to it, where
	// the cards and their own tooltips are. A name pieced together for a box a few pixels across was
	// cryptic, and attaching the real tooltip to thousands of boxes was not worth the risk to
	// panning.
	let hovered = $state<{ id: string; fullWidth: number } | null>(null);
	// The root container under the pointer when its level is not named at this zoom: it shows its
	// whole name until the pointer leaves it.
	let hoveredBox = $state<string | null>(null);
	let drawn = $derived.by((): PlacedLabel[] => {
		const candidate = hoveredBox ? candidateById.get(hoveredBox) : undefined;
		if (!candidate?.text || placedIds.has(candidate.id) || keptIds.has(candidate.id)) {
			return placed;
		}
		const { id, text, color, x, y, textWidth } = candidate;
		return [...placed, { id, text, color, x, y, maxWidth: textWidth }];
	});
	let layer = $state<HTMLElement>();

	$effect(() => {
		hoveredBox = null;
		if (!active || !pane) {
			hovered = null;
			return;
		}
		// A label takes no pointer events, so the pane and the nodes under it still pan, select and
		// click. Whether the pointer is on one is read from its rectangle.
		const labelUnder = (event: PointerEvent): HTMLElement | null => {
			for (const label of layer?.querySelectorAll<HTMLElement>('[data-label-for]') ?? []) {
				const rect = label.getBoundingClientRect();
				if (
					event.clientX >= rect.left &&
					event.clientX <= rect.right &&
					event.clientY >= rect.top &&
					event.clientY <= rect.bottom
				) {
					return label;
				}
			}
			return null;
		};
		const onMove = (event: PointerEvent) => {
			// A drag pans the canvas; the names wait for the pointer to rest.
			if (event.buttons) return;
			const label = labelUnder(event);
			const id = label?.dataset.labelFor ?? null;
			// On the name drawn for the box under the pointer.
			if (id && id === hoveredBox) return;
			if (label && id) {
				hoveredBox = null;
				if (id === hovered?.id) return;
				// `scrollWidth` is the text and padding, rounded down; the left border is outside it.
				// The spare pixels keep a name that just fits from being cut again by that rounding.
				hovered = { id, fullWidth: label.scrollWidth + LABEL_BORDER_PX + 3 };
				return;
			}
			hovered = null;
			// A label takes no pointer events, so this is the node itself.
			const node = document
				.elementFromPoint(event.clientX, event.clientY)
				?.closest<HTMLElement>('.svelte-flow__node');
			const root = node?.dataset.id ? rootOf(node.dataset.id) : null;
			hoveredBox = root && candidateById.get(root)?.text ? root : null;
		};
		const onLeave = () => {
			hovered = null;
			hoveredBox = null;
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
			{#each drawn as label (label.id)}
				{@const open = hovered?.id === label.id || hoveredBox === label.id}
				<div
					class="container-label"
					class:open
					data-label-for={label.id}
					style:--label-color={label.color}
					style:max-width="{open
						? Math.max(hovered?.fullWidth ?? 0, label.maxWidth)
						: label.maxWidth}px"
					style:transform="translate({label.x}px, {label.y}px) scale(var(--inv-zoom))"
				>
					{label.text}
				</div>
			{/each}
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

	/* 16px tall, as `label-placement.ts` plans for. Never wider than `max-width`, set per label to
	 * the whole name or to where the next box or name begins: a longer name is cut with an
	 * ellipsis. On hover the width alone grows to the whole name, and eases there; colour, weight
	 * and size stay as they are. */
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
