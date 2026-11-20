/**
 * Which container names to draw over a zoomed-out canvas, and how wide each may be.
 *
 * Below the zoom where a header can be read (`DETAIL_ZOOM` in `render-mode.ts`) a container's
 * name is either a few pixels tall or, at reduced detail, not mounted at all: thousands of headers
 * nobody can read cost a great deal and say nothing. The names drawn here go on one layer above
 * every node, at a fixed size on screen. A layer above the nodes is not covered by the container's
 * children, which is what rules this out for the container's own header.
 *
 * The rule: the root containers of one type (hosts in L2, subnets in L3) are a level, and a level
 * is named whole or not at all. At a given zoom, a level is named when every one of its boxes, over
 * the whole graph, has room for its name; otherwise none of it is. Which boxes are named depends on
 * the zoom alone, so a pan never changes it, and boxes of one type and size are always named alike.
 *
 * Each box was once named on its own, wherever the pane had room. On a ring or a column of
 * switches packed tighter than a name, that named whichever boxes had empty canvas to their right:
 * a scatter that told the user nothing and changed on every pan. The cost of naming a level whole
 * is that one crowded box leaves its level bare until the zoom gives that box room too.
 *
 * A name starts on its own box, at the top-left corner, and runs to its full width. A name over
 * another box reads as belonging to the box it covers, so it must stop short of any box or name
 * in its way. A box wide enough for a name may cut it to fit, down to the box's own width, and the
 * viewer shows it in full while the pointer is on it. A name too long for its box needs its whole
 * width of empty canvas: cut short, the names of a row of switches all read `acc-…`. Larger levels
 * claim their room first.
 *
 * Placement runs once per step of a fixed ladder of zooms (`zoomStep`), and the caller keeps the
 * result for the step. On screen, boxes and the gaps between them grow with the zoom while a name
 * keeps its size, so names that fit at a step still fit anywhere above it.
 *
 * Pure geometry over rectangles the caller already has. It reads no DOM.
 */

export interface LabelCandidate {
	id: string;
	text: string;
	/** The whole name's drawn width on screen, in pixels: text, padding and border. */
	textWidth: number;
	/** Colour of the container's type, as a CSS colour. */
	color: string;
	/** The container's box in flow coordinates. */
	x: number;
	y: number;
	width: number;
	height: number;
	/** Boxes of one level are named together or not at all: the container's type. */
	level: string;
	/** Higher is placed first within a level, and kept first among search matches. */
	priority: number;
}

export interface PlacedLabel {
	id: string;
	/** The whole name. The layer cuts it to `maxWidth` with an ellipsis. */
	text: string;
	color: string;
	/** Anchor in flow coordinates: the top-left corner of the container's box. */
	x: number;
	y: number;
	/** As wide as the name may be drawn, in screen pixels: the whole name, or up to whatever
	 * begins to its right. */
	maxWidth: number;
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

/** Height of a label on screen, in pixels. Matches `ContainerLabelLayer.svelte`. */
export const LABEL_HEIGHT = 16;

/** Narrowest box, on screen, that holds a name of its own and may cut it to its width. */
export const MIN_LABEL_WIDTH = 48;

/** Space kept between a cut name and the box or name it stops at, in screen pixels. */
export const LABEL_GAP = 4;

/**
 * More search matches than this on one screen is a wall of text. Level names need no cap: they
 * never overlap, so a pane holds only so many.
 */
export const MAX_LABELS = 300;

/** Lowest zoom on the ladder. Below it no level is named. */
export const MIN_STEP_ZOOM = 0.005;

/** Each step of the ladder is this much above the one below. */
export const ZOOM_STEP_RATIO = 1.1;

/** A rectangle in screen pixels. */
interface ScreenRect {
	left: number;
	top: number;
	right: number;
	bottom: number;
}

/** Side of a cell in the index of what is on the canvas, in screen pixels. */
const CELL = 64;

/**
 * What is on the canvas, by cell, so that asking whether a label's rectangle is free looks at the
 * few boxes near it and not at every box. A large site has thousands of boxes.
 */
class Occupied {
	private cells = new Map<number, ScreenRect[]>();

	private *keys(rect: ScreenRect): Generator<number> {
		const firstX = Math.floor(Math.max(rect.left, 0) / CELL);
		const lastX = Math.floor(Math.max(rect.right, 0) / CELL);
		const firstY = Math.floor(Math.max(rect.top, 0) / CELL);
		const lastY = Math.floor(Math.max(rect.bottom, 0) / CELL);
		for (let cx = firstX; cx <= lastX; cx++) {
			for (let cy = firstY; cy <= lastY; cy++) yield cx * 1_048_576 + cy;
		}
	}

	add(rect: ScreenRect) {
		for (const key of this.keys(rect)) {
			const cell = this.cells.get(key);
			if (cell) cell.push(rect);
			else this.cells.set(key, [rect]);
		}
	}

	/**
	 * How far right of `rect.left` a label may run within `rect`'s rows before it meets anything
	 * other than `own`, less `LABEL_GAP`. At most `rect`'s own width.
	 */
	clearRun(rect: ScreenRect, own: ScreenRect): number {
		let run = rect.right - rect.left;
		for (const key of this.keys(rect)) {
			for (const other of this.cells.get(key) ?? []) {
				if (other === own) continue;
				if (
					other.left < rect.right &&
					other.right > rect.left &&
					other.top < rect.bottom &&
					other.bottom > rect.top
				) {
					run = Math.min(run, Math.max(other.left, rect.left) - rect.left - LABEL_GAP);
				}
			}
		}
		return run;
	}
}

/**
 * The ladder step at or below `zoom`: `MIN_STEP_ZOOM` times a whole power of `ZOOM_STEP_RATIO`.
 * Zero below the ladder, where nothing is named.
 */
export function zoomStep(zoom: number): number {
	if (!(zoom >= MIN_STEP_ZOOM)) return 0;
	// The small term keeps a zoom sitting exactly on a step from rounding down to the one below.
	const index = Math.floor(Math.log(zoom / MIN_STEP_ZOOM) / Math.log(ZOOM_STEP_RATIO) + 1e-9);
	return MIN_STEP_ZOOM * ZOOM_STEP_RATIO ** index;
}

/**
 * The names drawn at `zoom`, over the whole graph: every box of each level whose boxes all have
 * room, and none of the others. Boxes with no name are not named, and names stay off them.
 */
export function nameLevels(candidates: LabelCandidate[], zoom: number): PlacedLabel[] {
	if (!(zoom > 0) || candidates.length === 0) return [];

	// Screen pixels measured from the graph's top-left corner, so every rectangle is on the index.
	const originX = Math.min(...candidates.map((candidate) => candidate.x));
	const originY = Math.min(...candidates.map((candidate) => candidate.y));
	const occupied = new Occupied();
	const rects = new Map<LabelCandidate, ScreenRect>();
	for (const candidate of candidates) {
		const left = (candidate.x - originX) * zoom;
		const top = (candidate.y - originY) * zoom;
		const rect = {
			left,
			top,
			right: left + candidate.width * zoom,
			bottom: top + candidate.height * zoom
		};
		rects.set(candidate, rect);
		occupied.add(rect);
	}

	const levels = new Map<string, LabelCandidate[]>();
	for (const candidate of candidates) {
		if (!candidate.text) continue;
		const members = levels.get(candidate.level);
		if (members) members.push(candidate);
		else levels.set(candidate.level, [candidate]);
	}
	const largestBox = (members: LabelCandidate[]) =>
		Math.max(...members.map((member) => member.width * member.height));
	const ordered = [...levels.entries()].sort(
		([aLevel, a], [bLevel, b]) => largestBox(b) - largestBox(a) || aLevel.localeCompare(bLevel)
	);

	const placed: PlacedLabel[] = [];
	for (const [, members] of ordered) {
		members.sort((a, b) => b.priority - a.priority || a.text.localeCompare(b.text));
		// This level's own names, kept apart until the whole level is known to fit.
		const pending = new Occupied();
		const pendingRects: ScreenRect[] = [];
		const labels: PlacedLabel[] = [];
		let fits = true;
		for (const candidate of members) {
			const rect = rects.get(candidate)!;
			const full = candidate.textWidth;
			const row = {
				left: rect.left,
				top: rect.top,
				right: rect.left + full,
				bottom: rect.top + LABEL_HEIGHT
			};
			const run = Math.min(occupied.clearRun(row, rect), pending.clearRun(row, rect));
			const boxWidth = rect.right - rect.left;
			const boxHoldsName = boxWidth >= MIN_LABEL_WIDTH && rect.bottom - rect.top >= LABEL_HEIGHT;
			let maxWidth = full;
			if (boxHoldsName) {
				// A box that holds a name keeps at least its own width of it: anything overlapping
				// the box itself is not the name's to avoid.
				maxWidth = Math.min(full, Math.max(run, boxWidth));
			} else if (run < full) {
				// Cut short, the names of neighbouring boxes read the same (`acc-…`) and identify
				// nothing, so this box has no room and its level goes unnamed.
				fits = false;
				break;
			}
			// Whatever of the name lies past its box is canvas the next name must stay off.
			if (rect.left + maxWidth > rect.right || rect.top + LABEL_HEIGHT > rect.bottom) {
				const extent = { ...row, right: rect.left + maxWidth };
				pending.add(extent);
				pendingRects.push(extent);
			}
			labels.push({
				id: candidate.id,
				text: candidate.text,
				color: candidate.color,
				x: candidate.x,
				y: candidate.y,
				maxWidth
			});
		}
		if (!fits) continue;
		for (const extent of pendingRects) occupied.add(extent);
		placed.push(...labels);
	}
	return placed;
}

/** Whether a box in flow coordinates shows on the pane. */
function boxOnPane(
	box: { x: number; y: number; width: number; height: number },
	viewport: LabelViewport
): boolean {
	const { zoom } = viewport;
	const left = (box.x - viewport.x) * zoom;
	const top = (box.y - viewport.y) * zoom;
	return (
		left + box.width * zoom >= 0 &&
		top + box.height * zoom >= 0 &&
		left <= viewport.width &&
		top <= viewport.height
	);
}

/** The labels whose box shows on the pane. Which boxes are named does not depend on it. */
export function onPane(
	labels: PlacedLabel[],
	boxes: ReadonlyMap<string, LabelCandidate>,
	viewport: LabelViewport
): PlacedLabel[] {
	if (!(viewport.zoom > 0)) return [];
	return labels.filter((label) => {
		const box = boxes.get(label.id);
		return box !== undefined && boxOnPane(box, viewport);
	});
}

/**
 * Every search match on the pane, named in full whatever its size and whatever its name covers: a
 * match with no name on it cannot be found, and the boxes its name covers are dimmed non-matches.
 * Highest priority first, up to `MAX_LABELS`.
 */
export function nameMatches(matches: LabelCandidate[], viewport: LabelViewport): PlacedLabel[] {
	if (!(viewport.zoom > 0)) return [];
	return matches
		.filter((candidate) => candidate.text && boxOnPane(candidate, viewport))
		.sort((a, b) => b.priority - a.priority || a.text.localeCompare(b.text))
		.slice(0, MAX_LABELS)
		.map((candidate) => ({
			id: candidate.id,
			text: candidate.text,
			color: candidate.color,
			x: candidate.x,
			y: candidate.y,
			maxWidth: candidate.textWidth
		}));
}
