/**
 * Whether every visible row is selected.
 *
 * Compares membership rather than counts. A count comparison is satisfiable by
 * any N selected ids — including N carried over from a previous page — which
 * made the control offer "deselect all" while a bulk delete would have acted on
 * an entirely different set of rows.
 */
export function isAllSelected<T>(
	visible: T[],
	selected: ReadonlySet<string>,
	getId: (item: T) => string
): boolean {
	return visible.length > 0 && visible.every((item) => selected.has(getId(item)));
}

/** Whether some, but not all, visible rows are selected — the checkbox's third state. */
export function isPartiallySelected<T>(
	visible: T[],
	selected: ReadonlySet<string>,
	getId: (item: T) => string
): boolean {
	if (visible.length === 0) return false;
	const hits = visible.filter((item) => selected.has(getId(item))).length;
	return hits > 0 && hits < visible.length;
}

/** The ids of every visible row, for select-all. */
export function visibleIds<T>(visible: T[], getId: (item: T) => string): string[] {
	return visible.map(getId).filter((id) => Boolean(id));
}

/**
 * The ids spanned by a shift-click, inclusive of both ends.
 *
 * Order-independent: dragging a selection upwards covers the same rows as
 * dragging it down. Ids outside `visible` yield an empty range rather than
 * throwing, since a stale anchor is reachable by paging between clicks.
 */
export function rangeSelect<T>(
	visible: T[],
	anchorId: string,
	targetId: string,
	getId: (item: T) => string
): string[] {
	const ids = visible.map(getId);
	const from = ids.indexOf(anchorId);
	const to = ids.indexOf(targetId);

	if (from === -1 || to === -1) return [];

	const [start, end] = from <= to ? [from, to] : [to, from];
	return ids.slice(start, end + 1);
}

/**
 * Run a bulk action over the current selection, reporting whether it ran.
 *
 * Delete, tag-add and tag-remove all need the same three things: refuse when
 * the action is unavailable or nothing is selected, await the call, and survive
 * a failure without taking the list down with it. Writing that out three times
 * is how one of them ends up missing its guard.
 */
export async function runBulkAction(
	description: string,
	selected: ReadonlySet<string>,
	enabled: boolean,
	run: (ids: string[]) => Promise<unknown>
): Promise<boolean> {
	if (!enabled || selected.size === 0) return false;

	try {
		await run(Array.from(selected));
		return true;
	} catch (error) {
		console.error(`${description} failed:`, error);
		return false;
	}
}
