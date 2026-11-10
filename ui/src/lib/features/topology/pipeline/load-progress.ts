/**
 * Progress of a topology load, by stage.
 *
 * A load has no continuous progress to report. Most of the download is the server building the
 * response, and the layout engine runs as one long main-thread task per pass, during which no
 * script can run at all. What is known for certain is which stage is running and when it ends.
 *
 * So the bar is planned from stages. Each stage owns a slice of the bar sized by how long that
 * stage took last time for the same topology and view (an estimate from the node count the first
 * time). The bar reaches the end of a slice only when its stage ends. Inside a slice the fill
 * moves at the recorded pace as a CSS transition, which the compositor keeps animating while the
 * main thread is blocked; that is the one part that is an estimate and not a measurement.
 */

import { writable, get } from 'svelte/store';
import { browser } from '$app/environment';

export type LoadStage = 'data' | 'measure' | 'layout' | 'layoutRefine' | 'draw';

/** Every stage a load can pass through, in order. A run enters a subset of them. */
export const LOAD_STAGES: LoadStage[] = ['data', 'measure', 'layout', 'layoutRefine', 'draw'];

export interface LoadProgress {
	stage: LoadStage;
	/** Where the bar stands when this stage starts, 0 to 1. */
	from: number;
	/** Where the bar stands when this stage ends. It does not pass this while the stage runs. */
	to: number;
	/** How long this stage is expected to take. */
	expectedMs: number;
	/** How long the stages still to come, this one included, are expected to take. */
	remainingMs: number;
}

/** The stage a load is in, or null when nothing is loading. */
export const loadProgress = writable<LoadProgress | null>(null);

export type StageTimes = Partial<Record<LoadStage, number>>;

/**
 * Expected duration of each stage when no run has been recorded for the scope.
 *
 * Rough by design: it only sizes the slices of the first bar a user sees for a view. Layout and
 * drawing grow with the node count; the download does not know it yet.
 */
export function estimateStageTimes(nodeCount: number): Record<LoadStage, number> {
	return {
		data: 800,
		measure: 150 + nodeCount * 0.1,
		layout: 100 + nodeCount * 0.6,
		layoutRefine: 100 + nodeCount * 0.6,
		draw: 150 + nodeCount * 0.3
	};
}

/**
 * Slices of the bar for `stages`, from `start` to 1, each proportional to its expected duration.
 */
export function planStages(
	stages: LoadStage[],
	expected: Record<LoadStage, number>,
	start = 0
): Map<LoadStage, LoadProgress> {
	const plan = new Map<LoadStage, LoadProgress>();
	const total = stages.reduce((sum, stage) => sum + Math.max(1, expected[stage]), 0);
	let from = start;
	let remainingMs = total;
	for (const stage of stages) {
		const expectedMs = Math.max(1, expected[stage]);
		const to = from + ((1 - start) * expectedMs) / total;
		plan.set(stage, { stage, from, to, expectedMs, remainingMs });
		from = to;
		remainingMs -= expectedMs;
	}
	return plan;
}

const STORAGE_KEY = 'scanopy_topology_stage_times';
/** Scopes kept. One entry is a handful of numbers. */
const MAX_SCOPES = 24;

function loadRecords(): Record<string, StageTimes> {
	if (!browser) return {};
	try {
		const parsed = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}');
		return parsed && typeof parsed === 'object' ? parsed : {};
	} catch {
		return {};
	}
}

const records = loadRecords();

function saveRecords(): void {
	if (!browser) return;
	const keys = Object.keys(records);
	for (const key of keys.slice(0, Math.max(0, keys.length - MAX_SCOPES))) delete records[key];
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(records));
	} catch {
		// The next load plans from the estimate.
	}
}

/** What the last run recorded for `scope`, over the estimate for whatever it did not reach. */
export function expectedStageTimes(scope: string, nodeCount: number): Record<LoadStage, number> {
	return { ...estimateStageTimes(nodeCount), ...records[scope] };
}

let plan = new Map<LoadStage, LoadProgress>();
let scopeOfRun = '';
let stageStartedAt = 0;
const measured: StageTimes = {};

function now(): number {
	return typeof performance === 'undefined' ? Date.now() : performance.now();
}

function closeStage(): void {
	const current = get(loadProgress);
	if (!current) return;
	measured[current.stage] = (measured[current.stage] ?? 0) + (now() - stageStartedAt);
}

/**
 * Plan a load over `stages` and enter the first one.
 *
 * Called again by a later part of the same load (the pipeline, once the download has handed it
 * the data and it knows the node count), it re-plans the stages still to come from wherever the
 * bar stands, so the bar only ever moves forward.
 */
export function beginLoad(scope: string, stages: LoadStage[], nodeCount = 0): LoadProgress | null {
	const current = get(loadProgress);
	// The download is still the download when the thing waiting on it changes (the site list
	// arrives and names the topology): keep the bar where it is and only take the new scope.
	if (current?.stage === 'data' && stages[0] === 'data') {
		scopeOfRun = scope;
		return current;
	}
	closeStage();
	// After the download, the rest of the load takes the bar from where the download left it. A
	// run that starts over mid-load (a corrective re-layout after the first draw) shares the slice
	// of the stage it interrupted.
	const start =
		current && scope === scopeOfRun ? (current.stage === 'data' ? current.to : current.from) : 0;
	if (start === 0) for (const stage of LOAD_STAGES) delete measured[stage];
	scopeOfRun = scope;
	plan = planStages(stages, expectedStageTimes(scope, nodeCount), start);
	return enter(stages[0]);
}

function enter(stage: LoadStage): LoadProgress | null {
	const next = plan.get(stage) ?? null;
	stageStartedAt = now();
	loadProgress.set(next);
	return next;
}

/** Move to `stage`. Stages the run skipped (a stored layout skips both layout stages) are passed over. */
export function enterLoadStage(stage: LoadStage): LoadProgress | null {
	if (get(loadProgress)?.stage === stage) return get(loadProgress);
	closeStage();
	return enter(stage);
}

/** The load is over: keep what each stage took for the next plan, and clear the bar. */
export function endLoad(): void {
	if (!get(loadProgress)) return;
	closeStage();
	if (scopeOfRun) {
		const previous = records[scopeOfRun];
		delete records[scopeOfRun];
		records[scopeOfRun] = { ...previous, ...measured };
		saveRecords();
	}
	loadProgress.set(null);
}

/** A stage shorter than this is not worth a frame of delay to start its animation. */
const WORTH_A_FRAME_MS = 250;

/**
 * Let the browser paint once if `progress` is a stage long enough to notice.
 *
 * The fill animates on the compositor, but only from the frame that committed the new stage. A
 * stage that goes straight into a long synchronous task without one leaves the bar where it was
 * for the whole task. One frame costs about 16 ms, so it is spent only ahead of a stage expected
 * to block for long.
 */
export function paintBeforeLongStage(progress: LoadProgress | null): Promise<void> {
	if (!browser || !progress || progress.expectedMs < WORTH_A_FRAME_MS) return Promise.resolve();
	// The frame callback runs before that frame is painted, so resolve on the task after it.
	return new Promise((resolve) => requestAnimationFrame(() => setTimeout(resolve, 0)));
}
