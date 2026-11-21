import { describe, it, expect } from 'vitest';
import {
	LABEL_GAP,
	LABEL_HEIGHT,
	MAX_LABELS,
	MIN_STEP_ZOOM,
	ZOOM_STEP_RATIO,
	nameLevels,
	nameMatches,
	onPane,
	zoomStep,
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
	level: 'Host',
	priority: width * height,
	...extra
});

/** A 1000×800 pane showing flow coordinates from (`x`, `y`) at `zoom`. */
const pane = (zoom: number, x = 0, y = 0): LabelViewport => ({
	x,
	y,
	width: 1000,
	height: 800,
	zoom
});

const ids = (labels: { id: string }[]) => labels.map((label) => label.id).sort();

/**
 * A row of `count` switches 330 wide and 430 tall, `pitch` apart, starting at `y`. At a zoom of
 * 0.1 each is 33px wide, too small to hold its 80px name, so each name runs onto the gap.
 */
const row = (count: number, pitch: number, y = 0, prefix = 'acc-sw-') =>
	Array.from({ length: count }, (_, i) =>
		box(`${prefix}${String(i).padStart(3, '0')}`, i * pitch, y, 330, 430)
	);

/**
 * The root containers of one type are a level, named whole or not at all, decided over the whole
 * graph and not the pane. The layer cuts the text to `maxWidth`.
 */
describe('nameLevels', () => {
	it('names every box of a level when each has room for its name', () => {
		// 1000 apart: 100px at 0.1, room for an 80px name and the gap.
		const switches = row(5, 1000);

		const placed = nameLevels(switches, 0.1);

		expect(ids(placed)).toEqual(ids(switches));
		expect(placed.every((label) => label.maxWidth === NAME_WIDTH)).toBe(true);
	});

	it('names none of a level when one of its boxes has no room', () => {
		// The same row, but the last switch sits 50px past the one before it: no room for that one's
		// name, so the whole row goes unnamed rather than all but one.
		const switches = row(5, 1000);
		switches.push(box('acc-sw-crowded', 4500, 0, 330, 430));

		expect(nameLevels(switches, 0.1)).toEqual([]);
	});

	it('names a level once the zoom gives its most crowded box room', () => {
		const switches = [...row(5, 1000), box('acc-sw-crowded', 4500, 0, 330, 430)];

		// 500 apart is 80px at 0.16, enough for the crowded box's name.
		expect(ids(nameLevels(switches, 0.16))).toEqual(ids(switches));
	});

	it('names the same boxes wherever the pane is, at one zoom', () => {
		// A long row with room, beside a level that is too crowded to name.
		const hosts = row(40, 1000);
		const crowded = row(10, 400, 2000, 'ap-');
		const all = [...hosts, ...crowded].map((candidate, i) =>
			i >= hosts.length ? { ...candidate, level: 'Subnet' } : candidate
		);
		const byId = new Map(all.map((candidate) => [candidate.id, candidate]));
		const named = nameLevels(all, 0.1);

		const left = new Set(ids(onPane(named, byId, pane(0.1, 0, 0))));
		const right = new Set(ids(onPane(named, byId, pane(0.1, 6000, 0))));
		const seenByBoth = all.filter(
			(candidate) =>
				onPane([{ ...candidate, maxWidth: 1 }], byId, pane(0.1, 0, 0)).length > 0 &&
				onPane([{ ...candidate, maxWidth: 1 }], byId, pane(0.1, 6000, 0)).length > 0
		);

		expect(seenByBoth.length).toBeGreaterThan(0);
		for (const candidate of seenByBoth) {
			expect(left.has(candidate.id)).toBe(right.has(candidate.id));
		}
		expect(named.some((label) => label.id.startsWith('ap-'))).toBe(false);
	});

	it('names a larger level first, and keeps a smaller level off its names', () => {
		// A 10px host whose name runs right, and a larger subnet just above that row, 40px along:
		// the subnet's 200px name crosses the row the host's name needs.
		const subnet = box('subnet', 400, -100, 300, 100, { level: 'Subnet', textWidth: 200 });
		const host = box('host', 0, 0, 100, 100, { level: 'Host' });

		expect(ids(nameLevels([subnet, host], 0.1))).toEqual(['subnet']);
		// Without the subnet, the host has the room.
		expect(ids(nameLevels([host], 0.1))).toEqual(['host']);
	});

	it('cuts a name to a box wide enough to hold one, and no shorter than the box', () => {
		// 60px wide, with a neighbour starting 65px from its left edge.
		const bridge = box('bridge', 0, 0, 600, 400, { textWidth: 190 });
		const subnet = box('subnet', 650, 0, 4000, 1500);
		const placed = nameLevels([bridge, subnet], 0.1);

		expect(placed.find((label) => label.id === 'bridge')!.maxWidth).toBe(65 - LABEL_GAP);

		const tight = box('subnet', 610, 0, 4000, 1500);
		expect(nameLevels([bridge, tight], 0.1).find((label) => label.id === 'bridge')!.maxWidth).toBe(
			60
		);
	});

	it('keeps names off a box with no name, and names a box shorter than a label', () => {
		const unnamed = box('unnamed', 200, 0, 2000, 2000, { text: '' });
		const flat = box('flat', 0, 5000, 2000, (LABEL_HEIGHT - 1) / 0.1);

		expect(nameLevels([box('bridge', 0, 0, 100, 90), unnamed], 0.1)).toEqual([]);
		expect(nameLevels([flat, unnamed], 0.1)).toMatchObject([{ id: 'flat', maxWidth: NAME_WIDTH }]);
	});

	it('keeps names that fit at a step fitting anywhere up to the next step', () => {
		const switches = [...row(5, 1000), box('acc-sw-crowded', 4500, 0, 330, 430)];
		// The lowest step where the crowded box has room.
		let step = MIN_STEP_ZOOM;
		while (nameLevels(switches, step).length === 0) step *= ZOOM_STEP_RATIO;

		for (const zoom of [step, step * 1.03, step * 1.06, step * 1.09]) {
			expect(zoomStep(zoom)).toBeCloseTo(step, 12);
			const named = nameLevels(switches, zoom);
			expect(named).toHaveLength(switches.length);
			// The widths placed at the step still fit at the higher zoom.
			const atStep = new Map(nameLevels(switches, step).map((label) => [label.id, label]));
			for (const label of named) {
				expect(label.maxWidth).toBeGreaterThanOrEqual(atStep.get(label.id)!.maxWidth);
			}
		}
		expect(zoomStep(MIN_STEP_ZOOM / 2)).toBe(0);
		expect(nameLevels(switches, 0)).toEqual([]);
	});
});

describe('onPane', () => {
	it('keeps the labels whose box shows on the pane', () => {
		const boxes = [box('seen', 100, 100, 2000, 2000), box('away', 50_000, 100, 2000, 2000)];
		const byId = new Map(boxes.map((candidate) => [candidate.id, candidate]));

		expect(ids(onPane(nameLevels(boxes, 0.1), byId, pane(0.1)))).toEqual(['seen']);
	});
});

describe('nameMatches', () => {
	it('names every match in full, whatever its size and whatever its name covers', () => {
		const hits = [
			box('acc-sw-058', 0, 0, 330, 430),
			box('acc-sw-059', 380, 0, 330, 430),
			box('bridge', 0, 1000, 100, 90),
			box('subnet', 300, 1000, 4000, 1500),
			box('away', 50_000, 0, 330, 430)
		];

		const placed = nameMatches(hits, pane(0.1));

		expect(ids(placed)).toEqual(['acc-sw-058', 'acc-sw-059', 'bridge', 'subnet']);
		expect(placed.every((label) => label.maxWidth === NAME_WIDTH)).toBe(true);
	});

	it('keeps the highest priority when there are more matches than the limit', () => {
		const many = Array.from({ length: MAX_LABELS + 20 }, (_, i) =>
			box(`host-${String(i).padStart(4, '0')}`, (i % 40) * 20, Math.floor(i / 40) * 20, 600, 200)
		);
		const wanted = many[MAX_LABELS + 10];
		const withPriority = many.map((candidate) =>
			candidate === wanted ? { ...candidate, priority: 1e12 } : candidate
		);

		const placed = nameMatches(withPriority, pane(0.1));

		expect(placed).toHaveLength(MAX_LABELS);
		expect(placed[0].id).toBe(wanted.id);
	});
});
