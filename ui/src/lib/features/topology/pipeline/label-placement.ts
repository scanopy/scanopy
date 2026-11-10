/**
 * Which container names to draw over a zoomed-out canvas, and where.
 *
 * Below the zoom where a header can be read (`DETAIL_ZOOM` in `render-mode.ts`) a container's
 * name is either a few pixels tall or, at reduced detail, not mounted at all: thousands of headers
 * nobody can read cost a great deal and say nothing. That leaves a zoomed-out map with no names
 * on it. The names drawn here go on one layer above every node, at a fixed size on screen, for as
 * many containers as there is room for:
 *
 * - **on the box**, starting at its top-left corner, when the name covers no other container and
 *   no other name. It may run past the box's own right edge into empty canvas. A layer above the
 *   nodes is not covered by the container's children, which is what rules this out for the
 *   container's own header;
 * - **landmark**, above the box's top-left corner, for a sparse sample of the containers whose
 *   name has no such room (a dense grid of small boxes), spaced so they do not crowd each other.
 *
 * Pure geometry over rectangles the caller already has. It reads no DOM.
 */

export interface LabelCandidate {
	id: string;
	text: string;
	/** Colour of the container's type, as a CSS colour. */
	color: string;
	/** The container's box in flow coordinates. */
	x: number;
	y: number;
	width: number;
	height: number;
	/** Higher is placed first when there is not room for every name. */
	priority: number;
}

export interface PlacedLabel {
	id: string;
	text: string;
	color: string;
	/** Anchor in flow coordinates: the top-left corner of the container's box. */
	x: number;
	y: number;
	placement: 'onBox' | 'landmark';
}

export interface LabelViewport {
	/** Flow coordinate at the left and top edge of the pane. */
	x: number;
	y: number;
	/** Pane size in screen pixels. */
	width: number;
	height: number;
	zoom: number;
}

/** Label metrics on screen, in pixels. They match `ContainerLabelLayer.svelte`. */
export const LABEL_HEIGHT = 16;
const CHAR_WIDTH = 6.2;
const LABEL_PADDING = 14;
export const LABEL_MAX_CHARS = 28;

/**
 * Clear space a landmark keeps from every other name, in screen pixels. The first is used unless
 * it yields more than `MAX_LANDMARKS`, then the next.
 */
const LANDMARK_CLEARANCES = [24, 40, 64, 96, 144, 216];
export const MAX_LANDMARKS = 60;
/** More names than this on one screen is a wall of text, whatever the room. */
export const MAX_ON_BOX = 300;

export function labelText(text: string): string {
	return text.length > LABEL_MAX_CHARS ? `${text.slice(0, LABEL_MAX_CHARS - 1)}…` : text;
}

/** Width on screen of a label showing `text`, estimated from its length. */
export function labelWidth(text: string): number {
	return labelText(text).length * CHAR_WIDTH + LABEL_PADDING;
}

interface ScreenRect {
	left: number;
	top: number;
	right: number;
	bottom: number;
}

const overlaps = (a: ScreenRect, b: ScreenRect, clearance = 0): boolean =>
	a.left < b.right + clearance &&
	b.left < a.right + clearance &&
	a.top < b.bottom + clearance &&
	b.top < a.bottom + clearance;

/** Rectangles bucketed by screen cell, so a lookup tests neighbours and not every rectangle. */
class RectIndex {
	private cells = new Map<number, { rect: ScreenRect; owner: string }[]>();
	private static readonly CELL = 96;

	private *keys(rect: ScreenRect, clearance: number): Generator<number> {
		const cell = RectIndex.CELL;
		const fromX = Math.floor((rect.left - clearance) / cell);
		const toX = Math.floor((rect.right + clearance) / cell);
		const fromY = Math.floor((rect.top - clearance) / cell);
		const toY = Math.floor((rect.bottom + clearance) / cell);
		for (let x = fromX; x <= toX; x++) {
			for (let y = fromY; y <= toY; y++) yield x * 65536 + y;
		}
	}

	add(rect: ScreenRect, owner: string): void {
		for (const key of this.keys(rect, 0)) {
			const bucket = this.cells.get(key);
			if (bucket) bucket.push({ rect, owner });
			else this.cells.set(key, [{ rect, owner }]);
		}
	}

	/** Whether `rect` comes within `clearance` of a rectangle owned by anything but `owner`. */
	hits(rect: ScreenRect, owner: string, clearance = 0): boolean {
		for (const key of this.keys(rect, clearance)) {
			for (const entry of this.cells.get(key) ?? []) {
				if (entry.owner !== owner && overlaps(rect, entry.rect, clearance)) return true;
			}
		}
		return false;
	}
}

export function placeLabels(
	candidates: LabelCandidate[],
	viewport: LabelViewport,
	options: { landmarks: boolean }
): PlacedLabel[] {
	const { zoom } = viewport;
	if (zoom <= 0) return [];

	// Every container on the pane, as a screen rectangle.
	const boxes = new RectIndex();
	const inView: { candidate: LabelCandidate; box: ScreenRect }[] = [];
	for (const candidate of candidates) {
		const left = (candidate.x - viewport.x) * zoom;
		const top = (candidate.y - viewport.y) * zoom;
		const box = {
			left,
			top,
			right: left + candidate.width * zoom,
			bottom: top + candidate.height * zoom
		};
		if (box.right < 0 || box.bottom < 0 || box.left > viewport.width || box.top > viewport.height) {
			continue;
		}
		boxes.add(box, candidate.id);
		if (candidate.text) inView.push({ candidate, box });
	}

	inView.sort(
		(a, b) =>
			b.candidate.priority - a.candidate.priority ||
			a.candidate.text.localeCompare(b.candidate.text)
	);

	const placed: PlacedLabel[] = [];
	const names = new RectIndex();
	const noRoom: typeof inView = [];

	let onBox = 0;
	for (const entry of inView) {
		const { candidate, box } = entry;
		const rect: ScreenRect = {
			left: box.left,
			top: box.top,
			right: box.left + labelWidth(candidate.text),
			bottom: box.top + LABEL_HEIGHT
		};
		const fits =
			onBox < MAX_ON_BOX && !boxes.hits(rect, candidate.id) && !names.hits(rect, candidate.id);
		if (!fits) {
			noRoom.push(entry);
			continue;
		}
		onBox++;
		names.add(rect, candidate.id);
		placed.push({ ...label(candidate), placement: 'onBox' });
	}

	if (!options.landmarks) return placed;

	// Spread the landmarks over the whole pane. Stopping at the limit would spend it all on the
	// first rows, so the spacing widens until the sample fits the limit.
	for (const clearance of LANDMARK_CLEARANCES) {
		const landmarks = placeLandmarks(noRoom, names, viewport, clearance);
		if (landmarks.length <= MAX_LANDMARKS || clearance === LANDMARK_CLEARANCES.at(-1)) {
			return [...placed, ...landmarks.slice(0, MAX_LANDMARKS)];
		}
	}
	return placed;
}

function placeLandmarks(
	noRoom: { candidate: LabelCandidate; box: ScreenRect }[],
	onBoxNames: RectIndex,
	viewport: LabelViewport,
	clearance: number
): PlacedLabel[] {
	const landmarks: PlacedLabel[] = [];
	const taken = new RectIndex();
	for (const { candidate, box } of noRoom) {
		// Sits above the box, its left edge on the box's left edge.
		const rect: ScreenRect = {
			left: box.left,
			top: box.top - LABEL_HEIGHT,
			right: box.left + labelWidth(candidate.text),
			bottom: box.top
		};
		if (rect.top < 0 || rect.left < 0 || rect.right > viewport.width) continue;
		if (onBoxNames.hits(rect, candidate.id, LANDMARK_CLEARANCES[0])) continue;
		if (taken.hits(rect, candidate.id, clearance)) continue;
		taken.add(rect, candidate.id);
		landmarks.push({ ...label(candidate), placement: 'landmark' });
	}
	return landmarks;
}

function label(candidate: LabelCandidate): Omit<PlacedLabel, 'placement'> {
	return {
		id: candidate.id,
		text: labelText(candidate.text),
		color: candidate.color,
		x: candidate.x,
		y: candidate.y
	};
}
