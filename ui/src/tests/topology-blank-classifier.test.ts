/**
 * The blank classifier decided a customer report and had no coverage at all.
 *
 * Its output is what latches the one-shot `firstBlank` capture, so a wrong answer does not just
 * mislead a reader — it spends the evidence. A capture came in whose only blank row was a frame
 * that had never been on screen, taken before the renderer had caught up with a viewport change,
 * which left the fault being reported absent from the file entirely.
 *
 * These pin the decision itself. Whether the caller samples at the right moment is a separate
 * question, answered by `sampleOnNextFrame` in the viewer.
 */
import { describe, it, expect } from 'vitest';
import { classifyBlank, type BlankInputs } from '#lib/features/topology/diagnostics.ts';

/** A frame with the graph on it. Each case below changes one thing. */
const healthy = (over: Partial<BlankInputs> = {}): BlankInputs => ({
	storeNodes: 1268,
	paneHidden: false,
	coldLoadMeasure: false,
	mountedCount: 438,
	intersectsPane: true,
	...over
});

describe('classifyBlank', () => {
	it('reports nothing for a frame with mounted nodes on the pane', () => {
		expect(classifyBlank(healthy())).toBeNull();
	});

	it('calls an empty store empty, not culled', () => {
		// The two lead to opposite places — `empty` is a pipeline or data fault and has nothing to
		// do with rendering, and a report that said `culled` here would send the next reader to
		// the renderer. With no nodes there is also nothing for culling to have hidden.
		expect(classifyBlank(healthy({ storeNodes: 0, mountedCount: 0 }))).toBe('empty');
	});

	it('prefers empty over a hidden pane', () => {
		// A cold load hides the pane *and* has not populated the store yet, so both conditions are
		// live on the same frame. The store is the more actionable of the two.
		expect(classifyBlank(healthy({ storeNodes: 0, mountedCount: 0, paneHidden: true }))).toBe(
			'empty'
		);
	});

	it('excuses a hidden pane while the cold load is measuring', () => {
		// That pass mounts every node at full size behind `visibility: hidden` to read heights. A
		// sample landing in the window sees an intentionally invisible canvas, which is the pass
		// working, not a fault.
		expect(classifyBlank(healthy({ paneHidden: true, coldLoadMeasure: true }))).toBeNull();
	});

	it('reports a pane still hidden with no cold load running', () => {
		// The cold load is the only thing that hides the pane, so one hidden outside that window
		// is stuck — and an operator describes it exactly like the others.
		expect(classifyBlank(healthy({ paneHidden: true }))).toBe('hidden');
	});

	it('reports a store with nothing mounted as culled', () => {
		expect(classifyBlank(healthy({ mountedCount: 0 }))).toBe('culled');
	});

	it('reports mounted nodes that miss the pane as culled', () => {
		// The viewport is somewhere the graph is not — a transform left over from the previous
		// view, which is the shape of the fault this whole diagnostic exists for.
		expect(classifyBlank(healthy({ intersectsPane: false }))).toBe('culled');
	});
});
