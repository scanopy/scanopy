/**
 * Blank-canvas diagnostics for the topology viewer.
 *
 * A customer on a ~300-host network reports the L2 canvas going blank at the upper collapse
 * levels — minimap still drawing, `F` doing nothing, intermittent and browser-dependent. It did
 * not reproduce on the 444-host seed here, so the next move is evidence from their browser rather
 * than more guessing.
 *
 * # Why this is not in `perf.ts`
 *
 * `perf.ts` is gated on `import.meta.env.DEV || window.__topoPerf`, so it records nothing in a
 * customer's build — and asking someone to set a global and reproduce an intermittent fault is
 * asking them to do the hard part twice. This is always on. It costs one
 * `querySelectorAll().length` per pipeline run plus a throttled handler, which is what makes that
 * affordable.
 *
 * # What it records
 *
 * A ring buffer, not a single snapshot. The interesting part of an intermittent blank is the
 * *transition into* it — what the culling gate, the viewport and the measured-size coverage were
 * doing on the runs before. One sample of a broken state rarely identifies a cause.
 *
 * Blankness has three distinguishable causes and the record has to separate them, since they lead
 * to completely different fixes:
 *
 * - **Culled** — the store holds nodes, none are mounted. The viewport is somewhere the graph is
 *   not, or the gate is wrong.
 * - **Empty** — the store holds nothing. A pipeline or data problem, nothing to do with rendering.
 * - **Hidden** — the container is `visibility: hidden` for the measure pass. Transient by design;
 *   only a fault if it sticks.
 */

import throttle from 'just-throttle';
import { get } from 'svelte/store';
import {
	CULLING_THRESHOLD_ELEMENTS,
	cullingDisabledForTooling as cullingSuppressedForTooling
} from './pipeline/render-mode';
import { collapseLevel, collapsedContainers } from './collapse';
import { activeView } from './queries';
import { usedJSHeapMb, totalJSHeapMb } from './perf';
import type { components } from '$lib/api/schema';

/** How many samples to keep. Enough to cover the runs leading into a blank, small enough to mail. */
const HISTORY_LIMIT = 30;

/**
 * Size at or below which a mounted node counts as degenerate, in either dimension.
 *
 * A node rendered with `width: 0` or `height: 0` still occupies its 1px borders, so the floor is a
 * couple of pixels rather than zero. Nothing legitimate comes close: element cards are a fixed 250
 * wide, and a container is at least its type's declared collapsed size.
 */
const DEGENERATE_SIZE_PX = 4;

/** Minimum gap between viewport-driven samples. Panning fires continuously. */
const VIEWPORT_SAMPLE_INTERVAL_MS = 500;

export type BlankReason = 'culled' | 'empty' | 'hidden' | null;

/** The frame's own facts, reduced to the five that decide whether anything is on screen. */
export interface BlankInputs {
	/** Nodes the store holds. Zero is a data problem, not a rendering one. */
	storeNodes: number;
	/** The pane is `visibility: hidden`. */
	paneHidden: boolean;
	/** The cold load's measure pass is running, which hides the pane on purpose. */
	coldLoadMeasure: boolean;
	/** `.svelte-flow__node` elements in this viewer. */
	mountedCount: number;
	/** Whether the mounted nodes' bounding box meets the pane at all. */
	intersectsPane: boolean;
}

/**
 * Why nothing is on the canvas, or `null` if something is.
 *
 * Split out of `sampleViewerState` so the decision can be tested without a DOM — the classifier
 * drove a customer report and had no coverage at all, which is how a sample taken before the
 * renderer had caught up came to be recorded as a blank canvas.
 *
 * Order matters. An empty store outranks everything: with no nodes there is nothing for culling or
 * a hidden pane to explain, and reporting `culled` there would send the next reader to the
 * renderer for what is a pipeline fault.
 */
export function classifyBlank({
	storeNodes,
	paneHidden,
	coldLoadMeasure,
	mountedCount,
	intersectsPane
}: BlankInputs): BlankReason {
	if (storeNodes === 0) return 'empty';
	// Hidden *while the cold load measures* is that pass doing its job — it mounts every node at
	// full size behind `visibility: hidden` to read their heights, and a sample landing in that
	// window sees an intentionally invisible canvas. Only a pane still hidden with no cold load
	// running is a fault, and it is one an operator would describe the same way as the others.
	//
	// Gated on `coldLoadMeasure`, not `measuring`: the cold load is the only pass that hides the
	// pane, so a later measure pass must not excuse a pane that is stuck hidden.
	if (paneHidden) return coldLoadMeasure ? null : 'hidden';
	if (mountedCount === 0 || !intersectsPane) return 'culled';
	return null;
}

/**
 * How many of the store's nodes SvelteFlow is *able* to cull.
 *
 * The quantity missing from the first version of this report, and the one that would have named
 * the fault immediately. Culling is opt-out until a node can be tested: `getNodesInside` skips the
 * viewport check entirely while `internals.handleBounds` is undefined (`forceInitialRender`), and
 * computes `area` from `measured`, treating an unknown height as zero — which passes
 * `overlappingArea >= area` wherever the viewport is. A node missing either is mounted no matter
 * where it sits, so a report can show `culling: on` beside `mounted == store.nodes` and nothing in
 * it explains why. `forceRendered` is that number directly.
 */
export interface CullabilityCounts {
	/** Internal nodes inspected. Less than `store.nodes` when the viewport path stride-samples. */
	total: number;
	withMeasured: number;
	withHandleBounds: number;
	/** Nodes SvelteFlow will mount regardless of the viewport. */
	forceRendered: number;
}

export interface CullabilitySummary extends CullabilityCounts {
	/**
	 * Whether `total` is a stride sample rather than the whole store.
	 *
	 * Without it the two numbers are indistinguishable and read as a fault. A capture showed
	 * `cullable.total: 423` on the viewport rows against a 1268-node store and `1268` on a pipeline
	 * row, which looks like the count moving — it is `1268 / ⌈1268/500⌉`, the stride below, taken
	 * on the throttled path and nowhere else.
	 */
	sampled: boolean;
}

/** The shape `summariseCullability` needs. Structural, so a test needs no SvelteFlow. */
export interface CullableNode {
	measured?: { width?: number; height?: number };
	internals?: { handleBounds?: unknown };
}

/**
 * Reduce internal nodes to the counts above. Pure, so it can be unit-tested without a DOM.
 *
 * A node counts as measured only with both dimensions: a width alone still yields `area === 0`,
 * which is the case that made every element card unconditionally visible.
 */
export function summariseCullability(nodes: (CullableNode | undefined)[]): CullabilityCounts {
	let total = 0;
	let withMeasured = 0;
	let withHandleBounds = 0;
	let forceRendered = 0;

	for (const node of nodes) {
		if (!node) continue;
		total += 1;
		const measured = Boolean(node.measured?.width && node.measured?.height);
		const handleBounds = Boolean(node.internals?.handleBounds);
		if (measured) withMeasured += 1;
		if (handleBounds) withHandleBounds += 1;
		if (!handleBounds || !measured) forceRendered += 1;
	}

	return { total, withMeasured, withHandleBounds, forceRendered };
}

/** Mirrors `RunHeapLedger` in `perf.ts`, which this file reads through `window` rather than importing. */
export interface PerfRunLedger {
	growthMb: number;
	stages: { name: string; heapBeforeMb?: number; heapAfterMb?: number; ms: number }[];
}

/**
 * Counters that accumulate over the session, rather than describing one moment.
 *
 * The customer's console held 247 `out of memory` throws, and the ring buffer could not say
 * whether that was one graph too large to mount or the same graph mounted over and over. These
 * separate the two: a handful of node-store writes with a high `peakMounted` is a single
 * allocation, while hundreds of writes is a remount loop. Deliberately not in `perf.ts`, which
 * records nothing unless the build is a dev build.
 *
 * `usedJSHeapSize` would answer it directly but is Chrome-only, and the report that prompted this
 * came from Firefox — so these are counters rather than a memory reading, and the heap figure is
 * included only when the browser happens to offer it.
 */
export interface CumulativeCounters {
	pipelineRuns: number;
	nodeStoreWrites: number;
	fullMeasurePasses: number;
	peakStoreNodes: number;
	peakMounted: number;
	/** Highest element count inside mounted nodes — the peak the renderer had to hold. */
	peakDomInNodes: number;
	/** Heap V8 has reserved, which exceeds what is live and is what the OS sees. Chrome-only. */
	totalJSHeapMb?: number;
	/** Chrome-only; absent in Firefox and Safari. */
	usedJSHeapMb?: number;
	/**
	 * Highest live heap seen at any moment this session, not just when the report was taken.
	 *
	 * The figures above describe the instant someone ran the command, and that instant is reliably
	 * the wrong one: the renderer was measured swinging 881 MB to 2,050 MB and back inside 90
	 * seconds, while captures taken right afterwards read ~267 MB live against a 1.2 GB process.
	 * The spike is allocated and released within a run, so it is only ever visible to something
	 * watching continuously. That transient peak — not the resting footprint — is what a tab runs
	 * out of memory on, so it is the number to read first.
	 */
	peakUsedJSHeapMb?: number;
	/** Peak reserved heap, tracked alongside `peakUsedJSHeapMb` and for the same reason. */
	peakTotalJSHeapMb?: number;
	/**
	 * Times a later input made the run in flight obsolete — signals raised, not runs saved.
	 *
	 * This counts the generation bump, which is all the supersede mechanism does. Whether it saved
	 * anything depends on where the run had got to: the only bail is the check immediately before
	 * `layoutEngine.compute`, so a signal that arrives one second into a twenty-second layout is
	 * counted here and still pays for every second of it.
	 *
	 * So this is not a numerator over `elkRuns`, and the two must not be read as a ratio — a
	 * customer capture of `runsSuperseded: 28` beside `elkRuns: 31` was read as "28 of 31 layouts
	 * were abandoned" when the counters describe disjoint events and no run in the buffer carried
	 * `supersededBeforeElk` at all. The pair that answers "did cancellation help" is
	 * `runsSuperseded` against `elkResultsDiscarded`: the difference is signals that landed in time.
	 */
	runsSuperseded: number;
	/**
	 * Layouts that ran to completion and were thrown away for being stale.
	 *
	 * The cost cancellation did *not* avoid, which nothing recorded before. Every one of these is a
	 * full ELK layout — 18-22s and several hundred MB on a large estate — paid for and discarded.
	 */
	elkResultsDiscarded: number;
	/**
	 * `elk.layout()` calls. The allocation driver: ~250-340MB and 1.8-3.7s apiece.
	 *
	 * Layouts, not pipeline runs. Each run performs two — a first pass to learn element positions
	 * and a second with the container ports pinned to them — and this used to be incremented once
	 * per run beside a doc that said "ELK layouts actually performed", so every reading of it was
	 * half the truth.
	 */
	elkRuns: number;
}

/**
 * Fold the current heap reading into the session peaks.
 *
 * Called at every point the pipeline passes through rather than on a timer, because the expensive
 * stage is synchronous: `elk.bundled.js` is the main-thread build, so during a layout no interval
 * or animation frame can fire and a sampler would step straight over the peak it exists to catch.
 * Unconditional, unlike `perf.ts` — a customer's build records nothing there, and the customer is
 * the one whose tab is dying.
 */
export function noteHeapSample(): void {
	const used = usedJSHeapMb();
	if (used !== undefined) {
		counters.peakUsedJSHeapMb = Math.max(counters.peakUsedJSHeapMb ?? 0, used);
	}
	const total = totalJSHeapMb();
	if (total !== undefined) {
		counters.peakTotalJSHeapMb = Math.max(counters.peakTotalJSHeapMb ?? 0, total);
	}
}

const counters: CumulativeCounters = {
	pipelineRuns: 0,
	nodeStoreWrites: 0,
	fullMeasurePasses: 0,
	peakStoreNodes: 0,
	peakMounted: 0,
	peakDomInNodes: 0,
	runsSuperseded: 0,
	elkResultsDiscarded: 0,
	elkRuns: 0
};

/** A later input made the run in flight obsolete. Whether that saved a layout is a separate fact. */
export function noteRunSuperseded(): void {
	counters.runsSuperseded += 1;
}

/** A completed layout was thrown away for being stale — the supersede that arrived too late. */
export function noteElkResultDiscarded(): void {
	counters.elkResultsDiscarded += 1;
}

/** One `elk.layout()` call is about to start — the expensive thing this work exists to do less of. */
export function noteElkRun(): void {
	counters.elkRuns += 1;
}

/** Called on every write to the node store, so a remount loop is visible as a count. */
export function noteNodeStoreWrite(nodeCount: number): void {
	counters.nodeStoreWrites += 1;
	counters.peakStoreNodes = Math.max(counters.peakStoreNodes, nodeCount);
}

/**
 * What one pipeline run did, recorded as it happens.
 *
 * The per-sample ring says *that* containers went zero-sized; it cannot say which run did it or
 * why. Every capture so far shows the same shape — a `pipeline` sample with degenerate containers
 * after a clean one — and the last one placed the onset 66 seconds after the user stopped
 * interacting, so the run was data-driven rather than a click. These fields separate the remaining
 * candidates: what started the run, whether it rebuilt the layout graph (which resets every
 * container's `expandedSize` to zero), whether ELK ran to put sizes back, and how many containers
 * the graph still believed were zero-sized when it finished.
 *
 * `containersZeroSizedAfter` is the one that matters: it reads the layout *model*, not the DOM, so
 * it distinguishes "the graph lost the sizes" from "the render is behind".
 */
export interface RunRecord {
	at: number;
	/** What triggered it — `collapsed`, `topology`, `pending`, and so on. */
	source: string;
	isNewStructure?: boolean;
	needsElk?: boolean;
	/** The layout graph was rebuilt, resetting every `expandedSize` to `{0, 0}`. */
	graphRebuilt?: boolean;
	/** Containers ELK returned a size for, when it ran. */
	elkSizedContainers?: number;
	/** Wall-clock milliseconds for the whole run. `perf.ts` times the stages, but it records
	 *  nothing in a customer's build, so the total is kept here where it is always on. */
	durationMs?: number;
	/**
	 * Live heap in MB entering and leaving the run, and the reserved heap on the way out.
	 *
	 * The pair is what identifies an expensive run. A run that enters at 300 and leaves at 300 cost
	 * nothing lasting even if it briefly allocated a great deal; one that leaves far above where it
	 * started is accumulating. `heapTotalEndMb` is the more sensitive of the three to a spike that
	 * has already been collected, since V8 gives reserved pages back slowly.
	 */
	heapStartMb?: number;
	heapEndMb?: number;
	heapTotalEndMb?: number;
	/** This run was abandoned before ELK because a later input superseded it. No layout was paid for. */
	supersededBeforeElk?: boolean;
	/**
	 * This run laid out in full and its result was discarded for being stale.
	 *
	 * The other half of `supersededBeforeElk`, and the expensive one. The stale check sits before
	 * `layoutEngine.compute` and again after it, with nothing in between, so a supersede raised any
	 * time after the run entered ELK lands here instead — the whole layout paid for, nothing drawn.
	 * A buffer full of these against a healthy `runsSuperseded` means cancellation is firing too
	 * late to be worth anything, which is invisible from the cumulative counters alone.
	 */
	supersededAfterElk?: boolean;
	/**
	 * Expanded containers whose `expandedSize` is still zero once the run finished.
	 *
	 * The number to read first. Anything above zero here means the layout model itself lost the
	 * sizes, and every one of those containers will draw as its borders with its contents outside.
	 * Collapsed containers are excluded — they have never been laid out expanded, so a zero is
	 * expected and would swamp the signal.
	 */
	containersZeroSizedAfter?: number;
}

/** Last few runs, so the transition into a bad state is visible rather than just its aftermath. */
const RUN_HISTORY_LIMIT = 12;
const runs: RunRecord[] = [];

export function noteRunStart(source: string): void {
	noteHeapSample();
	runs.push({
		at: Math.round(performance.now()),
		source,
		...(usedJSHeapMb() !== undefined && { heapStartMb: usedJSHeapMb() })
	});
	if (runs.length > RUN_HISTORY_LIMIT) runs.shift();
}

/** Close the run currently in flight, recording how long it took and what it cost. */
export function noteRunEnd(): void {
	noteHeapSample();
	const current = runs.at(-1);
	if (current && current.durationMs === undefined) {
		current.durationMs = Math.round(performance.now() - current.at);
		const used = usedJSHeapMb();
		if (used !== undefined) current.heapEndMb = used;
		const total = totalJSHeapMb();
		if (total !== undefined) current.heapTotalEndMb = total;
	}
}

/** Fill in detail on the run currently in flight. */
export function noteRunDetail(patch: Partial<RunRecord>): void {
	noteHeapSample();
	const current = runs.at(-1);
	if (current) Object.assign(current, patch);
}

/** Called when the pipeline takes the full measurement path, which mounts every node. */
export function noteFullMeasurePass(): void {
	counters.fullMeasurePasses += 1;
}

export interface ViewerSample {
	/** Milliseconds since page load, so a reader can see the spacing between samples. */
	at: number;
	/** What prompted this sample. */
	trigger: 'pipeline' | 'viewport' | 'manual';
	view: string;
	/** Nodes and edges the store holds — what *should* be drawn. */
	store: { nodes: number; edges: number };
	/** Nodes SvelteFlow actually mounted. The gap between this and `store.nodes` is culling. */
	mounted: number;
	/** How many mounted nodes report a non-zero size. Zero here with a non-zero mount count is
	 *  the measure pass having produced nothing, which starves both layout and fit-view. */
	withSize: number;
	/**
	 * Mounted nodes rendering at essentially no size in either dimension, split by node type.
	 *
	 * `withSize` cannot see this fault and never could: it asks whether a node's bounding box is
	 * non-zero, and a container collapsed to its own borders is 250x2 — non-zero, and counted as
	 * healthy. But a container at 2px has its contents drawn outside it, which is what an operator
	 * reports as "the nodes didn't finish resizing". Width and height are both tested because they
	 * collapse independently — the same fault produced 250x2 and 2x2 nodes in the same graph.
	 *
	 * Split by type because the two causes are different. Containers go degenerate when ELK never
	 * sized them — `LayoutContainer.expandedSize` starts at `{0, 0}` and `getContainerSize` returns
	 * that rather than `undefined`, so a container it never laid out is built with `height: 0`.
	 * Elements go degenerate when a measurement pass is mid-flight, since it deliberately builds
	 * them unsized.
	 */
	degenerate: { containers: number; elements: number };
	culling: {
		/** The value actually handed to SvelteFlow, not a re-derivation of it. */
		on: boolean;
		threshold: number;
		measuring: boolean;
		exporting: boolean;
		/** `window.__topoNoCull` — tooling suppressing culling entirely. */
		suppressedForTooling: boolean;
	};
	/** How much of the store SvelteFlow could cull even in principle. */
	cullable: CullabilitySummary | null;
	/**
	 * What the renderer is actually holding.
	 *
	 * The JS heap is a small fraction of a topology tab's memory — the dominant cost is the DOM
	 * itself, since Blink accounts for every node's style and layout objects whether or not
	 * anything reads them. Measured on the seeded reproduction that came to roughly 6 KB per
	 * element, so these counts track the tab's footprint far better than `usedJSHeapMb` does, and
	 * unlike it they are available in every browser.
	 *
	 * `inNodes` is the one to watch: it is `mounted` multiplied by however many elements a card
	 * renders, so it moves with both culling and card complexity.
	 */
	dom: {
		/** Every element in the document, so the topology's share of the page is visible. */
		total: number;
		/** Elements inside mounted nodes. */
		inNodes: number;
		/** Handle divs — geometry SvelteFlow reads, one or two per node with an edge. */
		handles: number;
		edgePaths: number;
	};
	/** The pane's own size. Zero either dimension and *everything* is off-screen by definition. */
	pane: { width: number; height: number };
	/** SvelteFlow's transform, verbatim. */
	transform: string | null;
	/** Bounding box of the mounted nodes in screen space, and whether it meets the pane at all. */
	bounds: { left: number; top: number; right: number; bottom: number } | null;
	intersectsPane: boolean;
	/**
	 * The zoom a fit needs for the current graph, against the zoom the floor allows.
	 *
	 * The quantity a capture of this fault had no way to express. A report showed every sample in
	 * the ring at `scale(0.1)` — both views, all four collapse levels, after eight separate fits on
	 * eight different graph sizes — which reads as a coincidence until you know 0.1 was the floor
	 * and every one of those fits was clamping. `clampedAtFloor` says so directly, and `required`
	 * says by how far: a graph needing 0.03 is being shown at a third of its width with the rest
	 * off-screen, which is what gets reported as an empty canvas.
	 *
	 * `null` when nothing is laid out yet, since there are no bounds to fit.
	 */
	fitZoom: { required: number; applied: number; clampedAtFloor: boolean } | null;
	/**
	 * The collapse level, and how many containers are collapsed at it.
	 *
	 * Note the two counts in this sample are over different populations and are not comparable.
	 * `collapsedContainers` is computed over the raw payload and includes containers nested inside
	 * other collapsed containers, while `store.nodes` is what survived collapse pruning and
	 * filtering — so 2,693 collapsed containers beside a 760-node store is the expected shape at a
	 * low level, not a counter overflowing its population. Level 1 is fully collapsed and level 4
	 * fully expanded, so zero here at level 4 is the definition, not an absence.
	 */
	collapse: { level: number | null; collapsedContainers: number };
	/**
	 * The payload's edges by `edge_type`, before anything the renderer does to them.
	 *
	 * `store.edges` counts what survived elevation, bundling, collapse aggregation and culling, so
	 * a low number answers to two completely different faults with the same figure: the server sent
	 * few edges, or it sent many and the pipeline dropped them. In L2 Physical the distinction is
	 * the whole diagnosis — 40 `NeighborLink`s means neighbour resolution degraded to device level
	 * and the fix is server-side, while 700 `PhysicalLink`s that never reached the store means the
	 * fix is here.
	 */
	edgesByType: Record<string, number>;
	/**
	 * The payload's resolved-adjacency rows (`neighbours`, GH #701) by kind: `Interface` is
	 * port-precise (draws a solid `PhysicalLink`), `Host` is device-level (a dashed `NeighborLink`).
	 * `none` counts *interfaces* with zero rows of their own, since a port with no adjacency draws
	 * nothing. Keyed by the backend's own discriminants.
	 *
	 * Rows, not interfaces, for `Interface`/`Host`: a port can now carry several adjacency rows
	 * (an LLDP entry and a CDP entry resolving to different neighbours), so one interface can
	 * contribute to both buckets — this is no longer a partition of the interface set the way it
	 * was before a port's resolution became a `Vec`.
	 *
	 * The upstream half of `edgesByType`: it says whether the edge mix the renderer was handed is
	 * what the data supports, so "almost everything is dashed" can be attributed to resolution
	 * rather than to rendering without a database.
	 */
	interfaceNeighborKinds: Record<NeighborKind | 'none', number>;
	blank: BlankReason;
}

const history: ViewerSample[] = [];

interface SampleInputs {
	storeNodes: number;
	storeEdges: number;
	/** A DOM measurement pass is running, on any load. Suspends culling. */
	measuring: boolean;
	/** The cold load is hiding the pane while it measures. Only ever true on the first render. */
	coldLoadMeasure: boolean;
	exporting: boolean;
	/** The value the viewer passed to `onlyRenderVisibleElements`. */
	culling: boolean;
	/**
	 * This viewer's own element. Scopes the DOM reads below.
	 *
	 * The counts used to come from `document`, which also finds any other `<SvelteFlow>` mounted
	 * at the time — the dependency tutorial and the read-only viewer both mount one — so a report
	 * could attribute another canvas's nodes to this one.
	 */
	container: HTMLElement | null;
	/** SvelteFlow's internal nodes, for the cullability summary. */
	internalNodes: () => (CullableNode | undefined)[];
	/**
	 * The payload as the server sent it, read lazily.
	 *
	 * Summarised rather than stored: the counts are what a report needs, and a sample holding the
	 * arrays would pin the whole topology in the ring buffer.
	 */
	payload: () => DiagnosablePayload | null;
	/**
	 * The zoom a fit needs against the floor allowing it, read lazily.
	 *
	 * Supplied by the viewer rather than derived here: the graph's bounds live in the node store in
	 * graph coordinates, and this file only ever reads the DOM, which at low zoom and with culling
	 * on holds a clipped subset of them.
	 */
	fitZoom: () => ViewerSample['fitZoom'];
	trigger: ViewerSample['trigger'];
}

/** The parts of the render payload a sample summarises, taken from the generated schema so the
 *  neighbour discriminants stay the backend's. */
export interface DiagnosablePayload {
	edges?: { edge_type?: unknown }[];
	interfaces?: { id?: string }[];
	/** GH #701: resolved adjacency rows, replacing the old per-interface `neighbor` scalar. */
	neighbours?: { interface_id?: string; neighbor?: components['schemas']['Neighbor'] | null }[];
}

type NeighborKind = components['schemas']['Neighbor']['type'];

/** Every neighbour discriminant, plus the un-resolved case the column expresses as NULL. */
const NEIGHBOR_KINDS = ['Interface', 'Host'] as const satisfies readonly NeighborKind[];

/** The payload summaries, and the payload they were taken from.
 *
 * Held because sampling happens on the throttled viewport path — twice a second while someone
 * pans — and these two walks are O(edges + interfaces) with no cap. At the scale this diagnostic
 * exists to explain, that is ~15k interfaces and thousands of edges rescanned per pan frame: the
 * same cost `CULLABILITY_SAMPLE_LIMIT` exists to avoid, and worse, because it would be paid on
 * every sample rather than sampled down.
 *
 * Memoised on payload identity rather than capped, because unlike the cullability walk this one
 * reads a value that cannot change between refetches — the payload is one object, replaced
 * wholesale when new data arrives. So the exact figure stays available at no recurring cost.
 */
let payloadSummary: {
	payload: DiagnosablePayload;
	edgesByType: Record<string, number>;
	interfaceNeighborKinds: ViewerSample['interfaceNeighborKinds'];
} | null = null;

/** Edge-type and neighbour-kind counts for a payload, computed once per payload. */
export function summarisePayload(payload: DiagnosablePayload | null): {
	edgesByType: Record<string, number>;
	interfaceNeighborKinds: ViewerSample['interfaceNeighborKinds'];
} {
	if (!payload) {
		return { edgesByType: {}, interfaceNeighborKinds: { Interface: 0, Host: 0, none: 0 } };
	}
	if (payloadSummary?.payload !== payload) {
		payloadSummary = {
			payload,
			edgesByType: summariseEdgeTypes(payload),
			interfaceNeighborKinds: summariseNeighborKinds(payload)
		};
	}
	return {
		edgesByType: payloadSummary.edgesByType,
		interfaceNeighborKinds: payloadSummary.interfaceNeighborKinds
	};
}

/** Payload edge counts keyed by `edge_type`. The type is a tagged union on the wire, so its
 *  discriminant is read from either the bare string or the `type` tag. */
function summariseEdgeTypes(payload: DiagnosablePayload | null): Record<string, number> {
	const out: Record<string, number> = {};
	for (const edge of payload?.edges ?? []) {
		const raw = edge.edge_type;
		const key =
			typeof raw === 'string' ? raw : ((raw as { type?: string } | null)?.type ?? 'unknown');
		out[key] = (out[key] ?? 0) + 1;
	}
	return out;
}

function summariseNeighborKinds(
	payload: DiagnosablePayload | null
): ViewerSample['interfaceNeighborKinds'] {
	const out: ViewerSample['interfaceNeighborKinds'] = { Interface: 0, Host: 0, none: 0 };
	const interfacesWithRows = new Set<string>();
	for (const row of payload?.neighbours ?? []) {
		const kind = row.neighbor?.type;
		if (kind && NEIGHBOR_KINDS.includes(kind)) out[kind] += 1;
		if (row.interface_id) interfacesWithRows.add(row.interface_id);
	}
	for (const iface of payload?.interfaces ?? []) {
		if (!iface.id || !interfacesWithRows.has(iface.id)) out.none += 1;
	}
	return out;
}

/**
 * Inspect at most this many internal nodes on the throttled viewport path.
 *
 * A pan samples up to twice a second and the walk is O(nodes); at 17,000 nodes doing it in full
 * would be a cost the diagnostic itself imposes on the case it exists to diagnose. Pipeline and
 * manual samples count everything, so the exact figure is always available when it matters.
 */
const CULLABILITY_SAMPLE_LIMIT = 500;

function sampleCullability(nodes: (CullableNode | undefined)[], full: boolean): CullabilitySummary {
	if (full || nodes.length <= CULLABILITY_SAMPLE_LIMIT) {
		return { ...summariseCullability(nodes), sampled: false };
	}
	const stride = Math.ceil(nodes.length / CULLABILITY_SAMPLE_LIMIT);
	const sampled: (CullableNode | undefined)[] = [];
	for (let i = 0; i < nodes.length; i += stride) sampled.push(nodes[i]);
	return { ...summariseCullability(sampled), sampled: true };
}

/**
 * The viewer's own pane, or the first one in the document if it hasn't bound yet.
 *
 * `container` is a `bind:this`, so it is null for the first sample or two after mount; the
 * document-wide fallback keeps those samples useful. Everywhere else it matters: another
 * `<SvelteFlow>` can be mounted at the same time (`DependencyTutorial`, `ReadOnlyTopologyViewer`),
 * and its pane would otherwise be measured against this viewer's node counts.
 */
function readPane(container: HTMLElement | null): { el: HTMLElement | null; rect: DOMRect | null } {
	const root: ParentNode = container ?? document;
	const el = root.querySelector('.svelte-flow') as HTMLElement | null;
	return { el, rect: el?.getBoundingClientRect() ?? null };
}

/**
 * One cheap snapshot of everything that decides whether the graph is visible.
 *
 * Reads the DOM rather than SvelteFlow's internals deliberately: the question is what the user is
 * looking at, and the store's opinion of that is exactly what is in doubt.
 */
export function sampleViewerState(inputs: SampleInputs): ViewerSample {
	noteHeapSample();
	const { edgesByType, interfaceNeighborKinds } = summarisePayload(inputs.payload());
	const root: ParentNode = inputs.container ?? document;
	const { el: pane, rect: paneRect } = readPane(inputs.container);
	const viewport = root.querySelector('.svelte-flow__viewport') as HTMLElement | null;
	const nodeEls = Array.from(root.querySelectorAll('.svelte-flow__node')) as HTMLElement[];

	let bounds: ViewerSample['bounds'] = null;
	let withSize = 0;
	const degenerate = { containers: 0, elements: 0 };
	for (const el of nodeEls) {
		const r = el.getBoundingClientRect();
		if (r.width > 0 && r.height > 0) withSize += 1;
		// Both dimensions, because they fail independently: a container ELK never sized was
		// measured here at 250x2 in one case and 2x2 in another, the first keeping a width from
		// CSS while the second lost both. Layout size, not the bounding rect — the rect is scaled
		// by the viewport transform, so at low zoom every node looks tiny.
		if (el.offsetWidth <= DEGENERATE_SIZE_PX || el.offsetHeight <= DEGENERATE_SIZE_PX) {
			if (el.classList.contains('svelte-flow__node-Container')) degenerate.containers += 1;
			else degenerate.elements += 1;
		}
		bounds = bounds
			? {
					left: Math.min(bounds.left, r.left),
					top: Math.min(bounds.top, r.top),
					right: Math.max(bounds.right, r.right),
					bottom: Math.max(bounds.bottom, r.bottom)
				}
			: { left: r.left, top: r.top, right: r.right, bottom: r.bottom };
	}

	const intersectsPane = Boolean(
		bounds &&
			paneRect &&
			bounds.right > paneRect.left &&
			bounds.left < paneRect.right &&
			bounds.bottom > paneRect.top &&
			bounds.top < paneRect.bottom
	);

	const paneHidden = pane ? getComputedStyle(pane).visibility === 'hidden' : false;
	const blank = classifyBlank({
		storeNodes: inputs.storeNodes,
		paneHidden,
		coldLoadMeasure: inputs.coldLoadMeasure,
		mountedCount: nodeEls.length,
		intersectsPane
	});

	counters.peakMounted = Math.max(counters.peakMounted, nodeEls.length);

	const dom = {
		total: document.querySelectorAll('*').length,
		inNodes: root.querySelectorAll('.svelte-flow__node *').length,
		handles: root.querySelectorAll('.svelte-flow__handle').length,
		edgePaths: root.querySelectorAll('.svelte-flow__edge path').length
	};
	counters.peakDomInNodes = Math.max(counters.peakDomInNodes, dom.inNodes);

	const round = (n: number) => Math.round(n);
	return {
		at: round(performance.now()),
		trigger: inputs.trigger,
		view: get(activeView),
		store: { nodes: inputs.storeNodes, edges: inputs.storeEdges },
		mounted: nodeEls.length,
		withSize,
		degenerate,
		culling: {
			// The value the viewer actually handed to SvelteFlow. This used to be re-derived from
			// the node count instead — on the reasoning that a sample stays meaningful even if the
			// two disagree — which is backwards for the fault it was built to catch: the report
			// that mattered said `on: true` beside `mounted == store.nodes`, and the re-derivation
			// is exactly what could not be trusted. It also silently omitted the `__topoNoCull`
			// term, so a tooling run would have been reported as culling when it was not.
			on: inputs.culling,
			threshold: CULLING_THRESHOLD_ELEMENTS,
			measuring: inputs.measuring,
			exporting: inputs.exporting,
			suppressedForTooling: cullingSuppressedForTooling()
		},
		// Full count on the two triggers that are taken rarely; stride-sampled on the throttled
		// viewport path, which fires throughout a pan.
		cullable: sampleCullability(inputs.internalNodes(), inputs.trigger !== 'viewport'),
		dom,
		pane: { width: round(pane?.clientWidth ?? 0), height: round(pane?.clientHeight ?? 0) },
		transform: viewport?.style.transform ?? null,
		bounds: bounds
			? {
					left: round(bounds.left),
					top: round(bounds.top),
					right: round(bounds.right),
					bottom: round(bounds.bottom)
				}
			: null,
		intersectsPane,
		fitZoom: inputs.fitZoom(),
		collapse: {
			level: get(collapseLevel) ?? null,
			collapsedContainers: get(collapsedContainers).size
		},
		edgesByType,
		interfaceNeighborKinds,
		blank
	};
}

/**
 * The buffer as it stood the first time the canvas went blank, kept verbatim.
 *
 * The ring alone is not enough. Someone who has just lost the diagram pans, zooms, clicks the
 * minimap and presses F before thinking to run anything — every one of which records a sample, at
 * up to two a second. Fifteen seconds of that evicts the runs that led *into* the blank, which is
 * the only part that identifies a cause. So the moment blankness is first seen, the buffer is
 * copied somewhere nothing overwrites, and the report carries it however long they take.
 */
let firstBlankCapture: ViewerSample[] | null = null;

/**
 * The same, for the *most recent* time it went blank.
 *
 * `firstBlankCapture` latches for the life of the tab, which is the right thing for a fault seen
 * once on a fresh load and the wrong thing for everything else. A capture that came in for
 * diagnosis had been open for 23 hours 50 minutes: its `firstBlank` was a transient from minute
 * 0:49 of the previous day, the ring held the last eight minutes, and the blank being complained
 * about fell in the gap between them and appeared nowhere in the file. A topology tab is left open
 * all day, so that is the normal case, not an unlucky one.
 *
 * Keeping both costs one more array of at most `HISTORY_LIMIT` samples and means a report always
 * carries the lead-in to *a* blank the reporter might actually have seen.
 */
let lastBlankCapture: ViewerSample[] | null = null;

function push(sample: ViewerSample): void {
	history.push(sample);
	if (history.length > HISTORY_LIMIT) history.shift();
}

/** The last sample's blank reason, so a transition is announced once rather than every frame. */
let previousBlank: BlankReason = null;

function record(inputs: SampleInputs): void {
	const sample = sampleViewerState(inputs);
	push(sample);

	// Announce the edge, not the state. A blank canvas that persists across a pan would otherwise
	// fill the console with the same line and bury whatever else is there.
	if (sample.blank && sample.blank !== previousBlank) {
		// Before anything else can push it out of the ring.
		firstBlankCapture ??= [...history];
		lastBlankCapture = [...history];
		console.warn(
			`[scanopy] topology canvas is blank (${sample.blank}): ` +
				`${sample.store.nodes} nodes in the graph, ${sample.mounted} drawn, ` +
				`culling ${sample.culling.on ? 'on' : 'off'}, level ${sample.collapse.level}. ` +
				`Run scanopyTopologyDiagnostics() to save a report.`
		);
	}
	previousBlank = sample.blank;
}

/** Called after each pipeline run, once the viewport has settled. */
export function recordAfterRun(inputs: Omit<SampleInputs, 'trigger'>): void {
	counters.pipelineRuns += 1;
	record({ ...inputs, trigger: 'pipeline' });
}

/**
 * Called on viewport movement, throttled.
 *
 * `leading: false, trailing: true` matches the convention used elsewhere: a pan's *final* position
 * is the one worth recording, and sampling its first frame would capture the state being left.
 */
export const recordAfterViewportMove = throttle(
	(inputs: Omit<SampleInputs, 'trigger'>) => record({ ...inputs, trigger: 'viewport' }),
	VIEWPORT_SAMPLE_INTERVAL_MS,
	{ leading: false, trailing: true }
);

export interface DiagnosticsReport {
	generatedAt: string;
	userAgent: string;
	screen: { width: number; height: number; devicePixelRatio: number };
	/**
	 * The buffer as it stood when the canvas first went blank, or `null` if it never did.
	 *
	 * Note this is the buffer, not a list of blanks: all but the last entry are the lead-in, with
	 * `blank: null`, and the last is the sample that tripped it. A report of seventeen entries
	 * describes one blank and sixteen healthy frames before it.
	 *
	 * `lastBlank` below is usually the one to read first. This one only wins when the fault was on
	 * a fresh load, since it never updates again afterwards.
	 */
	firstBlank: ViewerSample[] | null;
	/**
	 * The same buffer for the most recent blank — normally the one being reported.
	 *
	 * Identical to `firstBlank` when the canvas has only gone blank once. When they differ, the tab
	 * has been open long enough to matter and this is the recent evidence; `samples` is whatever
	 * the ring holds now, which after any panning is the aftermath rather than the cause.
	 */
	lastBlank: ViewerSample[] | null;
	samples: ViewerSample[];
	/** What each recent pipeline run did — see `RunRecord`. */
	runs: RunRecord[];
	/** Stage timings, counts, and per-stage peak heap, when `perf.ts` is recording. */
	perf: {
		durations: Record<string, number>;
		counts: Record<string, number>;
		/** Highest heap seen at each stage's end — a session maximum per stage name. See `perf.ts`. */
		heapAfterMb: Record<string, number>;
		/**
		 * Ordered before/after heap readings for the run that grew the heap most, and the last run.
		 *
		 * Read this to attribute a single run's growth. `heapAfterMb` mixes runs, so it cannot say
		 * where inside one run the memory went.
		 */
		worstRun: PerfRunLedger | null;
		lastRun: PerfRunLedger | null;
		runs: number;
	} | null;
	/**
	 * Totals for the session, which the per-sample ring cannot express.
	 *
	 * Read these against `samples`: they are what distinguishes one graph too large to mount from
	 * the same graph being mounted repeatedly, which the ring buffer alone cannot separate.
	 */
	cumulative: CumulativeCounters;
}

function readPerfSnapshot() {
	const api = (
		window as unknown as {
			__scanopyTopologyPerf?: {
				snapshot: () => {
					durations: Record<string, number>;
					counts: Record<string, number>;
					heapAfterMb: Record<string, number>;
					worstRun: PerfRunLedger | null;
					lastRun: PerfRunLedger | null;
					runs: number;
				};
			};
		}
	).__scanopyTopologyPerf;
	if (!api) return null;
	const snap = api.snapshot();
	// Nothing recorded means the build is not instrumented, which is worth distinguishing from
	// "instrumented and fast".
	return Object.keys(snap.counts).length === 0 ? null : snap;
}

function buildReport(current: ViewerSample): DiagnosticsReport {
	const heap = usedJSHeapMb();
	const heapTotal = totalJSHeapMb();
	return {
		generatedAt: new Date().toISOString(),
		// The fault is reported as browser-dependent, so the browser is part of the evidence.
		userAgent: navigator.userAgent,
		screen: {
			width: window.innerWidth,
			height: window.innerHeight,
			devicePixelRatio: window.devicePixelRatio
		},
		firstBlank: firstBlankCapture,
		lastBlank: lastBlankCapture,
		runs: [...runs],
		samples: [...history, current],
		cumulative: {
			...counters,
			...(heap !== undefined && { usedJSHeapMb: heap }),
			...(heapTotal !== undefined && { totalJSHeapMb: heapTotal })
		},
		/**
		 * Stage timings, when the build records them.
		 *
		 * `perf.ts` is gated on a dev build or `window.__topoPerf`, so this is usually absent in a
		 * customer capture — but when someone can set the flag and reproduce, it is the difference
		 * between "the view is slow" and knowing which stage is slow.
		 */
		perf: readPerfSnapshot()
	};
}

/**
 * Publish `scanopyTopologyDiagnostics()` on `window`.
 *
 * One command for a customer to run and one file to send back. It takes a fresh sample first, so
 * it works when called *while* the canvas is blank — which is when someone will reach for it.
 */
export interface DiagnosticsOptions {
	/**
	 * Save the report to a file. On by default — the point of the command is one file to send
	 * back. Tests pass `false`: they want the object, and a download triggers a browser prompt.
	 */
	download?: boolean;
}

export function installDiagnostics(read: () => Omit<SampleInputs, 'trigger'>): void {
	if (typeof window === 'undefined') return;
	(
		window as unknown as {
			scanopyTopologyDiagnostics?: (options?: DiagnosticsOptions) => DiagnosticsReport;
		}
	).scanopyTopologyDiagnostics = (options?: DiagnosticsOptions) => {
		const current = sampleViewerState({ ...read(), trigger: 'manual' });
		const report = buildReport(current);

		if (options?.download ?? true) {
			const blob = new Blob([JSON.stringify(report, null, 2)], { type: 'application/json' });
			const url = URL.createObjectURL(blob);
			const link = document.createElement('a');
			link.href = url;
			link.download = `scanopy-topology-diagnostics-${Date.now()}.json`;
			link.click();
			URL.revokeObjectURL(url);
		}

		// Returned as well as downloaded: a browser that blocks the download still leaves the
		// object in the console to copy.
		return report;
	};
}

/** Test seam — the ring buffer is module state and specs need a clean one. */
export function resetDiagnostics(): void {
	payloadSummary = null;
	history.length = 0;
	previousBlank = null;
	firstBlankCapture = null;
	lastBlankCapture = null;
	counters.pipelineRuns = 0;
	counters.nodeStoreWrites = 0;
	counters.fullMeasurePasses = 0;
	counters.peakStoreNodes = 0;
	counters.peakMounted = 0;
	counters.peakDomInNodes = 0;
	counters.runsSuperseded = 0;
	counters.elkResultsDiscarded = 0;
	counters.elkRuns = 0;
	runs.length = 0;
}

/** Test seam. */
export function diagnosticsHistory(): ViewerSample[] {
	return [...history];
}
