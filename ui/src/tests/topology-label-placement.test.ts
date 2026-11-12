import { describe, it, expect } from 'vitest';
import {
	LABEL_HEIGHT,
	MAX_LABELS,
	MIN_LABEL_WIDTH,
	ROOMY_LABEL_WIDTH,
	placeLabels,
	type LabelCandidate,
	type LabelViewport
} from '#lib/features/topology/pipeline/label-placement.ts';

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
		// 60px on screen for a name of about 190px, with a neighbour 5px to its right: the name
		// stays on the box, capped at the box.
		const [, label] = placeLabels(
			[
				box('subnet', 650, 0, 4000, 1500),
				box('bridge', 0, 0, 600, 400, { text: 'Docker Bridge: (172.17.0.0/16)' })
			],
			pane(0.1)
		);

		expect(label).toMatchObject({ id: 'bridge', maxWidth: 60 });
	});

	it('lets a name on a narrow box run onto empty canvas beside it', () => {
		const [label] = placeLabels([box('bridge', 0, 0, 600, 400)], pane(0.1));

		expect(label.maxWidth).toBe(ROOMY_LABEL_WIDTH);
	});

	it('names a box too small for a name when the canvas beside it is empty', () => {
		// 10×9px on screen, alone: Docker Bridge beside the large subnets of an expanded L3.
		const subnet = box('subnet', 0, 0, 4000, 1500);
		const bridge = box('bridge', 6000, 0, 100, 90);

		const placed = placeLabels([subnet, bridge], pane(0.1));

		expect(placed.map((label) => label.id)).toEqual(['subnet', 'bridge']);
		expect(placed[1].maxWidth).toBe(ROOMY_LABEL_WIDTH);
	});

	it('cuts a small box’s name shorter when only that much canvas is empty', () => {
		// 10px wide with a neighbour 60px to its right: no room for the wide name, room for the short.
		const boxes = [box('subnet', 700, 0, 4000, 1500), box('bridge', 0, 0, 100, 90)];

		expect(placeLabels(boxes, pane(0.1))[1]).toMatchObject({
			id: 'bridge',
			maxWidth: MIN_LABEL_WIDTH
		});
	});

	it('names a box shorter than a label when the canvas under it is empty', () => {
		const flat = box('flat', 0, 0, 2000, (LABEL_HEIGHT - 1) / 0.1);

		expect(placeLabels([flat], pane(0.1))).toMatchObject([{ id: 'flat', maxWidth: 200 }]);
	});

	it('names no small box when one of them has no room, the rim of a packed row included', () => {
		// Two switches 33px wide, 5px apart. A name on the first would cover the second; the second
		// has empty canvas beside it and stays bare with the first.
		const packed = [box('acc-sw-058', 0, 0, 330, 430), box('acc-sw-059', 380, 0, 330, 430)];
		const subnet = box('subnet', 0, 2000, 4000, 1500);

		expect(placeLabels([...packed, subnet], pane(0.1)).map((label) => label.id)).toEqual([
			'subnet'
		]);
		// With room for a name on each box, all are named.
		expect(placeLabels([...packed, subnet], pane(0.2))).toHaveLength(3);
	});

	it('treats two small boxes whose names would cross as having no room', () => {
		// The second sits up and to the right of the first. Each name clears the other box, and the
		// two names would cross.
		const pair = [box('first', 0, 100, 200, 100), box('second', 300, 0, 100, 50)];

		expect(placeLabels(pair, pane(0.1))).toHaveLength(0);
	});

	it('keeps a small box’s name off a box with no name', () => {
		const boxes = [box('bridge', 0, 0, 100, 90), box('unnamed', 200, 0, 2000, 2000, { text: '' })];

		expect(placeLabels(boxes, pane(0.1))).toHaveLength(0);
	});

	it('skips boxes that are off the pane, and boxes with no name', () => {
		const boxes = [
			box('seen', 100, 100, 2000, 2000),
			box('away', 50_000, 100, 2000, 2000),
			box('unnamed', 3000, 100, 2000, 2000, { text: '' })
		];
		expect(placeLabels(boxes, pane(0.1)).map((label) => label.id)).toEqual(['seen']);
	});

	it('names every box while searching, whatever its size and whatever its name covers', () => {
		// Two switches 33px wide, 5px apart, and a 10×9px box: none is named without a search.
		const hits = [
			box('acc-sw-058', 0, 0, 330, 430),
			box('acc-sw-059', 380, 0, 330, 430),
			box('bridge', 0, 1000, 100, 90)
		];

		expect(placeLabels(hits, pane(0.1))).toHaveLength(0);
		const placed = placeLabels(hits, pane(0.1), true);
		expect(placed.map((label) => label.id).sort()).toEqual(['acc-sw-058', 'acc-sw-059', 'bridge']);
		expect(placed.every((label) => label.maxWidth === ROOMY_LABEL_WIDTH)).toBe(true);
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
