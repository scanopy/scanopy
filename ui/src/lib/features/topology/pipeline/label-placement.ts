/**
 * Which container names to draw over a zoomed-out canvas, and how wide each may be.
 *
 * Below the zoom where a header can be read (`DETAIL_ZOOM` in `render-mode.ts`) a container's
 * name is either a few pixels tall or, at reduced detail, not mounted at all: thousands of headers
 * nobody can read cost a great deal and say nothing. That leaves a zoomed-out map with no names
 * on it. The names drawn here go on one layer above every node, at a fixed size on screen.
 *
 * A name always sits on its own box, at the top-left corner, and never runs past the box's right
 * edge: it is cut short to the box's width, and the viewer shows it in full while the pointer is
 * over the box. A name beside or overhanging its box reads as belonging to whatever it covers,
 * and a short box under a long name disappears behind it.
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
	/** The box's width on screen, in pixels: as wide as the name may be drawn. */
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
 * Narrowest box, on screen, that gets a name: room for about five characters and the ellipsis.
 * Names on one site tend to share a prefix (`acc-sw-058`, `acc-sw-059`), so a shorter cut would
 * label every box the same.
 */
export const MIN_LABEL_WIDTH = 48;

/** More names than this on one screen is a wall of text, whatever the room. */
export const MAX_LABELS = 300;

export function placeLabels(candidates: LabelCandidate[], viewport: LabelViewport): PlacedLabel[] {
	const { zoom } = viewport;
	if (zoom <= 0) return [];

	const fitting: LabelCandidate[] = [];
	for (const candidate of candidates) {
		if (!candidate.text) continue;
		const left = (candidate.x - viewport.x) * zoom;
		const top = (candidate.y - viewport.y) * zoom;
		const width = candidate.width * zoom;
		const height = candidate.height * zoom;
		// Off the pane entirely.
		if (left + width < 0 || top + height < 0 || left > viewport.width || top > viewport.height) {
			continue;
		}
		// A box narrower than a few characters, or shorter than the label, cannot hold a name
		// without the name spilling onto its neighbours.
		if (width < MIN_LABEL_WIDTH || height < LABEL_HEIGHT) continue;
		fitting.push(candidate);
	}

	fitting.sort((a, b) => b.priority - a.priority || a.text.localeCompare(b.text));

	return fitting.slice(0, MAX_LABELS).map((candidate) => ({
		id: candidate.id,
		text: candidate.text,
		color: candidate.color,
		x: candidate.x,
		y: candidate.y,
		maxWidth: candidate.width * zoom
	}));
}
