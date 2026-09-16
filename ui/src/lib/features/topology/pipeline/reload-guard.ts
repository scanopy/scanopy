/**
 * Guards against the pipeline re-triggering itself.
 *
 * Every store the viewer watches re-runs the whole pipeline when it changes —
 * two `elk.layout()` calls and a full DOM measure pass. But several of those
 * writes come from *inside* a run:
 *
 *  - `prepare` writes `collapsedContainers` while seeding collapse state,
 *    stripping stale ids, or applying a level on view switch. The run then uses
 *    the value it just wrote, so re-running changes nothing.
 *  - Derived option stores (hidden edge types, tag filters) fire whenever their
 *    parent object is replaced during hydration, even when the derived value is
 *    byte-identical.
 *
 * Both set `pendingReload`, so a cold load pays for extra complete runs that
 * cannot produce a different result.
 *
 * The fix is to compare, at the end of a run, the stores' current values
 * against the values that run actually consumed — snapshotted once `prepare`
 * has returned, which is the point where the run's inputs are fixed. A reload
 * happens only when something genuinely differs.
 */

export interface ReloadInputs {
	collapsed: Set<string>;
	expandedBundles: Set<string>;
	expandedPorts: Set<string>;
	bundleEdges: boolean;
	hiddenEdgeTypes: string;
	tagHidden: Set<string>;
	/**
	 * Entities hidden by a filter, in entity-space. Distinct from `tagHidden`, which holds *node*
	 * ids: an entity shown inline on another node's card has no node of its own, so hiding it
	 * changes that card's height while every node id stays the same. Omitting it here meant a
	 * filter change that arrived mid-run was queued and then suppressed as a no-op.
	 */
	hiddenEntities: Set<string>;
	/**
	 * Metadata-value filters for the active view (e.g. hiding the OpenPorts service category),
	 * pre-serialized. These are applied at render time straight from the options and never reach
	 * either hidden-id store, so nothing else here would notice them change.
	 */
	hiddenMetadata: string;
	/**
	 * The topology the run laid out, compared by identity.
	 *
	 * Every other input here is a filter or view setting; this is the graph itself. Leaving it out
	 * meant a refetched bundle that landed mid-run queued a reload, the run ended, the filter
	 * inputs matched, and the reload was suppressed as a no-op — so the new nodes were never laid
	 * out until a page refresh. A server-side filter makes that the normal sequence, not a race:
	 * clearing it rewrites the options immediately (starting a run) and the bundle carrying the
	 * restored entities arrives a round trip later.
	 *
	 * Identity, not content: an unchanged run of the idle path already re-lays out on every new
	 * topology object, so this only makes the in-flight path agree with it.
	 */
	topology: unknown;
}

/** Order-independent set equality. */
function sameSet(a: Set<string>, b: Set<string>): boolean {
	if (a === b) return true;
	if (a.size !== b.size) return false;
	for (const value of a) {
		if (!b.has(value)) return false;
	}
	return true;
}

/**
 * True when `next` would produce the same pipeline result as `previous`.
 *
 * Compares by value, not identity: a store that emits a fresh `Set` with the
 * same members has not changed anything the pipeline cares about.
 */
export function reloadInputsEqual(previous: ReloadInputs, next: ReloadInputs): boolean {
	return reloadInputsDiff(previous, next).length === 0;
}

/**
 * Names of the inputs that differ. Empty means a reload would be a no-op.
 *
 * Returning the field names rather than a bare boolean keeps the reason a
 * reload happened attributable — the difference between "the pipeline re-ran
 * once more" and knowing which store caused it.
 */
export function reloadInputsDiff(previous: ReloadInputs, next: ReloadInputs): string[] {
	const changed: string[] = [];
	if (previous.bundleEdges !== next.bundleEdges) changed.push('bundleEdges');
	if (previous.hiddenEdgeTypes !== next.hiddenEdgeTypes) changed.push('hiddenEdgeTypes');
	if (!sameSet(previous.collapsed, next.collapsed)) changed.push('collapsed');
	if (!sameSet(previous.expandedBundles, next.expandedBundles)) changed.push('expandedBundles');
	if (!sameSet(previous.expandedPorts, next.expandedPorts)) changed.push('expandedPorts');
	if (!sameSet(previous.tagHidden, next.tagHidden)) changed.push('tagHidden');
	if (!sameSet(previous.hiddenEntities, next.hiddenEntities)) changed.push('hiddenEntities');
	if (previous.hiddenMetadata !== next.hiddenMetadata) changed.push('hiddenMetadata');
	if (previous.topology !== next.topology) changed.push('topology');
	return changed;
}

/** Defensive copy, so later mutation of a store's set can't alias the snapshot. */
export function snapshotReloadInputs(inputs: ReloadInputs): ReloadInputs {
	return {
		collapsed: new Set(inputs.collapsed),
		expandedBundles: new Set(inputs.expandedBundles),
		expandedPorts: new Set(inputs.expandedPorts),
		bundleEdges: inputs.bundleEdges,
		hiddenEdgeTypes: inputs.hiddenEdgeTypes,
		tagHidden: new Set(inputs.tagHidden),
		hiddenEntities: new Set(inputs.hiddenEntities),
		hiddenMetadata: inputs.hiddenMetadata,
		// A reference on purpose: identity is the comparison, and copying would break it.
		topology: inputs.topology
	};
}
