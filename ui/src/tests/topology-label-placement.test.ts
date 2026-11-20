import { describe, it, expect } from 'vitest';
import {
	LABEL_GAP,
	LABEL_HEIGHT,
	MAX_LABELS,
	placeLabels,
	type LabelCandidate,
	type LabelViewport
} from '#lib/features/topology/pipeline/label-placement.ts';

/** Every name in these tests is 80px wide on screen unless a test says otherwise. */
const NAME_WIDTH = 80;

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
	textWidth: NAME_WIDTH,
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

const ids = (labels: { id: string }[]) => labels.map((label) => label.id);

/**
 * A name starts on its own box and runs to its full width, cut only where another box or name
 * begins. The layer cuts the text to `maxWidth`; what is decided here is which boxes get a name
 * and how wide it may be.
 */
describe('placeLabels', () => {
	it('draws a name in full on a box wide enough for it', () => {
		const name = 'Plant Engineering: 10.40.45.0/24';
		const [label] = placeLabels(
			[box('subnet', 100, 50, 4000, 1500, { text: name, textWidth: 190 })],
			pane(0.1)
		);

		expect(label).toMatchObject({ id: 'subnet', text: name, x: 100, y: 50, maxWidth: 190 });
	});

	it('runs a name past its box onto empty canvas, to its full width', () => {
		// 60px on screen with nothing beside it, for a name of 120px.
		const [label] = placeLabels([box('dist-sw-01', 0, 0, 600, 400, { textWidth: 120 })], pane(0.1));

		expect(label.maxWidth).toBe(120);
	});

	it('cuts a name where the next box to its right begins', () => {
		// 60px wide, with a neighbour starting 65px from its left edge.
		const [, label] = placeLabels(
			[box('subnet', 650, 0, 4000, 1500), box('bridge', 0, 0, 600, 400, { textWidth: 190 })],
			pane(0.1)
		);

		expect(label).toMatchObject({ id: 'bridge', maxWidth: 65 - LABEL_GAP });
	});

	it('names a box too small for a name when the canvas beside it is empty', () => {
		// 10×9px on screen, alone: Docker Bridge beside the large subnets of an expanded L3.
		const placed = placeLabels(
			[box('subnet', 0, 0, 4000, 1500), box('bridge', 6000, 0, 100, 90)],
			pane(0.1)
		);

		expect(ids(placed)).toEqual(['subnet', 'bridge']);
		expect(placed[1].maxWidth).toBe(NAME_WIDTH);
	});

	it('leaves a small box bare when a few characters would not fit beside it', () => {
		// 10px wide with a neighbour 30px to its right.
		const boxes = [box('subnet', 300, 0, 4000, 1500), box('bridge', 0, 0, 100, 90)];

		expect(ids(placeLabels(boxes, pane(0.1)))).toEqual(['subnet']);
	});

	it('names each small box that has room, though its neighbour has none', () => {
		// Two switches 33px wide, 5px apart. A name on the first would cover the second, so the
		// first is bare; the second has empty canvas beside it and is named in full.
		const packed = [box('acc-sw-058', 0, 0, 330, 430), box('acc-sw-059', 380, 0, 330, 430)];
		const subnet = box('subnet', 0, 2000, 4000, 1500);

		const placed = placeLabels([...packed, subnet], pane(0.1));
		expect(ids(placed).sort()).toEqual(['acc-sw-059', 'subnet']);
		expect(placed.find((label) => label.id === 'acc-sw-059')!.maxWidth).toBe(NAME_WIDTH);
	});

	it('keeps a later name off one already drawn past its box', () => {
		// The second box sits up and to the right of the first. The first, larger box's name runs
		// over the canvas where the second's name would start, so the second is bare.
		const placed = placeLabels(
			[box('first', 0, 100, 200, 100), box('second', 300, 0, 100, 50)],
			pane(0.1)
		);

		expect(placed).toMatchObject([{ id: 'first', maxWidth: NAME_WIDTH }]);
	});

	it('keeps a small box’s name off a box with no name', () => {
		const boxes = [box('bridge', 0, 0, 100, 90), box('unnamed', 200, 0, 2000, 2000, { text: '' })];

		expect(placeLabels(boxes, pane(0.1))).toHaveLength(0);
	});

	it('runs a name under a box shorter than a label onto empty canvas', () => {
		const flat = box('flat', 0, 0, 2000, (LABEL_HEIGHT - 1) / 0.1);

		expect(placeLabels([flat], pane(0.1))).toMatchObject([{ id: 'flat', maxWidth: NAME_WIDTH }]);
	});

	it('skips boxes that are off the pane, and boxes with no name', () => {
		const boxes = [
			box('seen', 100, 100, 2000, 2000),
			box('away', 50_000, 100, 2000, 2000),
			box('unnamed', 3000, 100, 2000, 2000, { text: '' })
		];
		expect(ids(placeLabels(boxes, pane(0.1)))).toEqual(['seen']);
	});

	it('names every box in full while searching, whatever its size and whatever its name covers', () => {
		// Two switches 33px wide, 5px apart, and a box squeezed beside a neighbour.
		const hits = [
			box('acc-sw-058', 0, 0, 330, 430),
			box('acc-sw-059', 380, 0, 330, 430),
			box('bridge', 0, 1000, 100, 90),
			box('subnet', 300, 1000, 4000, 1500)
		];

		expect(ids(placeLabels(hits, pane(0.1)))).not.toContain('acc-sw-058');
		const placed = placeLabels(hits, pane(0.1), true);
		expect(ids(placed).sort()).toEqual(['acc-sw-058', 'acc-sw-059', 'bridge', 'subnet']);
		expect(placed.every((label) => label.maxWidth === NAME_WIDTH)).toBe(true);
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
