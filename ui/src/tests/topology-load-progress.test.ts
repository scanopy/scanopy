import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
	LOAD_STAGES,
	beginLoad,
	cancelDownload,
	endLoad,
	endLoadOfRun,
	enterLoadStage,
	estimateStageTimes,
	loadProgress,
	planStages
} from '$lib/features/topology/pipeline/load-progress';

/**
 * The bar's one promise: it reaches the end of a stage's slice when that stage ends and not
 * before, and it never moves backwards within a load.
 */
describe('planStages', () => {
	it('sizes each slice by its expected share and ends at 1', () => {
		const plan = planStages(['measure', 'layout', 'draw'], {
			...estimateStageTimes(0),
			measure: 100,
			layout: 300,
			draw: 100
		});

		expect(plan.get('measure')).toMatchObject({ from: 0, to: 0.2, expectedMs: 100 });
		expect(plan.get('layout')).toMatchObject({ from: 0.2, expectedMs: 300 });
		expect(plan.get('layout')!.to).toBeCloseTo(0.8);
		expect(plan.get('draw')!.to).toBeCloseTo(1);
		expect(plan.get('measure')!.remainingMs).toBe(500);
		expect(plan.get('draw')!.remainingMs).toBe(100);
	});

	it('plans the remaining stages from where the bar already stands', () => {
		const plan = planStages(
			['layout', 'draw'],
			{ ...estimateStageTimes(0), layout: 100, draw: 100 },
			0.5
		);

		expect(plan.get('layout')).toMatchObject({ from: 0.5, to: 0.75 });
		expect(plan.get('draw')!.to).toBeCloseTo(1);
	});

	it('gives layout and drawing more of the first bar on a larger graph', () => {
		const small = estimateStageTimes(50);
		const large = estimateStageTimes(4000);
		expect(large.layout).toBeGreaterThan(small.layout);
		expect(large.draw).toBeGreaterThan(small.draw);
		expect(large.data).toBe(small.data);
	});
});

describe('a load', () => {
	beforeEach(() => endLoad());

	it('moves forward through its stages and clears when it ends', () => {
		beginLoad('scope', ['measure', 'layout', 'layoutRefine', 'draw'], 1000);
		const seen = [get(loadProgress)!];
		for (const stage of ['layout', 'layoutRefine', 'draw'] as const) {
			seen.push(enterLoadStage(stage)!);
		}

		expect(seen.map((progress) => progress.stage)).toEqual([
			'measure',
			'layout',
			'layoutRefine',
			'draw'
		]);
		for (let i = 1; i < seen.length; i++) expect(seen[i].from).toBeCloseTo(seen[i - 1].to);
		expect(seen[seen.length - 1].to).toBeCloseTo(1);

		endLoad();
		expect(get(loadProgress)).toBeNull();
	});

	it('passes over the stages a run skips', () => {
		// A stored layout runs neither layout pass.
		beginLoad('scope', ['measure', 'layout', 'layoutRefine', 'draw'], 1000);
		const measure = get(loadProgress)!;
		const draw = enterLoadStage('draw')!;

		expect(draw.from).toBeGreaterThan(measure.to);
		expect(draw.to).toBeCloseTo(1);
	});

	it('continues from the download when the pipeline takes over the same scope', () => {
		const download = beginLoad('scope', LOAD_STAGES)!;
		const pipeline = beginLoad('scope', ['measure', 'layout', 'layoutRefine', 'draw'], 4000)!;

		expect(pipeline.stage).toBe('measure');
		expect(pipeline.from).toBeCloseTo(download.to);
	});

	it('keeps the download where it is when only the scope becomes known', () => {
		const first = beginLoad('', LOAD_STAGES)!;
		const second = beginLoad('topology|L2Physical', LOAD_STAGES)!;

		expect(second).toBe(first);
	});

	it('clears a failed download and leaves a run past the download alone', () => {
		beginLoad('scope', LOAD_STAGES);
		cancelDownload();
		expect(get(loadProgress)).toBeNull();

		beginLoad('scope', ['layout', 'draw'], 100);
		cancelDownload();
		expect(get(loadProgress)?.stage).toBe('layout');
		endLoad();
	});

	it('keeps a download up when a run for another view ends, and ends with its own run', () => {
		beginLoad('topology|Workloads', LOAD_STAGES);

		endLoadOfRun('topology|L3Logical');
		expect(get(loadProgress)?.stage).toBe('data');

		endLoadOfRun('topology|Workloads');
		expect(get(loadProgress)).toBeNull();
	});

	it('starts a later run from zero', () => {
		beginLoad('scope', ['draw'], 100);
		endLoad();
		const next = beginLoad('scope', ['measure', 'layout', 'layoutRefine', 'draw'], 100)!;

		expect(next.from).toBe(0);
	});

	it('shares the interrupted stage when a run starts over mid-load', () => {
		// The corrective re-layout after a first draw.
		beginLoad('scope', ['measure', 'layout', 'layoutRefine', 'draw'], 1000);
		const draw = enterLoadStage('draw')!;
		const again = beginLoad('scope', ['measure', 'layout', 'layoutRefine', 'draw'], 1000)!;

		expect(again.from).toBeCloseTo(draw.from);
		expect(enterLoadStage('draw')!.to).toBeCloseTo(1);
	});
});
