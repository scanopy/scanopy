import type { TopologyEdge } from '../types/base';
import type { components } from '$lib/api/schema';
import {
	edgeTypes,
	views,
	type EdgeSelectionScope,
	type EdgeTypeMetadata
} from '$lib/shared/stores/metadata';
import { queryClient, queryKeys } from '$lib/api/query-client';
import { neighborEvidenceFreshness, neighborEvidenceTag } from '$lib/shared/utils/freshness';
import type { TagProps } from '$lib/shared/components/data/types';

type EdgeTypeDiscriminants = components['schemas']['EdgeTypeDiscriminants'];
type Interface = components['schemas']['Interface'];
type InterfaceNeighborRow = components['schemas']['InterfaceNeighborRow'];
type Network = components['schemas']['Network'];
type EdgeViewConfig = components['schemas']['EdgeViewConfig'];
type TopologyView = components['schemas']['TopologyView'];

/** Get the view config from an edge, defaulting to Disabled if absent. */
export function getViewConfig(edge: TopologyEdge): EdgeViewConfig {
	const vc = (edge as Record<string, unknown>).view_config as EdgeViewConfig | undefined;
	return vc ?? { type: 'disabled' };
}

/** Whether this edge is disabled (not available) in the current view. */
export function isDisabledEdge(edge: TopologyEdge): boolean {
	return getViewConfig(edge).type === 'disabled';
}

/** Whether this edge should affect ELK layout positioning. */
export function affectsLayout(edge: TopologyEdge): boolean {
	const vc = getViewConfig(edge);
	return vc.type === 'active' && vc.affects_layout;
}

/** Whether this edge is hidden by default (togglable). */
export function isHiddenByDefault(edge: TopologyEdge): boolean {
	const vc = getViewConfig(edge);
	return vc.type === 'active' && vc.default_visibility === 'hidden';
}

/** Whether this edge uses dotted stroke in the current view. */
export function isDottedEdge(edge: TopologyEdge): boolean {
	const vc = getViewConfig(edge);
	return vc.type === 'active' && vc.stroke === 'dotted';
}

/**
 * Whether this edge annotates the graph rather than structuring it — any non-solid stroke.
 * Drives the shared overlay treatment (thinner, dimmed until highlighted); the specific stroke
 * only decides the dash pattern.
 */
export function isOverlayEdge(edge: TopologyEdge): boolean {
	const vc = getViewConfig(edge);
	return vc.type === 'active' && vc.stroke !== 'solid';
}

/** Get the highlight behavior for an edge. Defaults to 'when_visible'. */
export function getHighlightBehavior(edge: TopologyEdge): 'when_visible' | 'always' | 'never' {
	const vc = getViewConfig(edge);
	if (vc.type !== 'active') return 'when_visible';
	return (
		((vc as Record<string, unknown>).highlight_behavior as
			| 'when_visible'
			| 'always'
			| 'never'
			| undefined) ?? 'when_visible'
	);
}

/** Whether this edge should show directional animation when highlighted. */
export function showDirectionality(edge: TopologyEdge): boolean {
	const vc = getViewConfig(edge);
	return vc.type === 'active' && vc.show_directionality;
}

/** What a click on this edge type highlights. Defaults to the clicked segment alone. */
export function getSelectionScope(edgeType: string): EdgeSelectionScope {
	// getMetadata falls back to an empty object for unknown ids, so treat the field as optional.
	const meta = edgeTypes.getMetadata(edgeType) as Partial<EdgeTypeMetadata>;
	return meta.selection_scope ?? { type: 'segment' };
}

/**
 * The relation this edge stands for, as computed by the backend (`EdgeType::relation_key`), or
 * null when it stands for nothing in particular — one of several interchangeable connections of
 * its kind. Two edges sharing this are one thing drawn twice.
 *
 * Qualified by edge type: a `RequestPath` and a `HubAndSpoke` of the same dependency carry the
 * same backend key but are not the same line.
 */
export function getRelationIdentity(edge: TopologyEdge): string | null {
	const relationKey = (edge as unknown as Record<string, unknown>).relation_key;
	return typeof relationKey === 'string' ? `${edge.edge_type}:${relationKey}` : null;
}

/**
 * Identity of the relation this edge is a segment of, or null when the edge stands alone
 * (segment scope, or a relation-scoped edge whose id is missing). Edges sharing a key are
 * segments of the same thing, so a click on one highlights all of them.
 */
export function getRelationKey(edge: TopologyEdge): string | null {
	const scope = getSelectionScope(edge.edge_type);
	if (scope.type !== 'connected_nodes') return null;
	return getRelationIdentity(edge);
}

/** Whether this edge should be elevated to target an accepting container. */
export function willTargetContainer(edge: TopologyEdge): boolean {
	const vc = getViewConfig(edge);
	return vc.type === 'active' && vc.will_target_container;
}

/** Returns the edge types that should be hidden by default for a given view.
 * Reads from view metadata — edge types with default_visibility = 'hidden'. */
export function getDefaultHiddenEdgeTypes(view: TopologyView): EdgeTypeDiscriminants[] {
	const meta = views.getMetadata(view) as {
		edge_view_configs?: Record<string, EdgeViewConfig>;
	} | null;
	if (!meta?.edge_view_configs) return [];
	return Object.entries(meta.edge_view_configs)
		.filter(([, c]) => c.type === 'active' && c.default_visibility === 'hidden')
		.map(([id]) => id as EdgeTypeDiscriminants);
}

/**
 * The staleness pill for a physical link, or `null` when there is nothing to say.
 *
 * A `PhysicalLink` is drawn from a resolved-adjacency row (`InterfaceNeighborRow`, GH #701 — the
 * `Vec` that replaced the old `Interface.neighbor` scalar), which the server preserves across a
 * scan that read nothing (GH #649) — correctly, since one failed walk must not tear down a
 * network's L2 topology. The consequence is that a link whose evidence has completely disappeared
 * keeps being drawn solid and port-precise while both endpoints stay Current, because their
 * `last_seen_at` only ever answered "was this port observed". `neighbor_seen_at` is the
 * adjacency's own subject; this reads it, on the same window and through the same amber pill a
 * stale host gets, titled to name the neighbour report rather than the port.
 *
 * Judged on the *older* of the rows naming this edge's two endpoints of each other: evidence
 * disappearing from either side is what leaves the link unsupported, and the end that went quiet
 * is the one worth naming in the tooltip. `null` for anything but a `PhysicalLink`, and for one
 * with no matching row.
 *
 * `interfaces`/`neighbours` must come from the rendered topology, not from
 * `queryKeys.interfaces.all` — that cache is written only by the hosts query, so on the topology
 * route it is empty and every link silently read as current. Networks still come from the query
 * cache, which is where `currentElementRenderContext` reads them for the node pills.
 */
export function getLinkEvidenceTag(
	edge: TopologyEdge,
	interfaces: Interface[],
	neighbours: InterfaceNeighborRow[]
): TagProps | null {
	if (edge.edge_type !== 'PhysicalLink') return null;

	const networks = queryClient.getQueryData<Network[]>(queryKeys.networks.all) ?? [];
	const sourceId = edge.source_entity_id;
	const targetId = edge.target_entity_id;

	// The row recording this specific edge's adjacency, from whichever endpoint's perspective it
	// was resolved — a link is recorded on one side, so only one direction is expected to exist,
	// but both are checked in case resolution ever recorded it from each end.
	const rows = neighbours.filter(
		(n) =>
			(n.interface_id === sourceId &&
				n.neighbor.type === 'Interface' &&
				n.neighbor.id === targetId) ||
			(n.interface_id === targetId && n.neighbor.type === 'Interface' && n.neighbor.id === sourceId)
	);

	const networkFor = (row: InterfaceNeighborRow) =>
		networks.find((n) => n.id === interfaces.find((i) => i.id === row.interface_id)?.network_id);

	const stale = rows.filter((row) => neighborEvidenceFreshness(row, networkFor(row)) === 'stale');
	if (stale.length === 0) return null;

	const oldest = stale.reduce((a, b) =>
		(a.neighbor_seen_at ?? '') <= (b.neighbor_seen_at ?? '') ? a : b
	);
	return neighborEvidenceTag(oldest, networkFor(oldest));
}
