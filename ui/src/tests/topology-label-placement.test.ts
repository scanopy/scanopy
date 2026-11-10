import { describe, it, expect } from 'vitest';
import {
	LABEL_MAX_CHARS,
	MAX_LANDMARKS,
	labelText,
	labelWidth,
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

const placedIds = (candidates: LabelCandidate[], zoom: number, landmarks = false) =>
	placeLabels(candidates, pane(zoom), { landmarks }).map((label) => `${label.id}:${label.placement}`);

describe('placeLabels', () => {
	it('names a box whose name covers nothing else, even past its own right edge', () => {
		// 50px wide on screen, a name of about 76px, and empty canvas to its right.
		expect(placedIds([box('acc-sw-058', 0, 0, 500, 600)], 0.1)).toEqual(['acc-sw-058:onBox']);
	});

	it('leaves out a name that would cover the next box', () => {
		// Two boxes 10px apart on screen: the left name would run over the right box.
		const boxes = [box('left-host', 0, 0, 400, 200), box('right-host', 500, 0, 400, 200)];
		expect(placedIds(boxes, 0.1)).toEqual(['right-host:onBox']);
	});

	it('names every box once each is larger than its name', () => {
		const boxes = [box('left-host', 0, 0, 400, 200), box('right-host', 500, 0, 400, 200)];
		expect(placedIds(boxes, 0.3).sort()).toEqual(['left-host:onBox', 'right-host:onBox']);
	});

	it('skips boxes that are off the pane', () => {
		const boxes = [box('seen', 100, 100, 2000, 2000), box('away', 50_000, 100, 2000, 2000)];
		expect(placedIds(boxes, 0.1)).toEqual(['seen:onBox']);
	});

	describe('over a dense grid', () => {
		// 40 columns × 30 rows of small boxes with no room between them for a name.
		const grid: LabelCandidate[] = [];
		for (let row = 0; row < 30; row++) {
			for (let column = 0; column < 40; column++) {
				grid.push(
					box(`host-${String(row * 40 + column).padStart(4, '0')}`, column * 240, row * 200, 200, 80)
				);
			}
		}

		it('names almost nothing without landmarks', () => {
			// Only boxes at the right-hand end of a row have empty canvas beside them.
			const placed = placeLabels(grid, pane(0.1), { landmarks: false });
			expect(placed.length).toBeLessThanOrEqual(30);
			expect(placed.every((label) => label.placement === 'onBox')).toBe(true);
		});

		it('adds a sparse sample with landmarks, spread over the pane and within the limit', () => {
			const placed = placeLabels(grid, pane(0.1), { landmarks: true });
			const landmarks = placed.filter((label) => label.placement === 'landmark');

			expect(landmarks.length).toBeGreaterThan(10);
			expect(landmarks.length).toBeLessThanOrEqual(MAX_LANDMARKS);
			// Not all from the first rows: the sample reaches the lower half of the pane.
			const lowest = Math.max(...landmarks.map((label) => label.y * 0.1));
			expect(lowest).toBeGreaterThan(300);
		});

		it('makes a landmark of the highest priority first, such as a search match', () => {
			const wanted = 'host-0617';
			const withMatch = grid.map((candidate) =>
				candidate.id === wanted ? { ...candidate, priority: 1e12 } : candidate
			);
			const before = placeLabels(grid, pane(0.1), { landmarks: true }).map((label) => label.id);
			const after = placeLabels(withMatch, pane(0.1), { landmarks: true }).map((label) => label.id);

			expect(before).not.toContain(wanted);
			expect(after).toContain(wanted);
		});

		it('keeps landmarks clear of each other', () => {
			const landmarks = placeLabels(grid, pane(0.1), { landmarks: true }).filter(
				(label) => label.placement === 'landmark'
			);
			for (let i = 0; i < landmarks.length; i++) {
				for (let j = i + 1; j < landmarks.length; j++) {
					const a = landmarks[i];
					const b = landmarks[j];
					const apartX =
						Math.abs(a.x - b.x) * 0.1 >= Math.min(labelWidth(a.text), labelWidth(b.text)) + 24;
					const apartY = Math.abs(a.y - b.y) * 0.1 >= 16 + 24;
					expect(apartX || apartY, `${a.id} / ${b.id}`).toBe(true);
				}
			}
		});
	});
});

describe('labelText', () => {
	it('shortens a long name and leaves a short one alone', () => {
		expect(labelText('acc-sw-058')).toBe('acc-sw-058');
		const long = labelText('Plant Wireless Infrastructure: 10.40.26.0/23');
		expect(long.length).toBe(LABEL_MAX_CHARS);
		expect(long.endsWith('…')).toBe(true);
	});
});
