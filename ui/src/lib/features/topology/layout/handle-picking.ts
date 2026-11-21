/**
 * Which side of each card an edge attaches to.
 *
 * Every candidate side pair is scored on length plus the number of other cards its straight line
 * crosses. Counting crossings against every card for every edge made this O(edges × cards), the
 * worst-scaling step left in the pipeline on a large site. `ObstacleGrid` buckets the cards once
 * per pass, so each candidate line is tested only against the cards in the grid cells it passes
 * through.
 */

import type { EdgeHandles, HandleSide } from './elk-layout';

export interface ObstacleRect {
	x: number;
	y: number;
	w: number;
	h: number;
}

/** Counts the obstacles the segment from (x1,y1) to (x2,y2) crosses. */
export type CrossingCounter = (x1: number, y1: number, x2: number, y2: number) => number;

// How much a single node crossing can increase a candidate's effective score.
// score = distSq × (1 + CROSSING_PENALTY × crossingCount), so penalty=3 lets
// a zero-crossing candidate win over a crossing candidate whose squared
// distance is up to 4× smaller. Chosen against the authentik→Docker-Bridge
// reproducer: bottom→top (803², 0 crossings) = 644,809 beats bottom→left
// (736², 1 crossing) = 2,166,784 by a wide margin.
const CROSSING_PENALTY = 3;

/**
 * Segment-vs-AABB intersection via Liang-Barsky clipping. Returns true when
 * the line from (x1,y1) to (x2,y2) shares any point with the rect. Endpoints
 * exactly on the boundary count as a hit (t ∈ [0,1]).
 */
function segmentIntersectsRect(
	x1: number,
	y1: number,
	x2: number,
	y2: number,
	rect: ObstacleRect
): boolean {
	const dx = x2 - x1;
	const dy = y2 - y1;
	let tMin = 0;
	let tMax = 1;
	const xMin = rect.x;
	const xMax = rect.x + rect.w;
	const yMin = rect.y;
	const yMax = rect.y + rect.h;

	// Clip against each of the four rect edges.
	const clip = (p: number, q: number): boolean => {
		if (p === 0) {
			// Parallel to this edge; outside the slab means no intersection.
			return q >= 0;
		}
		const t = q / p;
		if (p < 0) {
			if (t > tMax) return false;
			if (t > tMin) tMin = t;
		} else {
			if (t < tMin) return false;
			if (t < tMax) tMax = t;
		}
		return true;
	};

	if (!clip(-dx, x1 - xMin)) return false;
	if (!clip(dx, xMax - x1)) return false;
	if (!clip(-dy, y1 - yMin)) return false;
	if (!clip(dy, yMax - y1)) return false;
	return tMin <= tMax;
}

/** Cells per axis never exceed this, which bounds the grid's memory on a sprawling layout. */
const MAX_CELLS_PER_AXIS = 4096;

/**
 * Uniform grid over a set of card rects, for counting the cards a straight line crosses.
 *
 * A rect is listed in every cell it overlaps. A query walks only the cells the segment passes
 * through, column by column, and runs the exact intersection test on each rect found there once.
 * The count therefore equals a test against every rect, at a cost proportional to the cards near
 * the line.
 */
export class ObstacleGrid {
	private readonly cells = new Map<number, number[]>();
	private readonly cellSize: number;
	private readonly originX: number;
	private readonly originY: number;
	/** Per-rect stamp of the last query that tested it, so a rect in several cells counts once. */
	private readonly testedBy: Uint32Array;
	private query = 0;

	constructor(private readonly rects: ReadonlyArray<ObstacleRect & { id: string }>) {
		this.testedBy = new Uint32Array(rects.length);
		if (rects.length === 0) {
			this.cellSize = 1;
			this.originX = 0;
			this.originY = 0;
			return;
		}

		let minX = Infinity;
		let minY = Infinity;
		let maxX = -Infinity;
		let maxY = -Infinity;
		let dimSum = 0;
		for (const r of rects) {
			minX = Math.min(minX, r.x);
			minY = Math.min(minY, r.y);
			maxX = Math.max(maxX, r.x + r.w);
			maxY = Math.max(maxY, r.y + r.h);
			dimSum += Math.max(r.w, r.h);
		}
		this.originX = minX;
		this.originY = minY;
		const width = maxX - minX;
		const height = maxY - minY;
		// About one card per cell, but never so fine that the grid outgrows the cards it indexes.
		this.cellSize = Math.max(
			1,
			dimSum / rects.length,
			Math.sqrt((width * height) / (4 * rects.length)),
			width / MAX_CELLS_PER_AXIS,
			height / MAX_CELLS_PER_AXIS
		);

		rects.forEach((r, index) => {
			const cx0 = this.column(r.x);
			const cx1 = this.column(r.x + r.w);
			const cy0 = this.row(r.y);
			const cy1 = this.row(r.y + r.h);
			for (let cx = cx0; cx <= cx1; cx++) {
				for (let cy = cy0; cy <= cy1; cy++) {
					const key = cellKey(cx, cy);
					const cell = this.cells.get(key);
					if (cell) cell.push(index);
					else this.cells.set(key, [index]);
				}
			}
		});
	}

	/** Rects the segment crosses, leaving out any whose id is in `exclude`. */
	countCrossings(
		x1: number,
		y1: number,
		x2: number,
		y2: number,
		exclude: ReadonlySet<string>
	): number {
		if (this.rects.length === 0) return 0;
		this.query += 1;
		if (this.query === 0xffffffff) {
			this.testedBy.fill(0);
			this.query = 1;
		}
		const stamp = this.query;

		let crossings = 0;
		const xLo = Math.min(x1, x2);
		const xHi = Math.max(x1, x2);
		const slope = x1 === x2 ? 0 : (y2 - y1) / (x2 - x1);
		const yAt = (x: number) => y1 + slope * (x - x1);
		// A hair of slack so a segment lying exactly on a cell boundary still visits both sides.
		const slack = this.cellSize * 1e-9;

		for (let cx = this.column(xLo); cx <= this.column(xHi); cx++) {
			// The part of the segment inside this column, and the rows it spans there.
			const colLeft = this.originX + cx * this.cellSize;
			const xa = Math.max(xLo, colLeft);
			const xb = Math.min(xHi, colLeft + this.cellSize);
			const ya = x1 === x2 ? Math.min(y1, y2) : yAt(xa);
			const yb = x1 === x2 ? Math.max(y1, y2) : yAt(xb);
			const cy0 = this.row(Math.min(ya, yb) - slack);
			const cy1 = this.row(Math.max(ya, yb) + slack);

			for (let cy = cy0; cy <= cy1; cy++) {
				const cell = this.cells.get(cellKey(cx, cy));
				if (!cell) continue;
				for (const index of cell) {
					if (this.testedBy[index] === stamp) continue;
					this.testedBy[index] = stamp;
					const rect = this.rects[index];
					if (exclude.has(rect.id)) continue;
					if (segmentIntersectsRect(x1, y1, x2, y2, rect)) crossings += 1;
				}
			}
		}
		return crossings;
	}

	private column(x: number): number {
		return Math.floor((x - this.originX) / this.cellSize);
	}

	private row(y: number): number {
		return Math.floor((y - this.originY) / this.cellSize);
	}
}

/**
 * Cells sit in [-1, MAX_CELLS_PER_AXIS + 1] on each axis (a query's slack can reach one past
 * either end), so a fixed stride keys them uniquely.
 */
function cellKey(cx: number, cy: number): number {
	return (cx + 1) * (MAX_CELLS_PER_AXIS + 4) + (cy + 1);
}

/**
 * Compute optimal handle sides by picking the pair that minimizes a
 * crossing-aware score across all 4×4 candidate pairs. Distance is the
 * primary signal; `obstacles` (if supplied, pre-filtered to exclude source,
 * target, and their ancestors) adds a crossing-count penalty so a slightly
 * longer edge that clears an unrelated node can beat a shorter one that
 * cuts through it. With no obstacles the picker is distance-only. `obstacles`
 * is either the rects themselves or a counter over an index of them (see
 * `ObstacleGrid`). Ties resolved by iteration order (Top, Right, Bottom, Left).
 */
export function computeOptimalHandles(
	srcPos: { x: number; y: number },
	srcSize: { w: number; h: number },
	tgtPos: { x: number; y: number },
	tgtSize: { w: number; h: number },
	obstacles?: ReadonlyArray<ObstacleRect> | CrossingCounter
): EdgeHandles {
	const SIDES: HandleSide[] = ['Top', 'Right', 'Bottom', 'Left'];
	const anchor = (
		pos: { x: number; y: number },
		size: { w: number; h: number },
		side: HandleSide
	): { x: number; y: number } => {
		const cx = pos.x + size.w / 2;
		const cy = pos.y + size.h / 2;
		switch (side) {
			case 'Top':
				return { x: cx, y: pos.y };
			case 'Bottom':
				return { x: cx, y: pos.y + size.h };
			case 'Left':
				return { x: pos.x, y: cy };
			case 'Right':
				return { x: pos.x + size.w, y: cy };
		}
	};
	const countCrossings: CrossingCounter | null =
		typeof obstacles === 'function'
			? obstacles
			: obstacles && obstacles.length > 0
				? (x1, y1, x2, y2) => {
						let n = 0;
						for (const rect of obstacles) {
							if (segmentIntersectsRect(x1, y1, x2, y2, rect)) n++;
						}
						return n;
					}
				: null;
	// A chord that leaves a side heading back into its own card passes through that card, which the
	// obstacle count never sees because endpoints are excluded from it. The drawn edge leaves the
	// side outward and doubles back across the card, so it counts as one crossing. When one card
	// contains the other, every side of the outer card faces inward, so the check is skipped.
	const contains = (
		outer: { x: number; y: number },
		outerSize: { w: number; h: number },
		inner: { x: number; y: number },
		innerSize: { w: number; h: number }
	) =>
		inner.x >= outer.x &&
		inner.y >= outer.y &&
		inner.x + innerSize.w <= outer.x + outerSize.w &&
		inner.y + innerSize.h <= outer.y + outerSize.h;
	const nested =
		contains(srcPos, srcSize, tgtPos, tgtSize) || contains(tgtPos, tgtSize, srcPos, srcSize);
	const facesAway = (side: HandleSide, dx: number, dy: number): boolean => {
		switch (side) {
			case 'Top':
				return dy > 0;
			case 'Bottom':
				return dy < 0;
			case 'Left':
				return dx > 0;
			case 'Right':
				return dx < 0;
		}
	};

	let best: EdgeHandles = { sourceHandle: 'Bottom', targetHandle: 'Top' };
	let bestScore = Infinity;
	for (const sourceHandle of SIDES) {
		const sp = anchor(srcPos, srcSize, sourceHandle);
		for (const targetHandle of SIDES) {
			const tp = anchor(tgtPos, tgtSize, targetHandle);
			const dx = tp.x - sp.x;
			const dy = tp.y - sp.y;
			const distSq = dx * dx + dy * dy;
			let crossings = countCrossings ? countCrossings(sp.x, sp.y, tp.x, tp.y) : 0;
			if (!nested) {
				if (facesAway(sourceHandle, dx, dy)) crossings += 1;
				if (facesAway(targetHandle, -dx, -dy)) crossings += 1;
			}
			const score = distSq * (1 + CROSSING_PENALTY * crossings);
			if (score < bestScore) {
				bestScore = score;
				best = { sourceHandle, targetHandle };
			}
		}
	}
	return best;
}

/**
 * Left-to-right handles for two ends that sit side by side, as in L2's columns: out of the right
 * side of the left end, into the left side of the right one. Null when the ends share any
 * horizontal span, such as cards stacked above one another on a ring: from the right side of one
 * to the left of the other, the line loops back across both, so those take `computeOptimalHandles`.
 */
export function sideBySideHandles(
	srcPos: { x: number; y: number },
	srcSize: { w: number; h: number },
	tgtPos: { x: number; y: number },
	tgtSize: { w: number; h: number }
): EdgeHandles | null {
	const sideBySide = srcPos.x + srcSize.w <= tgtPos.x || tgtPos.x + tgtSize.w <= srcPos.x;
	if (!sideBySide) return null;
	const leftToRight = srcPos.x < tgtPos.x;
	return {
		sourceHandle: leftToRight ? 'Right' : 'Left',
		targetHandle: leftToRight ? 'Left' : 'Right'
	};
}
