/**
 * Which container names to draw over a zoomed-out canvas, and how wide each may be.
 *
 * Below the zoom where a header can be read (`DETAIL_ZOOM` in `render-mode.ts`) a container's
 * name is either a few pixels tall or, at reduced detail, not mounted at all: thousands of headers
 * nobody can read cost a great deal and say nothing. That leaves a zoomed-out map with no names
 * on it. The names drawn here go on one layer above every node, at a fixed size on screen.
 *
 * A name always starts on its own box, at the top-left corner. On a box with room for it the name
 * never runs past the box's right edge: it is cut short to the box's width, and the viewer shows
 * it in full while the pointer is on the name. A box too small for a name gets a short one that
 * runs past its edge onto empty canvas: a name over another box reads as belonging to the box it
 * covers. Small boxes are named together or not at all, so one screen never has a few of them
 * named and the rest bare.
 *
 * A layer above the nodes is not covered by the container's children, which is what rules this
 * out for the container's own header.
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
	/** As wide as the name may be drawn, in screen pixels: the box's width, or more on a narrow
	 * box with empty canvas beside it. */
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
 * Narrowest a name is drawn, on screen: room for about five characters and the ellipsis. Names on
 * one site tend to share a prefix (`acc-sw-058`, `acc-sw-059`), so a shorter cut would label every
 * box the same. A narrower box gets a name when the canvas beside it is empty.
 */
export const MIN_LABEL_WIDTH = 48;

/**
 * Width a name on a narrower box grows to when the canvas beside the box is empty: about fourteen
 * characters, enough to tell `Docker Bridge` from `Docker Swarm`.
 */
export const ROOMY_LABEL_WIDTH = 96;

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

	/** Whether anything other than `own` overlaps `rect`. */
	blocks(rect: ScreenRect, own: ScreenRect): boolean {
		for (const key of this.keys(rect)) {
			for (const other of this.cells.get(key) ?? []) {
				if (other === own) continue;
				if (
					other.left < rect.right &&
					other.right > rect.left &&
					other.top < rect.bottom &&
					other.bottom > rect.top
				) {
					return true;
				}
			}
		}
		return false;
	}
}

/**
 * @param nameEvery Name every candidate on the pane, whatever its size and whatever its name
 * covers. Set while searching, when the candidates are the containers holding a match: a match
 * with no name on it cannot be found, and the boxes its name covers are dimmed non-matches.
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

	// A box too small to hold a name is named only when every such box on the pane has empty
	// canvas for one. Otherwise the boxes on the rim of a packed grid would be named and the
	// thousands inside it would not.
	let smallBoxBlocked = false;
	const placed: (PlacedLabel & { boxHoldsName: boolean })[] = [];
	for (const { candidate, rect } of onPane) {
		const boxWidth = rect.right - rect.left;
		if (nameEvery) {
			placed.push({
				id: candidate.id,
				text: candidate.text,
				color: candidate.color,
				x: candidate.x,
				y: candidate.y,
				maxWidth: Math.max(boxWidth, ROOMY_LABEL_WIDTH),
				boxHoldsName: true
			});
			continue;
		}
		const boxHoldsName =
			boxWidth >= MIN_LABEL_WIDTH && rect.bottom - rect.top >= LABEL_HEIGHT;
		if (!boxHoldsName && smallBoxBlocked) continue;
		// Widest first. A name past its box's edge, or below it, is drawn only where it lands on
		// empty canvas. Among packed boxes it would cover a neighbour, so a box too small for a
		// name stays unnamed there until the zoom gives it the room.
		const widths =
			boxWidth >= ROOMY_LABEL_WIDTH
				? [boxWidth]
				: [ROOMY_LABEL_WIDTH, Math.max(boxWidth, MIN_LABEL_WIDTH)];
		let maxWidth: number | null = null;
		for (const width of widths) {
			const label = {
				left: rect.left,
				top: rect.top,
				right: rect.left + width,
				bottom: rect.top + LABEL_HEIGHT
			};
			const overhangs = label.right > rect.right || label.bottom > rect.bottom;
			if (overhangs && occupied.blocks(label, rect)) continue;
			// The next box's name must stay off this one.
			if (overhangs) occupied.add(label);
			maxWidth = width;
			break;
		}
		if (maxWidth === null) {
			smallBoxBlocked = true;
			continue;
		}
		placed.push({
			id: candidate.id,
			text: candidate.text,
			color: candidate.color,
			x: candidate.x,
			y: candidate.y,
			maxWidth,
			boxHoldsName
		});
	}
	return placed
		.filter((label) => label.boxHoldsName || !smallBoxBlocked)
		.slice(0, MAX_LABELS)
		.map(({ boxHoldsName: _, ...label }) => label);
}
