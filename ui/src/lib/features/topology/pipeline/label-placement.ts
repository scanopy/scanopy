/**
 * Which container names to draw over a zoomed-out canvas, and how wide each may be.
 *
 * Below the zoom where a header can be read (`DETAIL_ZOOM` in `render-mode.ts`) a container's
 * name is either a few pixels tall or, at reduced detail, not mounted at all: thousands of headers
 * nobody can read cost a great deal and say nothing. That leaves a zoomed-out map with no names
 * on it. The names drawn here go on one layer above every node, at a fixed size on screen.
 *
 * A name always starts on its own box, at the top-left corner, and runs to its full width. It is
 * cut short only where another box or another name begins, and the viewer shows it in full while
 * the pointer is on it: a name over another box reads as belonging to the box it covers. Each box
 * is named on its own, whenever it has room for at least a few characters. Larger boxes claim
 * their room first.
 *
 * A layer above the nodes is not covered by the container's children, which is what rules this
 * out for the container's own header.
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
	/** Higher is kept first when there are more names than `MAX_LABELS`. */
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

/**
 * Narrowest a cut name is drawn, on screen: room for about five characters and the ellipsis. Names
 * on one site tend to share a prefix (`acc-sw-058`, `acc-sw-059`), so a shorter cut would label
 * every box the same. A box with less room than this gets no name.
 */
export const MIN_LABEL_WIDTH = 48;

/** Space kept between a cut name and the box or name it stops at, in screen pixels. */
export const LABEL_GAP = 4;

/** More names than this on one screen is a wall of text, whatever the room. */
export const MAX_LABELS = 300;

/** A rectangle on the pane, in screen pixels. */
interface ScreenRect {
	left: number;
	top: number;
	right: number;
	bottom: number;
}

/** Side of a cell in the index of what is on the pane, in screen pixels. */
const CELL = 64;

/**
 * What is on the pane, by cell, so that asking whether a label's rectangle is free looks at the
 * few boxes near it and not at every box. A large site has thousands of boxes on one screen.
 */
class Occupied {
	private cells = new Map<number, ScreenRect[]>();

	constructor(
		private paneWidth: number,
		private paneHeight: number
	) {}

	private *keys(rect: ScreenRect): Generator<number> {
		const firstX = Math.floor(Math.max(rect.left, 0) / CELL);
		const lastX = Math.floor(Math.min(rect.right, this.paneWidth) / CELL);
		const firstY = Math.floor(Math.max(rect.top, 0) / CELL);
		const lastY = Math.floor(Math.min(rect.bottom, this.paneHeight) / CELL);
		for (let cx = firstX; cx <= lastX; cx++) {
			for (let cy = firstY; cy <= lastY; cy++) yield cx * 65536 + cy;
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
 * @param nameEvery Name every candidate on the pane at its full width, whatever its size and
 * whatever its name covers. Set while searching, when the candidates are the containers holding a
 * match: a match with no name on it cannot be found, and the boxes its name covers are dimmed
 * non-matches.
 */
export function placeLabels(
	candidates: LabelCandidate[],
	viewport: LabelViewport,
	nameEvery = false
): PlacedLabel[] {
	const { zoom } = viewport;
	if (zoom <= 0) return [];

	const onPane: { candidate: LabelCandidate; rect: ScreenRect }[] = [];
	const occupied = new Occupied(viewport.width, viewport.height);
	for (const candidate of candidates) {
		const left = (candidate.x - viewport.x) * zoom;
		const top = (candidate.y - viewport.y) * zoom;
		const rect = {
			left,
			top,
			right: left + candidate.width * zoom,
			bottom: top + candidate.height * zoom
		};
		// Off the pane entirely.
		if (rect.right < 0 || rect.bottom < 0 || left > viewport.width || top > viewport.height) {
			continue;
		}
		// A box with no name is not labelled, and a label must still stay off it.
		occupied.add(rect);
		if (candidate.text) onPane.push({ candidate, rect });
	}

	onPane.sort(
		(a, b) =>
			b.candidate.priority - a.candidate.priority ||
			a.candidate.text.localeCompare(b.candidate.text)
	);

	const placed: PlacedLabel[] = [];
	for (const { candidate, rect } of onPane) {
		if (placed.length === MAX_LABELS) break;
		const full = candidate.textWidth;
		let maxWidth = full;
		if (!nameEvery) {
			const boxWidth = rect.right - rect.left;
			const boxHoldsName = boxWidth >= MIN_LABEL_WIDTH && rect.bottom - rect.top >= LABEL_HEIGHT;
			const run = occupied.clearRun(
				{
					left: rect.left,
					top: rect.top,
					right: rect.left + full,
					bottom: rect.top + LABEL_HEIGHT
				},
				rect
			);
			// A box that holds a name keeps at least its own width of it: anything overlapping the
			// box itself is not the name's to avoid.
			maxWidth = Math.min(full, boxHoldsName ? Math.max(run, boxWidth) : run);
			if (maxWidth < Math.min(full, MIN_LABEL_WIDTH)) continue;
			// Whatever of the name lies past its box is canvas the next name must stay off.
			if (rect.left + maxWidth > rect.right || rect.top + LABEL_HEIGHT > rect.bottom) {
				occupied.add({
					left: rect.left,
					top: rect.top,
					right: rect.left + maxWidth,
					bottom: rect.top + LABEL_HEIGHT
				});
			}
		}
		placed.push({
			id: candidate.id,
			text: candidate.text,
			color: candidate.color,
			x: candidate.x,
			y: candidate.y,
			maxWidth
		});
	}
	return placed;
}
