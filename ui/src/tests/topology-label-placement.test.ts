import { describe, it, expect } from 'vitest';
import {
	LABEL_HEIGHT,
	MAX_LABELS,
	MIN_LABEL_WIDTH,
	placeLabels,
	type LabelCandidate,
	type LabelViewport
} from '$lib/features/topology/pipeline/label-placement';

const box = (
	id: string,
	x: number,
	y: number,
	width: number,
	height: number,
	extra: Partial<LabelCandidate> = {}
): LabelCandidate => ({
	id,
	text: id,
	color: '#000',
	x,
	y,
	width,
	height,
	priority: width * height,
	...extra
});

/** A 1000×800 pane showing flow coordinates from the origin at `zoom`. */
const pane = (zoom: number): LabelViewport => ({ x: 0, y: 0, width: 1000, height: 800, zoom });

/**
 * A name sits on its own box and is never wider than it. The layer cuts the text to `maxWidth`;
 * what is decided here is which boxes get a name and how wide it may be.
 */
describe('placeLabels', () => {
	it('gives a name the width of its box on screen, and the whole text', () => {
		const name = 'Plant Engineering: 10.40.45.0/24';
		const [label] = placeLabels([box('subnet', 100, 50, 4000, 1500, { text: name })], pane(0.1));

		expect(label).toMatchObject({ id: 'subnet', text: name, x: 100, y: 50, maxWidth: 400 });
	});

	it('keeps a long name on a box too narrow for it, to be cut by the layer', () => {
		// 60px on screen for a name of about 190px: it stays on the box, capped at the box.
		const [label] = placeLabels(
			[box('bridge', 0, 0, 600, 400, { text: 'Docker Bridge: (172.17.0.0/16)' })],
			pane(0.1)
		);

		expect(label.maxWidth).toBe(60);
	});

	it('leaves out a box narrower than a few characters, or shorter than a label', () => {
		const narrow = box('narrow', 0, 0, (MIN_LABEL_WIDTH - 1) / 0.1, 2000);
		const short = box('short', 3000, 0, 2000, (LABEL_HEIGHT - 1) / 0.1);
		const enough = box('enough', 6000, 0, MIN_LABEL_WIDTH / 0.1, LABEL_HEIGHT / 0.1);

		expect(placeLabels([narrow, short, enough], pane(0.1)).map((label) => label.id)).toEqual([
			'enough'
		]);
	});

	it('names more boxes as the zoom rises', () => {
		const switches = [box('acc-sw-058', 0, 0, 330, 430), box('acc-sw-059', 0, 600, 330, 430)];

		expect(placeLabels(switches, pane(0.1))).toHaveLength(0);
		expect(placeLabels(switches, pane(0.2))).toHaveLength(2);
	});

	it('skips boxes that are off the pane, and boxes with no name', () => {
		const boxes = [
			box('seen', 100, 100, 2000, 2000),
			box('away', 50_000, 100, 2000, 2000),
			box('unnamed', 3000, 100, 2000, 2000, { text: '' })
		];
		expect(placeLabels(boxes, pane(0.1)).map((label) => label.id)).toEqual(['seen']);
	});

	it('keeps the highest priority when there are more names than the limit', () => {
		const many = Array.from({ length: MAX_LABELS + 20 }, (_, i) =>
			box(`host-${String(i).padStart(4, '0')}`, (i % 40) * 20, Math.floor(i / 40) * 20, 600, 200)
		);
		const wanted = many[MAX_LABELS + 10];
		const withMatch = many.map((candidate) =>
			candidate === wanted ? { ...candidate, priority: 1e12 } : candidate
		);

		const placed = placeLabels(withMatch, pane(0.1));

		expect(placed).toHaveLength(MAX_LABELS);
		expect(placed[0].id).toBe(wanted.id);
	});
});
