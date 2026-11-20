/**
 * Packs boxes beside boxes already placed, keeping the whole as close to a screen's aspect as it
 * can, so the packing fits a screen at the highest zoom.
 */

import { connectedGroups } from './child-order';

export interface Box {
	x: number;
	y: number;
	width: number;
	height: number;
}

/** The placed box `box` would overlap, counting `spacing` around each. */
function overlapping(placed: Map<string, Box>, box: Box, spacing: number): Box | undefined {
	for (const other of placed.values()) {
		if (
			box.x < other.x + other.width + spacing &&
			other.x < box.x + box.width + spacing &&
			box.y < other.y + other.height + spacing &&
			other.y < box.y + box.height + spacing
		) {
			return other;
		}
	}
	return undefined;
}

/**
 * The two free slots flush against placed box `p`: below it, moved down past anything in the way,
 * and right of it, moved right past anything in the way.
 */
export function slotsBeside(
	placed: Map<string, Box>,
	p: Box,
	size: { width: number; height: number },
	spacing: number
): [Box, Box] {
	const below = { x: p.x, y: p.y + p.height + spacing, ...size };
	for (let hit = overlapping(placed, below, spacing); hit;) {
		below.y = hit.y + hit.height + spacing;
		hit = overlapping(placed, below, spacing);
	}
	const right = { x: p.x + p.width + spacing, y: p.y, ...size };
	for (let hit = overlapping(placed, right, spacing); hit;) {
		right.x = hit.x + hit.width + spacing;
		hit = overlapping(placed, right, spacing);
	}
	return [below, right];
}

/**
 * Packs each box of `toPlace`, in the order given, into the free slot beside a box of `placed`
 * that grows the bounding box least, measured as the smallest rectangle of `aspect` that holds it.
 * Ties go to the higher slot, then the one further left. With nothing placed, the first box goes
 * at the origin. Boxes already in `placed` never move; each packed box is added to it.
 */
export function packBoxes(
	placed: Map<string, Box>,
	toPlace: { id: string; width: number; height: number }[],
	aspect: number,
	spacing: number
): void {
	for (const { id, width, height } of toPlace) {
		if (placed.size === 0) {
			placed.set(id, { x: 0, y: 0, width, height });
			continue;
		}

		let minX = Infinity;
		let minY = Infinity;
		let maxX = -Infinity;
		let maxY = -Infinity;
		for (const box of placed.values()) {
			minX = Math.min(minX, box.x);
			minY = Math.min(minY, box.y);
			maxX = Math.max(maxX, box.x + box.width);
			maxY = Math.max(maxY, box.y + box.height);
		}
		// The bounding box as a screen fits it, so a tall, narrow packing costs what it costs to view.
		const areaWith = (slot: Box) => {
			const w = Math.max(maxX, slot.x + slot.width) - Math.min(minX, slot.x);
			const h = Math.max(maxY, slot.y + slot.height) - Math.min(minY, slot.y);
			return Math.max(w, h * aspect) * Math.max(h, w / aspect);
		};

		let best: { slot: Box; area: number } | undefined;
		for (const box of placed.values()) {
			for (const slot of slotsBeside(placed, box, { width, height }, spacing)) {
				const area = areaWith(slot);
				if (
					!best ||
					area < best.area ||
					(area === best.area &&
						(slot.y < best.slot.y || (slot.y === best.slot.y && slot.x < best.slot.x)))
				) {
					best = { slot, area };
				}
			}
		}
		placed.set(id, best!.slot);
	}
}

/** Grid that packed components are moved by, matching the grid layouts snap to. */
const SNAP = 25;

/**
 * Tiles the connected components of `graph` to `aspect`. Each component keeps its own layout and
 * moves as one block, largest first (ties by its first node in `graph.nodes`), into the slot that
 * grows the whole least. Returns the moved position of every node that had a position and a size.
 */
export function packComponents(
	graph: { nodes: string[]; adjacency: Map<string, Set<string>> },
	positions: ReadonlyMap<string, { x: number; y: number }>,
	sizes: ReadonlyMap<string, { width: number; height: number }>,
	aspect: number,
	spacing: number
): Map<string, { x: number; y: number }> {
	const edges = graph.nodes.flatMap((source) =>
		[...graph.adjacency.get(source)!].map((target) => ({ source, target }))
	);
	const groupOf = connectedGroups(graph.nodes, edges);

	const blocks = new Map<string, { members: string[]; box: Box; order: number }>();
	graph.nodes.forEach((id, order) => {
		const position = positions.get(id);
		const size = sizes.get(id);
		if (!position || !size) return;
		const group = groupOf.get(id)!;
		const block = blocks.get(group);
		if (!block) {
			blocks.set(group, { members: [id], box: { ...position, ...size }, order });
			return;
		}
		block.members.push(id);
		const { box } = block;
		const right = Math.max(box.x + box.width, position.x + size.width);
		const bottom = Math.max(box.y + box.height, position.y + size.height);
		box.x = Math.min(box.x, position.x);
		box.y = Math.min(box.y, position.y);
		box.width = right - box.x;
		box.height = bottom - box.y;
	});

	const ordered = [...blocks.entries()].sort(
		([, a], [, b]) => b.box.width * b.box.height - a.box.width * a.box.height || a.order - b.order
	);
	const placed = new Map<string, Box>();
	packBoxes(
		placed,
		ordered.map(([group, { box }]) => ({ id: group, width: box.width, height: box.height })),
		aspect,
		spacing
	);

	const moved = new Map<string, { x: number; y: number }>();
	for (const [group, { members, box }] of ordered) {
		const slot = placed.get(group)!;
		const dx = Math.round((slot.x - box.x) / SNAP) * SNAP;
		const dy = Math.round((slot.y - box.y) / SNAP) * SNAP;
		for (const id of members) {
			const p = positions.get(id)!;
			moved.set(id, { x: p.x + dx, y: p.y + dy });
		}
	}
	return moved;
}

/** The box around `ids`, from their positions and sizes. Null when none has both. */
export function rootBounds(
	ids: readonly string[],
	positions: ReadonlyMap<string, { x: number; y: number }>,
	sizes: ReadonlyMap<string, { width: number; height: number }>
): Box | null {
	let left = Infinity;
	let top = Infinity;
	let right = -Infinity;
	let bottom = -Infinity;
	let any = false;
	for (const id of ids) {
		const p = positions.get(id);
		const s = sizes.get(id);
		if (!p || !s) continue;
		any = true;
		left = Math.min(left, p.x);
		top = Math.min(top, p.y);
		right = Math.max(right, p.x + s.width);
		bottom = Math.max(bottom, p.y + s.height);
	}
	return any ? { x: left, y: top, width: right - left, height: bottom - top } : null;
}

/**
 * Most a ring layout may grow over the layout it replaces, measured as the screen it needs: the
 * area of the smallest box of the screen's aspect that holds it. Past this the viewer zooms out so
 * far to fit it that cards are specks, or, below its smallest zoom, cannot fit it at all and shows
 * a stretch of empty canvas.
 */
export const RING_AREA_LIMIT = 2;

/** Area of the smallest box of `aspect` (width over height) that holds `box`. */
function fittedArea(box: Box, aspect: number): number {
	return Math.max(box.width, box.height * aspect) * Math.max(box.height, box.width / aspect);
}

/**
 * Whether a ring layout may replace the layout it was made from: every position a number, and the
 * screen of `aspect` it needs at most `RING_AREA_LIMIT` times what the original needs.
 */
export function ringLayoutAcceptable(
	ringed: Box | null,
	original: Box | null,
	aspect: number
): boolean {
	if (!ringed || !original) return false;
	const { x, y, width, height } = ringed;
	if (![x, y, width, height].every(Number.isFinite)) return false;
	return fittedArea(ringed, aspect) <= RING_AREA_LIMIT * Math.max(fittedArea(original, aspect), 1);
}
