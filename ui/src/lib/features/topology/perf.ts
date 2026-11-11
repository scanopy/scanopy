/**
 * Topology render-pipeline instrumentation.
 *
 * Exists so the Playwright perf harness (`ui/tests/topology-perf.ts`) can read
 * real stage timings and call counts out of the running app rather than
 * inferring them from wall-clock and DOM polling. Keeping it in the app — and
 * keeping it cheap — is what makes topology performance trackable over time
 * instead of a one-off measurement.
 *
 * Disabled unless the build is a dev build or `window.__topoPerf` is set, so
 * production pays nothing beyond one boolean test per call site.
 *
 * Usage:
 *   const done = perf.stage('elk');
 *   ...
 *   done();
 */

import { browser } from '$app/env';

export interface TopologyPerfSnapshot {
	/** Cumulative milliseconds spent in each stage, keyed by stage name. */
	durations: Record<string, number>;
	/** How many times each stage ran. */
	counts: Record<string, number>;
	/** Milliseconds since the current pipeline run started, if one is running. */
	runStartedAt: number | null;
	/** Completed pipeline runs since the last reset. */
	runs: number;
	/**
	 * Highest heap reading seen at each stage's *end*, in MB. Chrome-only, so absent elsewhere.
	 *
	 * Stage boundaries are the only place the peak is observable. The tab was measured swinging
	 * 881 MB to 2,050 MB and back within 90 seconds of interaction, but every capture taken after
	 * the fact reported a live heap of ~267 MB — the spike is transient and released, so a snapshot
	 * taken when someone notices the number always lands in a trough. ELK compounds this: it is the
	 * main-thread `elk.bundled.js`, so while it runs no timer or animation frame can fire and a
	 * sampler cannot see inside it at all.
	 *
	 * Reading it as a max rather than a mean is deliberate — one run that allocates 1.5 GB is the
	 * fault, and averaging it against a dozen cheap runs hides exactly that.
	 */
	heapAfterMb: Record<string, number>;
	/**
	 * Ordered heap ledger for the run that grew the heap most, and for the last run.
	 *
	 * `heapAfterMb` above is a session maximum per stage name, which cannot attribute a single
	 * run's growth: its entries come from different runs, so a sub-stage can read *lower* than the
	 * stage containing it — `measure.build-nodes` at 525MB inside a `measure` at 764MB, in one
	 * capture. Answering "where inside this run did 400MB go" needs before/after readings in order,
	 * from one run.
	 *
	 * Kept for the worst run and not only the last, because the expensive run is rarely the one
	 * someone stops on, and once it leaves the last-run slot the evidence is gone.
	 */
	worstRun: RunHeapLedger | null;
	lastRun: RunHeapLedger | null;
}

/** One stage's heap footprint within a single run. */
export interface StageHeapEntry {
	name: string;
	/** Live heap entering and leaving the stage, in MB. Chrome-only, so absent elsewhere. */
	heapBeforeMb?: number;
	heapAfterMb?: number;
	ms: number;
}

export interface RunHeapLedger {
	/** Heap growth across the run, in MB. Negative when GC ran partway through. */
	growthMb: number;
	/** In the order stages *started*, so nesting reads as containment. */
	stages: StageHeapEntry[];
}

/** Chrome-only, and non-standard, hence the cast rather than a global type. */
interface JSHeapMemory {
	usedJSHeapSize?: number;
	totalJSHeapSize?: number;
}

function heapMemory(): JSHeapMemory | undefined {
	if (!browser) return undefined;
	return (performance as Performance & { memory?: JSHeapMemory }).memory;
}

function toMb(bytes: number | undefined): number | undefined {
	return typeof bytes === 'number' ? Math.round(bytes / 1024 / 1024) : undefined;
}

/** Live heap in MB. Chrome-only; absent in Firefox and Safari. */
export function usedJSHeapMb(): number | undefined {
	return toMb(heapMemory()?.usedJSHeapSize);
}

/**
 * Heap V8 has reserved, in MB. Chrome-only.
 *
 * Larger than the live figure and slower to fall, so it retains evidence of a spike for a short
 * while after the objects themselves are collected — which makes it the better of the two to read
 * when a capture was taken shortly after the event rather than during it.
 */
export function totalJSHeapMb(): number | undefined {
	return toMb(heapMemory()?.totalJSHeapSize);
}

interface PerfGlobal {
	__topoPerf?: boolean;
	__scanopyTopologyPerf?: {
		snapshot: () => TopologyPerfSnapshot;
		reset: () => void;
	};
}

const durations: Record<string, number> = {};
const counts: Record<string, number> = {};
const heapAfterMb: Record<string, number> = {};
let runStartedAt: number | null = null;
let runs = 0;
let currentRunStages: StageHeapEntry[] = [];
let worstRun: RunHeapLedger | null = null;
let lastRun: RunHeapLedger | null = null;

function perfGlobal(): PerfGlobal | null {
	return browser ? (window as unknown as PerfGlobal) : null;
}

/**
 * Whether instrumentation should record. Read on every call rather than cached
 * so the harness can switch it on after load.
 */
export function enabled(): boolean {
	if (!browser) return false;
	return import.meta.env.DEV || perfGlobal()?.__topoPerf === true;
}

function record(name: string, elapsedMs: number): void {
	durations[name] = (durations[name] ?? 0) + elapsedMs;
	counts[name] = (counts[name] ?? 0) + 1;
}

/**
 * Time a stage. Returns a function to call when the stage finishes; calling it
 * more than once is ignored. When instrumentation is off this is a no-op
 * closure, so call sites need no branching of their own.
 */
export function stage(name: string): () => void {
	if (!enabled()) return () => {};
	const startedAt = performance.now();
	// Pushed on entry rather than on completion, so the ledger reads in start order and a stage
	// that contains others brackets them instead of trailing behind.
	const entry: StageHeapEntry = { name, heapBeforeMb: usedJSHeapMb(), ms: 0 };
	currentRunStages.push(entry);
	let finished = false;
	return () => {
		if (finished) return;
		finished = true;
		const elapsed = performance.now() - startedAt;
		record(name, elapsed);
		// Read before yielding: this runs synchronously at the stage boundary, so it sees the heap
		// as the stage left it rather than after the event loop has had a chance to collect.
		const used = usedJSHeapMb();
		if (used !== undefined) {
			heapAfterMb[name] = Math.max(heapAfterMb[name] ?? 0, used);
		}
		entry.heapAfterMb = used;
		entry.ms = Math.round(elapsed);
		performance.measure(`topology:${name}`, { start: startedAt, duration: elapsed });
	};
}

/** Count an event that has no duration worth timing. */
export function count(name: string): void {
	if (!enabled()) return;
	counts[name] = (counts[name] ?? 0) + 1;
}

export function beginRun(): void {
	if (!enabled()) return;
	runStartedAt = performance.now();
	currentRunStages = [];
}

export function endRun(): void {
	if (!enabled()) return;
	if (runStartedAt !== null) {
		record('run', performance.now() - runStartedAt);
		runStartedAt = null;
	}
	runs += 1;

	if (currentRunStages.length > 0) {
		// Growth across the run: the first stage's entry reading against the last stage's exit.
		// Both come from the same run, which is the whole point of keeping this separately from the
		// per-name maxima.
		const first = currentRunStages[0].heapBeforeMb;
		const last = currentRunStages[currentRunStages.length - 1].heapAfterMb;
		const growthMb = first !== undefined && last !== undefined ? last - first : 0;
		const ledger: RunHeapLedger = { growthMb, stages: currentRunStages };
		lastRun = ledger;
		if (!worstRun || growthMb > worstRun.growthMb) worstRun = ledger;
		currentRunStages = [];
	}
}

export function snapshot(): TopologyPerfSnapshot {
	return {
		durations: { ...durations },
		counts: { ...counts },
		heapAfterMb: { ...heapAfterMb },
		worstRun,
		lastRun,
		runStartedAt,
		runs
	};
}

export function reset(): void {
	for (const key of Object.keys(durations)) delete durations[key];
	for (const key of Object.keys(counts)) delete counts[key];
	for (const key of Object.keys(heapAfterMb)) delete heapAfterMb[key];
	currentRunStages = [];
	worstRun = null;
	lastRun = null;
	runStartedAt = null;
	runs = 0;
}

// Expose to the harness. Attached unconditionally in the browser so a test can
// set `window.__topoPerf = true` and then read results without a reload.
const globals = perfGlobal();
if (globals) {
	globals.__scanopyTopologyPerf = { snapshot, reset };
}
