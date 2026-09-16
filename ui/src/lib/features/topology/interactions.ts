import { writable, get } from 'svelte/store';
import type { Edge } from '@xyflow/svelte';
import type { Node } from '@xyflow/svelte';
import { edgeTypes, entities, views, serviceDefinitions } from '$lib/shared/stores/metadata';
import { hostDisplayName } from '$lib/features/hosts/host-display-name';
import type { TopologyEdge, TopologyNode, RenderableTopology } from './types/base';
import {
	isDisabledEdge,
	getHighlightBehavior,
	getRelationKey,
	showDirectionality
} from './layout/edge-classification';
import { elevateEdgesToContainers } from './layout/edge-elevation';
import {
	getContainerContents,
	buildEntityNodeIndex,
	entityCollection,
	type EntityNodeIndex
} from './resolvers';
import type { Network } from '$lib/features/networks/types';
import { entityFreshness, type FreshnessSubject } from '$lib/shared/utils/freshness';
import { buildFullParentMap, resolveCollapsedAncestor } from './collapse';
import { formatEntityLabelTitle } from './labels';
import { common_byTag, common_untagged } from '$lib/paraglide/messages';
import type { components } from '$lib/api/schema';

type Entity = components['schemas']['EntityDiscriminants'];

// Shared stores for hover state across all component instances
export const groupHoverState = writable<Map<string, boolean>>(new Map());
export const edgeHoverState = writable<Map<string, boolean>>(new Map());
export const connectedNodeIds = writable<Set<string>>(new Set());
export const isExporting = writable(false);
/**
 * A DOM measurement pass is mounting every node to read its real card height.
 *
 * A store rather than viewer-local state because node components have to see it: a simplified card
 * measures its pinned height instead of its content's, so a measure pass running below the
 * level-of-detail threshold would confirm whatever the last measurement happened to be and hand
 * ELK the result. `shouldSimplify` suspends on it for the same reason `shouldCull` does.
 */
export const isMeasuring = writable(false);

/**
 * Whether the canvas is currently drawing boxes rather than card contents.
 *
 * Decided once in the viewer and published, rather than each node deciding for itself, because the
 * decision now depends on the size of the whole graph — which a node cannot see. It is also simply
 * cheaper: one subscription shared across every node instead of thousands of viewport reads.
 * Containers still read the zoom directly, because their tier depends on their own box.
 */
export const detailSimplified = writable(false);

/**
 * A pipeline run is in flight — layout, measurement, the lot.
 *
 * Surfaced so the viewer can say so. On a large estate a collapse press costs eighteen to
 * twenty-two seconds of mostly-synchronous ELK, during which the canvas sits there looking broken;
 * an operator who cannot tell "working" from "hung" reasonably concludes the latter, which is how
 * the report behind this work described it.
 */
export const isRenderingTopology = writable(false);
export const newNodeIds = writable<Set<string>>(new Set());

// Tag filter stores - nodes/services hidden by tag filter
/**
 * Ids of *nodes* the filters remove from the graph. Node-space: consumed by the pipeline, which
 * drops these from ELK's input, from the DOM, and from the edge graph.
 *
 * Derived from `hiddenEntityIds` by [`updateTagFilter`] — see the resolution pass there.
 */
export const tagHiddenNodeIds = writable<Set<string>>(new Set());

/**
 * Ids of *entities* the filters hide, whatever their type and however they render. Entity-space:
 * consumed by render gates that show entities inline on another node's card, where there is no
 * node to remove.
 *
 * Deliberately not per-entity-type. This was `tagHiddenServiceIds`, which forced every filter
 * pass to special-case Service (`if (entityType === 'Service')`, `if (serviceIsElement)`) and
 * left every other inlined type — Port among them — with no path to being hidden at all.
 */
export const hiddenEntityIds = writable<Set<string>>(new Set());

// Search stores
export const searchHiddenNodeIds = writable<Set<string>>(new Set());
export const searchMatchNodeIds = writable<string[]>([]);
export const searchActiveIndex = writable<number>(0);
export const searchOpen = writable<boolean>(false);
// Map: collapsed container ID → matched element IDs inside it
export const searchMatchContainerMap = writable<Map<string, string[]>>(new Map());
// Navigation-ready list: element IDs for visible matches, container IDs for collapsed matches
export const searchNavigableNodeIds = writable<string[]>([]);

// Special sentinel value for "Untagged" pseudo-tag
export const UNTAGGED_SENTINEL = '__untagged__';

// Hover state for highlighting a set of nodes in the topology. Two modes:
//   tagId string + color string  → tag-scoped hover (animated colored pulse on
//                                   nodes whose tagged entity of `entityType`
//                                   includes `tagId`).
//   tagId null  + color null     → entity-type-wide hover (subdued gray outline
//                                   + dotted-underline on every node whose
//                                   type matches `entityType`).
// Mutex by UX: at most one hover is active at a time, so a single store
// handles both modes.
export interface HoveredTag {
	entityType: import('$lib/api/schema').components['schemas']['EntityDiscriminants'];
	tagId: string | null;
	color: string | null;
}
export const hoveredTag = writable<HoveredTag | null>(null);

// Generic metadata-value hover. Mirrors the tag hover loop but for any
// (entity type, filter type, value id) tuple declared by a view's
// element_config.metadata_filters — Service.Category, Host.Virtualization,
// and future extractor registrations below.
export interface HoveredMetadata {
	entityType: import('$lib/api/schema').components['schemas']['EntityDiscriminants'];
	filterType: string;
	valueId: string;
	color: string;
}
export const hoveredMetadata = writable<HoveredMetadata | null>(null);

// Edge type hover state for highlighting edges of a specific type
export interface HoveredEdgeType {
	edgeTypes: string[];
	color: string;
}
export const hoveredEdgeType = writable<HoveredEdgeType | null>(null);

/**
 * Per node, the handles an edge actually connects to, as `"<type>:<handleId>"`.
 *
 * Node components render only these. Every node declares all eight handles in its `handles` data —
 * that costs nothing and keeps `parseHandles` able to synthesize bounds for a node that has never
 * mounted — but the *DOM* only has to carry the ones SvelteFlow will look up. It reads handle boxes
 * out of the DOM whenever a node's rendered size drifts from its `measured`, and only ever for a
 * handle some edge names, so anything else is eight divs of pure cost. In practice a node uses one
 * or two: the layout is columnar, so edges leave one side and arrive on the other.
 *
 * Written by the pipeline immediately *before* the node store, so a node can never gain an edge
 * ahead of the handle it needs — that ordering is what stops "Couldn't create edge for source
 * handle id", which is how SvelteFlow reports a handle it cannot find.
 *
 * Real edges only. Node components read it with `previewEdgeHandlesByNode` merged over it.
 */
export const edgeHandlesByNode = writable<Map<string, Set<string>>>(new Map());

/**
 * The same map for the dependency editor's preview edges.
 *
 * A new dependency's preview picks its sides by geometry, so it usually names a handle no real edge
 * on that node uses, and the node never rendered it. Kept apart from `edgeHandlesByNode` so the
 * pipeline's map stays a pure function of the real edges. A preview only touches selected nodes, so
 * this adds at most two handles to each of a handful of nodes.
 */
export const previewEdgeHandlesByNode = writable<Map<string, Set<string>>>(new Map());

/**
 * `real` with `extra` unioned in.
 *
 * Only the node ids `extra` names get a new set. Every other entry keeps its original `Set`, and an
 * empty `extra` returns `real` itself, so a node the preview does not touch renders exactly what it
 * did without one.
 */
export function mergeEdgeHandles(
	real: Map<string, Set<string>>,
	extra: Map<string, Set<string>>
): Map<string, Set<string>> {
	if (extra.size === 0) return real;
	const merged = new Map(real);
	for (const [nodeId, handles] of extra) {
		const existing = real.get(nodeId);
		merged.set(nodeId, existing ? new Set([...existing, ...handles]) : handles);
	}
	return merged;
}

/**
 * Build that map from the edges about to be drawn.
 *
 * A null handle means SvelteFlow falls back to the node's first handle of that type, so one still
 * has to exist in the DOM — hence the `Top` default rather than skipping the node.
 */
export function collectEdgeHandles(
	edges: {
		source?: string;
		target?: string;
		sourceHandle?: string | null;
		targetHandle?: string | null;
	}[]
): Map<string, Set<string>> {
	const byNode = new Map<string, Set<string>>();
	const note = (
		nodeId: string | undefined,
		type: 'source' | 'target',
		handle: string | null | undefined
	) => {
		if (!nodeId) return;
		let set = byNode.get(nodeId);
		if (!set) byNode.set(nodeId, (set = new Set()));
		set.add(`${type}:${handle || 'Top'}`);
	};
	for (const e of edges) {
		note(e.source, 'source', e.sourceHandle);
		note(e.target, 'target', e.targetHandle);
	}
	return byNode;
}

// Edge bundle expand/collapse state (transient, not persisted)
export const expandedBundles = writable<Set<string>>(new Set());

// Open ports expand/collapse state per leaf node (transient, not persisted)
export const expandedPortNodeIds = writable<Set<string>>(new Set());

export function toggleBundleExpanded(bundleId: string): void {
	expandedBundles.update((set) => {
		const next = new Set(set);
		if (next.has(bundleId)) {
			next.delete(bundleId);
		} else {
			next.add(bundleId);
		}
		return next;
	});
}

export function toggleExpandedPorts(nodeId: string): void {
	expandedPortNodeIds.update((set) => {
		const next = new Set(set);
		if (next.has(nodeId)) {
			next.delete(nodeId);
		} else {
			next.add(nodeId);
		}
		return next;
	});
}

export function collapseAllBundles(): void {
	if (get(expandedBundles).size > 0) {
		expandedBundles.set(new Set());
	}
}

/** Clear all edge hover state — prevents stale hover from drag interactions */
export function clearEdgeHoverState(): void {
	edgeHoverState.set(new Map());
	groupHoverState.set(new Map());
}

interface TagFilter {
	hidden_host_tag_ids?: string[];
	hidden_service_tag_ids?: string[];
	hidden_subnet_tag_ids?: string[];
}

/**
 * Update hidden nodes/services based on tag filter and category filter settings.
 * - Hosts with hidden tags -> their Element nodes fade out
 * - Services with hidden tags -> hidden from node display (node does NOT fade)
 * - Services in hidden categories -> hidden from node display
 * - Subnets with hidden tags -> Container nodes fade out
 * - UNTAGGED_SENTINEL in hidden arrays -> hide entities with no tags
 */
/**
 * Hide a set of containers and all their descendants (subcontainers + elements).
 * Recursively finds nested subcontainers via parent_container_id, then hides
 * all elements whose container_id matches any hidden container.
 */
function hideContainersAndDescendants(
	containerIds: Set<string>,
	nodes: TopologyNode[],
	hiddenNodeIds: Set<string>
) {
	if (containerIds.size === 0) return;

	// Add containers themselves to hidden
	for (const cid of containerIds) hiddenNodeIds.add(cid);

	// Recursively find subcontainers inside hidden containers
	let changed = true;
	while (changed) {
		changed = false;
		for (const node of nodes) {
			if (node.node_type !== 'Container' || containerIds.has(node.id)) continue;
			const parentId = (node as Record<string, unknown>).parent_container_id as string | undefined;
			if (parentId && containerIds.has(parentId)) {
				containerIds.add(node.id);
				hiddenNodeIds.add(node.id);
				changed = true;
			}
		}
	}

	// Hide all element nodes inside any hidden container
	for (const node of nodes) {
		if (node.node_type === 'Element') {
			const containerId = (node as Record<string, unknown>).container_id as string | undefined;
			if (containerId && containerIds.has(containerId)) {
				hiddenNodeIds.add(node.id);
			}
		}
	}
}

/**
 * Per-(entity, filter) extractor registry. For each declared metadata filter
 * on an entity, returns the filter value id (matching `FilterValue.id` on the
 * backend) for a given entity instance. The updateTagFilter pass below
 * matches these values against the user's hide set to decide whether to
 * hide each entity. Adding a new filter = one line here + a backend impl.
 */
/**
 * Context for extractors whose value isn't intrinsic to the entity. Staleness
 * is the first such filter: it depends on the entity's network staleness
 * window and the current time, so the network has to be threaded in.
 */
export interface FilterValueContext {
	network?: Network;
	/** The graph being filtered, for values that depend on other entities rather than just this one. */
	topology?: RenderableTopology;
}

/**
 * Which interfaces are "linked", per topology: those with a resolved adjacency row of their own
 * (`own`), and those some other interface's row names as its `Interface`-type target (`referenced`)
 * — GH #701 replaced the old single-valued `Interface.neighbor` with a `Vec` of rows on the
 * topology bundle (`neighbours`), so both halves now come from iterating that array rather than a
 * per-interface field.
 *
 * A link is recorded on one side. In the seeded reproduction 728 interfaces carry a neighbour and
 * 721 are the *target* of one, but only 12 have it set both ways — so judging "linked" from an
 * interface's own resolution alone marks the far end of nearly every link as unlinked. With the
 * link-state filter hiding unlinked ports that silently deleted one endpoint of almost every
 * physical link, and `buildFlowEdges` drops an edge whose endpoint is not in the node set: 11 edges
 * drew where there were ~704, which is exactly the count of the 12 bidirectional pairs.
 *
 * Cached per topology object so the pass over `neighbours` runs once rather than per interface.
 */
interface NeighbourIndex {
	/** Interfaces with at least one resolved adjacency row of their own. */
	own: ReadonlySet<string>;
	/** Interfaces some other row names as its `Interface`-type target — see doc above. */
	referenced: ReadonlySet<string>;
}

const neighbourIndexByTopology = new WeakMap<RenderableTopology, NeighbourIndex>();

/** Shared empty index for the no-topology-in-context case, so the extractor allocates nothing. */
const EMPTY_NEIGHBOUR_INDEX: NeighbourIndex = { own: new Set(), referenced: new Set() };

function neighbourIndex(topology: RenderableTopology): NeighbourIndex {
	const cached = neighbourIndexByTopology.get(topology);
	if (cached) return cached;

	const own = new Set<string>();
	const referenced = new Set<string>();
	for (const row of topology.neighbours ?? []) {
		if (row.interface_id) own.add(row.interface_id);
		// Only a port-level resolution makes a specific port linked; `Host` names a device, not a port.
		if (row.neighbor?.type === 'Interface' && row.neighbor.id) referenced.add(row.neighbor.id);
	}
	const index: NeighbourIndex = { own, referenced };
	neighbourIndexByTopology.set(topology, index);
	return index;
}

export type FilterValueExtractor = (entity: unknown, ctx: FilterValueContext) => string | null;
export const FILTER_VALUE_EXTRACTORS: Record<string, Record<string, FilterValueExtractor>> = {
	Service: {
		Category: (s) =>
			serviceDefinitions.getCategory((s as { service_definition: string }).service_definition) ??
			null,
		// Delegates to the same helper the card badge and the node tag use, so
		// the filter cannot disagree with what the user sees marked as stale.
		Staleness: (s, ctx) => entityFreshness(s as FreshnessSubject, ctx.network)
	},
	Host: {
		Virtualization: (h) =>
			(h as { virtualization_metadata?: unknown | null }).virtualization_metadata != null
				? 'Virtualized'
				: 'BareMetal',
		Staleness: (h, ctx) => entityFreshness(h as FreshnessSubject, ctx.network)
	},
	Interface: {
		// Ids match `InterfaceLinkState` on the backend, which is what supplies the filter's
		// values. A partial resolution (`Neighbor::Host` — the remote device known but not the
		// port) counts as linked: it still draws an edge, so hiding it would break the diagram.
		//
		// Linked in *either* direction: a port with a resolved row of its own (`own`), or one no
		// row names but that some other interface's row targets (`referenced`) — a link is
		// recorded on one side, so an interface with no neighbour of its own is still linked if
		// another interface names it. See `neighbourIndex`.
		LinkState: (i, ctx) => {
			const iface = i as { id: string };
			const { own, referenced } = ctx.topology
				? neighbourIndex(ctx.topology)
				: EMPTY_NEIGHBOUR_INDEX;
			return own.has(iface.id) || referenced.has(iface.id) ? 'Linked' : 'Unlinked';
		}
	}
};

/**
 * Which filter values are actually represented in the current topology, keyed
 * `[entityType][filterType]`.
 *
 * A filter whose entities all share one value offers no choice — every toggle
 * either shows everything or hides everything — so the panel drops the group
 * rather than presenting a control that cannot discriminate. Kept generic
 * across all metadata filters rather than special-cased to staleness.
 */
/**
 * The same hidden entities as `hiddenEntityIds`, but keyed by entity type.
 *
 * The flat set cannot say whether what was hidden is drawn *inside* another node's card or is a
 * node in its own right, and those invalidate completely different things: hiding an inline entity
 * resizes the card containing it, while hiding an element entity removes a node and leaves every
 * surviving card exactly as it was. `prepare` needs the distinction to decide whether measured
 * sizes survive a filter change — conflating them re-measured 19,095 nodes on every toggle.
 */
export const hiddenEntityIdsByType = writable<Map<string, Set<string>>>(new Map());

export const presentFilterValues = writable<Record<string, Record<string, string[]>>>({});

function computePresentFilterValues(
	topology: RenderableTopology,
	network: Network | undefined
): Record<string, Record<string, string[]>> {
	const out: Record<string, Record<string, string[]>> = {};
	for (const [entityType, extractors] of Object.entries(FILTER_VALUE_EXTRACTORS)) {
		const collection = entityCollection(topology, entityType);
		if (!collection) continue;
		for (const [filterType, extract] of Object.entries(extractors)) {
			const seen = new Set<string>();
			for (const entity of collection) {
				const value = extract(entity, { network, topology });
				if (value) seen.add(value);
			}
			(out[entityType] ??= {})[filterType] = [...seen];
		}
	}
	return out;
}

/**
 * Union the currently-hidden values into the represented set.
 *
 * Applied to every filter rather than only server-side ones: a value the user has hidden is one they
 * can evidently choose, so offering the toggle is right either way, and keying this on
 * `FilterApplication` would make the panel's behaviour depend on where a filter happens to run.
 */
function withHiddenValues(
	present: Record<string, Record<string, string[]>>,
	hidden: Record<string, Record<string, string[]>> | undefined
): Record<string, Record<string, string[]>> {
	if (!hidden) return present;
	for (const [entityType, byFilter] of Object.entries(hidden)) {
		for (const [filterType, values] of Object.entries(byFilter)) {
			if (!values?.length) continue;
			const existing = (present[entityType] ??= {})[filterType] ?? [];
			(present[entityType] as Record<string, string[]>)[filterType] = [
				...new Set([...existing, ...values])
			];
		}
	}
	return present;
}

/** Collections on Topology indexed by the entity-type key used in filters. */
export function updateTagFilter(
	topology: RenderableTopology | undefined,
	tagFilter: TagFilter | undefined,
	view?: string,
	/** hide_metadata_values[activeView] — a nested map keyed by entity type, then filter type, to hidden value ids. */
	hiddenMetadataValues?: Record<string, Record<string, string[]>>,
	hiddenEntityTypes?: string[],
	/** The topology's network — supplies the staleness window to extractors. */
	network?: Network
) {
	if (!topology) {
		tagHiddenNodeIds.set(new Set());
		hiddenEntityIds.set(new Set());
		hiddenEntityIdsByType.set(new Map());
		presentFilterValues.set({});
		return;
	}

	// Computed before the early-return below: the panel needs this even when no
	// filter is currently applied, to decide which filter groups are worth
	// offering at all.
	//
	// The hidden values are folded back in because a server-side filter removes its entities from
	// the response entirely. Without this, hiding `Unlinked` leaves only `Linked` represented, the
	// group is judged to offer no choice, the panel drops it — and the user has no way to bring
	// 16,000 ports back. They are hidden *because* they exist.
	presentFilterValues.set(
		withHiddenValues(computePresentFilterValues(topology, network), hiddenMetadataValues)
	);

	const hasTagFilter = tagFilter && !isTagFilterEmpty(tagFilter);
	const hasMetadataFilter = hasAnyMetadataFilter(hiddenMetadataValues);
	const hasEntityFilter = hiddenEntityTypes && hiddenEntityTypes.length > 0;

	if (!hasTagFilter && !hasMetadataFilter && !hasEntityFilter) {
		tagHiddenNodeIds.set(new Set());
		hiddenEntityIds.set(new Set());
		hiddenEntityIdsByType.set(new Map());
		return;
	}

	const config = getViewElementConfig(view);

	// Determine filter roles from element config and parent_taggable_entity relationships
	const hostIsContainer = config.container_entity === 'Host';
	const hostIsElement = config.element_entities.some((e) => e.entity_type === 'Host');
	const hostIsParent = config.element_entities.some(
		(e) => entities.getMetadata(e.entity_type)?.parent_taggable_entity === 'Host'
	);
	const hostIsRelevant = hostIsContainer || hostIsElement || hostIsParent;
	const serviceIsElement = config.element_entities.some((e) => e.entity_type === 'Service');
	const serviceIsInline = viewInlinesEntity(config, 'Service');
	const serviceIsVisible = serviceIsElement || serviceIsInline;

	const hiddenHostTagIds = tagFilter?.hidden_host_tag_ids ?? [];
	const hiddenServiceTagIds = tagFilter?.hidden_service_tag_ids ?? [];
	const hiddenSubnetTagIds = tagFilter?.hidden_subnet_tag_ids ?? [];

	const hideUntaggedHosts = hiddenHostTagIds.includes(UNTAGGED_SENTINEL);
	const hideUntaggedServices = hiddenServiceTagIds.includes(UNTAGGED_SENTINEL);
	const hideUntaggedSubnets = hiddenSubnetTagIds.includes(UNTAGGED_SENTINEL);

	const hiddenNodeIds = new Set<string>();
	// Entity-space hits, plus the same ids grouped by type. The grouping is what lets the single
	// resolution pass at the end reach nodes that represent an entity *indirectly* — a node whose
	// own id belongs to a different entity — without any pass here naming an entity type.
	const hiddenEntities = new Set<string>();
	const hiddenByType = new Map<string, Set<string>>();
	const noteHidden = (entityType: string, entityId: string) => {
		hiddenEntities.add(entityId);
		let forType = hiddenByType.get(entityType);
		if (!forType) {
			forType = new Set<string>();
			hiddenByType.set(entityType, forType);
		}
		forType.add(entityId);
	};
	const index = buildEntityNodeIndex(topology.nodes);

	// Host filtering: runs when Host is container, element, or parent of an element entity
	if (hostIsRelevant) {
		const hiddenHostIds = new Set<string>();
		for (const host of topology.hosts) {
			const isUntagged = host.tags.length === 0;
			const hostHasHiddenTag = host.tags.some((t) => hiddenHostTagIds.includes(t));
			if (hostHasHiddenTag || (isUntagged && hideUntaggedHosts)) {
				hiddenHostIds.add(host.id);
				// Hide element nodes that represent this host entity (VMs in Workloads)
				const nodeIds = index.hostIdToNodes.get(host.id);
				nodeIds?.forEach((id) => hiddenNodeIds.add(id));
			}
		}

		// When Host is the container (L2, Workloads): hide container nodes + descendants
		if (hostIsContainer && hiddenHostIds.size > 0) {
			const hiddenContainerIds = new Set<string>();
			for (const [hostId, containerIds] of index.hostIdToContainerIds) {
				if (hiddenHostIds.has(hostId)) {
					containerIds.forEach((cid) => hiddenContainerIds.add(cid));
				}
			}
			hideContainersAndDescendants(hiddenContainerIds, topology.nodes, hiddenNodeIds);
		}
	}

	// Service filtering: tag-based only. Category / other metadata filters
	// come through the generic metadata-filter pass below.
	//
	// Records entity-space only; whether a hidden service also removes a node is settled by the
	// resolution pass at the end, which is what the `serviceIsElement` gate used to decide here.
	if (serviceIsVisible) {
		for (const service of topology.services) {
			const isUntagged = service.tags.length === 0;
			const serviceHasHiddenTag = service.tags.some((t) => hiddenServiceTagIds.includes(t));
			if (serviceHasHiddenTag || (isUntagged && hideUntaggedServices)) {
				noteHidden('Service', service.id);
			}
		}
	}

	// Subnet filtering: hide container nodes and their child elements
	if (hiddenSubnetTagIds.length > 0) {
		const hiddenContainerIds = new Set<string>();
		for (const subnet of topology.subnets) {
			const isUntagged = subnet.tags.length === 0;
			const subnetHasHiddenTag = subnet.tags.some((t) => hiddenSubnetTagIds.includes(t));
			if (subnetHasHiddenTag || (isUntagged && hideUntaggedSubnets)) {
				hiddenNodeIds.add(subnet.id);
				hiddenContainerIds.add(subnet.id);
			}
		}
		hideContainersAndDescendants(hiddenContainerIds, topology.nodes, hiddenNodeIds);
	}

	// Generic metadata-filter hide: for each (entity, filter) in the user's
	// hide-set, look up the extractor, iterate the entity's collection, and
	// add matching IDs to the hidden sets. Orphaned hide entries (value ids
	// no longer declared by the current filter) silently match nothing.
	if (hasMetadataFilter && hiddenMetadataValues) {
		for (const [entityType, byFilter] of Object.entries(hiddenMetadataValues)) {
			const extractors = FILTER_VALUE_EXTRACTORS[entityType];
			if (!extractors) continue;
			const collection = entityCollection(topology, entityType);
			if (!collection) continue;
			for (const entity of collection) {
				for (const [filterType, hiddenValues] of Object.entries(byFilter)) {
					if (!hiddenValues.length) continue;
					const extract = extractors[filterType];
					if (!extract) continue;
					const value = extract(entity, { network, topology });
					if (value && hiddenValues.includes(value)) {
						noteHidden(entityType, entity.id);
					}
				}
			}
		}
	}

	// Entity-type hide: extend hidden sets for every entity type the user
	// toggled off via the eye icon. Matches on element_type directly to
	// avoid the over-broad hostIdToNodes lookup (which would include every
	// service/port under the host).
	if (hasEntityFilter) {
		const entityTypes = new Set(hiddenEntityTypes);
		for (const node of topology.nodes) {
			if (node.node_type !== 'Element') continue;
			if (entityTypes.has(node.element_type)) {
				hiddenNodeIds.add(node.id);
			}
		}
		// Every entity of a hidden type, so the types that render only inline are covered too.
		// Previously this named Service and left Port to a separate render gate.
		for (const entityType of entityTypes) {
			for (const entity of entityCollection(topology, entityType) ?? []) {
				noteHidden(entityType, entity.id);
			}
		}
	}

	// Entity-space -> node-space, once, generically. A node stands for a hidden entity in one of
	// two ways: it *is* that entity, so its own id matches; or it is an element of that type
	// related to it, for the types whose node id is some other entity's. Every pass above records
	// entity ids and leaves this to decide what that means for the graph.
	for (const node of topology.nodes) {
		if (hiddenEntities.has(node.id)) {
			hiddenNodeIds.add(node.id);
			continue;
		}
		if (node.node_type !== 'Element') continue;
		const relatedId = relatedEntityId(node, node.element_type);
		if (relatedId && hiddenByType.get(node.element_type)?.has(relatedId)) {
			hiddenNodeIds.add(node.id);
		}
	}

	tagHiddenNodeIds.set(hiddenNodeIds);
	hiddenEntityIds.set(hiddenEntities);
	hiddenEntityIdsByType.set(hiddenByType);
}

function isTagFilterEmpty(filter: {
	hidden_host_tag_ids?: string[];
	hidden_service_tag_ids?: string[];
	hidden_subnet_tag_ids?: string[];
}): boolean {
	return (
		(filter.hidden_host_tag_ids?.length ?? 0) === 0 &&
		(filter.hidden_service_tag_ids?.length ?? 0) === 0 &&
		(filter.hidden_subnet_tag_ids?.length ?? 0) === 0
	);
}

function hasAnyMetadataFilter(m: Record<string, Record<string, string[]>> | undefined): boolean {
	if (!m) return false;
	for (const byFilter of Object.values(m)) {
		for (const values of Object.values(byFilter)) {
			if (values.length > 0) return true;
		}
	}
	return false;
}

/** One control currently hiding something, named the way the options panel names it. */
export interface ActiveFilterSummary {
	/** The control's own label: "By link", or the entity's plural name for an eye toggle. */
	label: string;
	/** Value labels it is hiding. Empty for a control that hides a whole entity type. */
	values: string[];
	/** Entities it removed from this view, where that is knowable. */
	count?: number;
}

type MetadataFilterDef = {
	filter_type: string;
	label: string;
	values: Array<{ id: string; label: string }>;
};

/** The metadata filters a view declares, keyed by entity type — from the generated view fixture. */
function declaredMetadataFilters(view: string): Record<string, MetadataFilterDef[]> {
	const meta = views.getMetadata(view) as {
		element_config?: { metadata_filters?: Record<string, MetadataFilterDef[]> };
	} | null;
	return meta?.element_config?.metadata_filters ?? {};
}

/**
 * Which controls are hiding something in `view`, and how much.
 *
 * The point of it is an emptied view that can say what emptied it. A server-applied filter removes
 * its entities from the response entirely, so the browser cannot count what it never received —
 * `topology.filtered_out` is the backend's tally of exactly that, and the client-side hidden sets
 * cover the rest.
 *
 * Edge-type hides are deliberately absent: they remove edges, never nodes, so they cannot be the
 * reason a view has nothing in it.
 */
export function activeViewFilters(
	view: string,
	topology: RenderableTopology | undefined,
	hiddenMetadataValues: Record<string, Record<string, string[]>> | undefined,
	hiddenEntityTypes: string[] | undefined,
	tagFilter: TagFilter | undefined,
	network?: Network
): ActiveFilterSummary[] {
	const summaries: ActiveFilterSummary[] = [];
	if (!topology) return summaries;

	const declared = declaredMetadataFilters(view);
	const serverDropped = topology.filtered_out ?? {};

	for (const [entityType, byFilter] of Object.entries(hiddenMetadataValues ?? {})) {
		for (const [filterType, hiddenValues] of Object.entries(byFilter)) {
			if (!hiddenValues.length) continue;
			const def = declared[entityType]?.find((f) => f.filter_type === filterType);
			// A hide entry for a filter this view no longer declares matches nothing and is not
			// hiding anything, so it has no business being named as a cause.
			if (!def) continue;

			// Entities this filter removed: those dropped before the response was built, plus
			// those still in the bundle that the browser is hiding. Counted per filter with the
			// same extractor the hide pass uses, so two filters on one entity type each report
			// their own share rather than both claiming the total.
			let count = serverDropped[entityType]?.[filterType] ?? 0;
			const extract = FILTER_VALUE_EXTRACTORS[entityType]?.[filterType];
			if (extract) {
				for (const entity of entityCollection(topology, entityType) ?? []) {
					const value = extract(entity, { network, topology });
					if (value && hiddenValues.includes(value)) count++;
				}
			}

			summaries.push({
				label: def.label,
				values: hiddenValues.map((id) => def.values.find((v) => v.id === id)?.label ?? id),
				count
			});
		}
	}

	for (const entityType of hiddenEntityTypes ?? []) {
		summaries.push({
			label: formatEntityLabelTitle([entityType as Entity]),
			values: [],
			count: entityCollection(topology, entityType)?.length
		});
	}

	if (tagFilter && !isTagFilterEmpty(tagFilter)) {
		const hiddenTagIds = [
			...(tagFilter.hidden_host_tag_ids ?? []),
			...(tagFilter.hidden_service_tag_ids ?? []),
			...(tagFilter.hidden_subnet_tag_ids ?? [])
		];
		summaries.push({
			label: common_byTag(),
			values: hiddenTagIds.map(
				(id) =>
					topology.entity_tags.find((t) => t.id === id)?.name ??
					(id === UNTAGGED_SENTINEL ? common_untagged() : id)
			)
		});
	}

	return summaries;
}

/** For non-Service metadata filters: check that an Element node represents
 *  the same entity instance as the filter target. Keeps the filter match
 *  narrowly scoped instead of leaking to unrelated elements sharing a host_id. */
/**
 * The id of the `entityType` entity this node hangs off, when that is something other than the
 * node itself. `undefined` for a node that simply *is* its entity — those are matched by id, so
 * they need no mapping.
 *
 * Returns the id rather than testing one, so a caller can resolve in a single pass over the
 * nodes instead of once per (node, hidden entity) pair.
 */
function relatedEntityId(node: TopologyNode, entityType: string): string | undefined {
	if (node.node_type !== 'Element') return undefined;
	const data = node as unknown as Record<string, string | undefined>;
	switch (entityType) {
		case 'Host':
			return data.host_id;
		case 'IPAddress':
			return data.ip_address_id;
		case 'Interface':
			return data.interface_id;
		default:
			return undefined;
	}
}

/**
 * Node ids a click on an edge should light up.
 *
 * Each edge type declares its own scope (`selection_scope` in edge-type metadata). An edge
 * that is one segment of a wider relation — a dependency's chain, a host's addresses, a
 * container's addresses, a runtime's bridges — lights up every segment of that relation;
 * anything else lights up its own two endpoints. No edge type is named here.
 */
export function nodesConnectedByEdge(
	edge: TopologyEdge,
	candidateEdges: TopologyEdge[]
): Set<string> {
	const connected = new Set<string>();

	// The clicked edge's own endpoints always count — and for an aggregated edge those are
	// the collapsed containers standing in for what is inside them.
	connected.add(edge.source as string);
	connected.add(edge.target as string);

	const relationKey = getRelationKey(edge);
	if (relationKey) {
		for (const other of candidateEdges) {
			if (getRelationKey(other) !== relationKey) continue;
			connected.add(other.source as string);
			connected.add(other.target as string);
		}
	}

	return connected;
}

/**
 * Add container highlights: when a container is in the connected set,
 * also include its element contents and subcontainers so they highlight.
 * When a container has connected elements inside it, highlight that container.
 * Uses topology data to find elements hidden by collapsed containers.
 */
function addContainerHighlights(
	connected: Set<string>,
	allNodes: Node[],
	topology?: RenderableTopology
) {
	const topoNodes = topology?.nodes ?? allNodes.map((n) => n.data as TopologyNode);

	for (const nd of topoNodes) {
		if (nd.node_type !== 'Container') continue;

		const contents = getContainerContents(nd.id, topoNodes);

		if (connected.has(nd.id)) {
			// Container is connected — include its elements and subcontainers
			for (const id of contents.elementNodeIds) connected.add(id);
			for (const id of contents.subcontainerIds) connected.add(id);
		} else {
			// Check if this container has any connected elements inside it
			for (const elementId of contents.elementNodeIds) {
				if (connected.has(elementId)) {
					connected.add(nd.id);
					break;
				}
			}
		}
	}
}

/**
 * Update connected nodes when a node or edge is selected
 * @param topology - Optional topology data for direct lookups (used in share views where cache is empty)
 * @param multiSelectedNodes - Optional array of multi-selected nodes
 */
export function updateConnectedNodes(
	selectedNode: Node | null,
	selectedEdge: Edge | null,
	allEdges: Edge[],
	allNodes: Node[],
	topology?: RenderableTopology,
	multiSelectedNodes?: Node[],
	hiddenEdgeTypes?: string[]
) {
	const connected = new Set<string>();

	// If multiple nodes are selected
	if (multiSelectedNodes && multiSelectedNodes.length >= 2) {
		const rawEdges = topology?.edges ?? [];
		const topologyNodes = topology?.nodes ?? [];
		const elevatedEdges = elevateEdgesToContainers(rawEdges, topologyNodes);
		for (const node of multiSelectedNodes) {
			connected.add(node.id);
			// Add direct neighbors of each selected node (using elevated edges)
			for (const edge of elevatedEdges) {
				if (isDisabledEdge(edge)) continue;
				const behavior = getHighlightBehavior(edge);
				if (behavior === 'never') continue;
				if (behavior === 'when_visible' && hiddenEdgeTypes?.includes(edge.edge_type)) continue;
				if (edge.source === node.id) {
					connected.add(edge.target);
				}
				if (edge.target === node.id) {
					connected.add(edge.source);
				}
			}
		}
		// Expand connected containers to include their element contents
		const topoNodes = topology?.nodes ?? allNodes.map((n) => n.data as TopologyNode);
		for (const nd of topoNodes) {
			if (nd.node_type !== 'Container' || !connected.has(nd.id)) continue;
			const contents = getContainerContents(nd.id, topoNodes);
			for (const id of contents.elementNodeIds) connected.add(id);
			for (const id of contents.subcontainerIds) connected.add(id);
		}
		connectedNodeIds.set(connected);
		return;
	}

	// If a node is selected
	if (selectedNode) {
		connected.add(selectedNode.id);
		const nodeData = selectedNode.data as TopologyNode;

		if (nodeData.node_type == 'Container') {
			const topoNodes2 = topology?.nodes ?? allNodes.map((n) => n.data as TopologyNode);
			const contents = getContainerContents(nodeData.id, topoNodes2);
			for (const id of contents.elementNodeIds) connected.add(id);
			for (const id of contents.subcontainerIds) connected.add(id);
		}

		// Use elevated edges so absorbing containers appear in the connected set.
		// Elevation rewrites edge endpoints from elements to their outermost
		// absorbing container, which is exactly what highlighting needs.
		const rawEdges = topology?.edges ?? [];
		const topologyNodes = topology?.nodes ?? [];
		const elevatedEdges = elevateEdgesToContainers(rawEdges, topologyNodes);

		for (const edge of elevatedEdges) {
			// Skip disabled edges entirely
			if (isDisabledEdge(edge)) continue;

			// Use highlight_behavior to decide if this edge contributes
			const behavior = getHighlightBehavior(edge);
			if (behavior === 'never') continue;
			if (behavior === 'when_visible' && hiddenEdgeTypes?.includes(edge.edge_type)) continue;

			// Add directly connected nodes. Container-runtime edges land on the bridge box
			// itself (they target containers), and addContainerHighlights below expands that
			// box to the container cards inside it.
			if (edge.source === selectedNode.id) {
				connected.add(edge.target);
			}
			if (edge.target === selectedNode.id) {
				connected.add(edge.source);
			}
		}

		addContainerHighlights(connected, allNodes, topology);

		connectedNodeIds.set(connected);
		return;
	}

	// If an edge is selected (group OR non-group)
	if (selectedEdge) {
		const edgeData = selectedEdge.data as TopologyEdge | undefined;
		if (!edgeData) {
			connectedNodeIds.set(new Set());
			return;
		}

		// A bundle is drawn as one line standing for several edges, so a click on it counts as
		// a click on each of them.
		const anyData = selectedEdge.data as Record<string, unknown> | undefined;
		const clickedEdges =
			anyData?.isBundle && Array.isArray(anyData.bundleEdges)
				? (anyData.bundleEdges as TopologyEdge[])
				: [edgeData];

		// Elevated topology edges, matching the endpoints the selected edge itself carries —
		// and unlike the rendered flow edges, they still include every member of a bundle.
		const rawEdges = topology?.edges ?? [];
		const candidateEdges = rawEdges.length
			? elevateEdgesToContainers(rawEdges, topology?.nodes ?? [])
			: allEdges
					.map((e) => e.data as TopologyEdge | undefined)
					.filter((e): e is TopologyEdge => !!e);

		for (const clicked of clickedEdges) {
			for (const id of nodesConnectedByEdge(clicked, candidateEdges)) connected.add(id);
		}

		addContainerHighlights(connected, allNodes, topology);
		connectedNodeIds.set(connected);
		return;
	}

	// Nothing selected - clear
	connectedNodeIds.set(new Set());
}

/**
 * Set edge hover state explicitly — avoids toggle desync when enter/leave events fire asymmetrically
 */
export function setEdgeHover(edge: Edge, hovered: boolean, allEdges: Edge[]) {
	const edgeData = edge.data as TopologyEdge | undefined;
	if (!edgeData) return;
	const edgeTypeMetadata = edgeTypes.getMetadata(edgeData.edge_type);

	// Set individual edge hover state
	edgeHoverState.update((state) => {
		const newState = new Map(state);
		newState.set(edge.id, hovered);
		return newState;
	});

	// For group edges, update group hover state
	if (edgeTypeMetadata.is_dependency_edge && 'dependency_id' in edgeData) {
		const dependencyId = edgeData.dependency_id as string;

		groupHoverState.update((state) => {
			const newState = new Map(state);

			// Get the UPDATED edge hover states (after we just toggled this edge)
			const updatedEdgeStates = get(edgeHoverState);
			let anyEdgeInGroupHovered = false;

			// Check if ANY edge in this dependency is hovered
			for (const e of allEdges) {
				const eData = e.data as TopologyEdge | undefined;
				if (!eData) continue;
				const eMetadata = edgeTypes.getMetadata(eData.edge_type);
				if (
					eMetadata.is_dependency_edge &&
					'dependency_id' in eData &&
					eData.dependency_id === dependencyId
				) {
					const eIsHovered = updatedEdgeStates.get(e.id) || false;
					if (eIsHovered) {
						anyEdgeInGroupHovered = true;
						break;
					}
				}
			}

			newState.set(dependencyId, anyEdgeInGroupHovered);
			return newState;
		});
	}
}

export interface EdgeDisplayState {
	shouldShowFull: boolean;
	shouldAnimate: boolean;
	isEndpointSearchHidden: boolean;
	isEndpointTagHidden: boolean;
}

/**
 * Get display state for an edge based on hover, selection, search, and tag filters.
 * Single source of truth for edge visual state computation.
 */
export function getEdgeDisplayState(
	edge: Edge,
	selectedNode: Node | null,
	selectedEdge: Edge | null,
	searchHidden?: Set<string>,
	tagHidden?: Set<string>
): EdgeDisplayState {
	const edgeData = edge.data as TopologyEdge | undefined;
	if (!edgeData) {
		return {
			shouldShowFull: false,
			shouldAnimate: false,
			isEndpointSearchHidden: false,
			isEndpointTagHidden: false
		};
	}

	const source = edgeData.source as string;
	const target = edgeData.target as string;

	// Centralized endpoint-hidden checks
	const isEndpointSearchHidden = searchHidden
		? searchHidden.has(source) || searchHidden.has(target)
		: false;
	const isEndpointTagHidden = tagHidden ? tagHidden.has(source) || tagHidden.has(target) : false;

	const edgeTypeMetadata = edgeTypes.getMetadata(edgeData.edge_type);
	const isGroupEdge = edgeTypeMetadata.is_dependency_edge;

	let shouldShowFull: boolean;
	let shouldAnimate: boolean;

	// Check if this edge is hovered
	const isThisEdgeHovered = get(edgeHoverState).get(edge.id) || false;

	// Check if this edge is selected
	const isThisEdgeSelected = selectedEdge?.id === edge.id;

	// For group edges, check group hover/selection state
	if (isGroupEdge && 'dependency_id' in edgeData) {
		const dependencyId = edgeData.dependency_id as string;
		const isGroupHovered = get(groupHoverState).get(dependencyId) || false;

		// Check if any edge in this group is selected
		let isGroupSelected = false;
		if (selectedEdge) {
			const selectedEdgeData = selectedEdge.data as TopologyEdge | undefined;
			if (selectedEdgeData) {
				const selectedMetadata = edgeTypes.getMetadata(selectedEdgeData.edge_type);
				if (selectedMetadata.is_dependency_edge && 'dependency_id' in selectedEdgeData) {
					isGroupSelected = selectedEdgeData.dependency_id === dependencyId;
				}
			}
		}

		// Check if connected node is selected
		const isConnectedNodeSelected =
			selectedNode && (edgeData.source === selectedNode.id || edgeData.target === selectedNode.id);

		// Should show full if: group hovered, group selected, or connected node selected
		shouldShowFull = isGroupHovered || isGroupSelected || !!isConnectedNodeSelected;

		// Animate only if view config enables directionality
		shouldAnimate = showDirectionality(edgeData)
			? isGroupHovered || isGroupSelected || !!isConnectedNodeSelected
			: false;
	} else {
		// Non-group edges: show full if hovered, selected, or connected node selected
		const isConnectedNodeSelected =
			selectedNode && (edgeData.source === selectedNode.id || edgeData.target === selectedNode.id);

		shouldShowFull = isThisEdgeHovered || isThisEdgeSelected || !!isConnectedNodeSelected;

		// Animate only if view config enables directionality
		shouldAnimate = showDirectionality(edgeData)
			? isThisEdgeHovered || isThisEdgeSelected || !!isConnectedNodeSelected
			: false;
	}

	return { shouldShowFull, shouldAnimate, isEndpointSearchHidden, isEndpointTagHidden };
}

export interface ViewElementEntityConfig {
	entity_type: string;
	inline_entities: string[];
}

export interface ViewElementConfig {
	container_entity: string | null;
	element_entities: ViewElementEntityConfig[];
}

/** Read ViewElementConfig from views metadata store, with safe defaults */
export function getViewElementConfig(view?: string): ViewElementConfig {
	if (!view) return { container_entity: null, element_entities: [] };
	const meta = views.getMetadata(view) as {
		element_config?: ViewElementConfig;
	} | null;
	return {
		container_entity: meta?.element_config?.container_entity ?? null,
		element_entities: meta?.element_config?.element_entities ?? []
	};
}

/** True if any element entity in the view inlines the given entity type. */
export function viewInlinesEntity(config: ViewElementConfig, entityType: string): boolean {
	return config.element_entities.some((e) => e.inline_entities.includes(entityType));
}

/** Entity-type names of the view's element entities (no inline detail). */
export function elementEntityTypes(config: ViewElementConfig): string[] {
	return config.element_entities.map((e) => e.entity_type);
}

interface EntityResolution {
	elementNodeIds: string[];
	containerNodeIds: string[];
}

/** Index map for looking up element nodes by entity type */
const ENTITY_ELEMENT_INDEX: Record<string, (idx: EntityNodeIndex) => Map<string, string[]>> = {
	Host: (idx) => idx.hostIdToNodes,
	IPAddress: (idx) => idx.ipAddressIdToNodes,
	Service: (idx) => idx.serviceIdToNodes,
	Interface: (idx) => idx.interfaceIdToNodes
};

/**
 * Resolve an entity to topology node IDs based on the current view's element config.
 * Returns element and container node IDs separately so callers can decide behavior
 * (search: add both to matches; tag filter: hide containers + descendants).
 */
function resolveEntityToNodes(
	entityType: string,
	entityId: string,
	config: ViewElementConfig,
	index: EntityNodeIndex,
	topology: RenderableTopology
): EntityResolution {
	const elementNodeIds: string[] = [];
	const containerNodeIds: string[] = [];

	const isDirectElement = config.element_entities.some((e) => e.entity_type === entityType);

	// 1. Direct element: entity type is an element in this view
	if (isDirectElement) {
		const getIndex = ENTITY_ELEMENT_INDEX[entityType];
		if (getIndex) {
			const nodeIds = getIndex(index).get(entityId);
			if (nodeIds) elementNodeIds.push(...nodeIds);
		}
	}

	// 2. Container: entity type is the container in this view
	if (entityType === config.container_entity) {
		if (entityType === 'Host') {
			const cids = index.hostIdToContainerIds.get(entityId);
			if (cids) containerNodeIds.push(...cids);
		} else {
			// Subnet containers: entity ID is the container node ID
			containerNodeIds.push(entityId);
		}
	}

	// 3. Parent propagation: entity type is parent_taggable_entity of an element entity
	const isParent = config.element_entities.some(
		(e) => entities.getMetadata(e.entity_type)?.parent_taggable_entity === entityType
	);
	if (isParent && entityType !== config.container_entity && !isDirectElement) {
		// hostIdToNodes maps host_id to all element nodes with that host_id,
		// regardless of element type — works across views
		const nodeIds = index.hostIdToNodes.get(entityId);
		if (nodeIds) elementNodeIds.push(...nodeIds);
	}

	// 4. Inline entity: resolve through bindings to element nodes.
	// Only Service is wired through here today; Port inline filtering would
	// need a similar branch if product adds port tag filtering later.
	if (viewInlinesEntity(config, entityType) && entityType === 'Service') {
		const service = topology.services.find((s) => s.id === entityId);
		if (service) {
			for (const binding of service.bindings) {
				if (binding.ip_address_id) {
					const nodeIds = index.ipAddressIdToNodes.get(binding.ip_address_id);
					if (nodeIds) elementNodeIds.push(...nodeIds);
				}
			}
		}
	}

	return { elementNodeIds, containerNodeIds };
}

/**
 * Update search filter: find nodes matching query, set non-matching nodes to fade.
 * Uses resolveEntityToNodes for view-aware entity→node resolution.
 * Searches hosts (name/hostname), ip addresses (ip_address/name), services (name),
 * subnets (name/cidr), and tags (name matched to entities).
 */
export function updateSearchFilter(
	topology: RenderableTopology | undefined,
	query: string,
	view?: string
) {
	if (!topology || !query.trim()) {
		searchHiddenNodeIds.set(new Set());
		searchMatchNodeIds.set([]);
		searchActiveIndex.set(0);
		return;
	}

	const q = query.toLowerCase().trim();
	const index = buildEntityNodeIndex(topology.nodes);
	const config = getViewElementConfig(view);
	const matchingSet = new Set<string>();

	/** Resolve an entity match to visible nodes and add them to matchingSet */
	const addResolved = (entityType: string, entityId: string) => {
		const { elementNodeIds, containerNodeIds } = resolveEntityToNodes(
			entityType,
			entityId,
			config,
			index,
			topology
		);
		for (const id of elementNodeIds) matchingSet.add(id);
		for (const id of containerNodeIds) matchingSet.add(id);
	};

	// allNodeIds = all element nodes + container nodes
	const allNodeIds = new Set<string>([...index.allElementNodeIds, ...index.allContainerNodeIds]);

	// Search hosts
	for (const host of topology.hosts) {
		// The title, not the stored name — a host labelled `core-sw-01` on the canvas because that
		// is its sysName has to be findable by typing what is drawn on it. Checking `name` as well
		// would be redundant: it is the ladder's first rung, so a named host's title *is* its name.
		const nameMatch = hostDisplayName(host).toLowerCase().includes(q);
		const hostnameMatch = host.hostname?.toLowerCase().includes(q) ?? false;
		if (nameMatch || hostnameMatch) {
			addResolved('Host', host.id);
		}
	}

	// Search IP addresses
	for (const ipAddr of topology.ip_addresses) {
		const ipMatch = ipAddr.ip_address?.toLowerCase().includes(q) ?? false;
		const nameMatch = ipAddr.name?.toLowerCase().includes(q) ?? false;
		if (ipMatch || nameMatch) {
			addResolved('IPAddress', ipAddr.id);
		}
	}

	// Search services
	for (const service of topology.services) {
		if (service.name.toLowerCase().includes(q)) {
			addResolved('Service', service.id);
		}
	}

	// Search subnets
	for (const subnet of topology.subnets) {
		const nameMatch = subnet.name.toLowerCase().includes(q);
		const cidrMatch = subnet.cidr.toLowerCase().includes(q);
		if (nameMatch || cidrMatch) {
			addResolved('Subnet', subnet.id);
		}
	}

	// Search interfaces
	if (topology.interfaces) {
		for (const iface of topology.interfaces) {
			const aliasMatch = iface.if_alias?.toLowerCase().includes(q) ?? false;
			const nameMatch = iface.if_name?.toLowerCase().includes(q) ?? false;
			if (aliasMatch || nameMatch) {
				addResolved('Interface', iface.id);
			}
		}
	}

	// Search tags -> match entities with matching tag names
	const entityTags = topology.entity_tags ?? [];
	for (const host of topology.hosts) {
		for (const tagId of host.tags) {
			const tag = entityTags.find((t) => t.id === tagId);
			if (tag && tag.name.toLowerCase().includes(q)) {
				addResolved('Host', host.id);
				break;
			}
		}
	}

	for (const service of topology.services) {
		for (const tagId of service.tags) {
			const tag = entityTags.find((t) => t.id === tagId);
			if (tag && tag.name.toLowerCase().includes(q)) {
				addResolved('Service', service.id);
				break;
			}
		}
	}

	for (const subnet of topology.subnets) {
		for (const tagId of subnet.tags) {
			const tag = entityTags.find((t) => t.id === tagId);
			if (tag && tag.name.toLowerCase().includes(q)) {
				addResolved('Subnet', subnet.id);
				break;
			}
		}
	}

	// Hidden = all nodes NOT in the matching set
	const hiddenIds = new Set<string>();
	for (const nodeId of allNodeIds) {
		if (!matchingSet.has(nodeId)) {
			hiddenIds.add(nodeId);
		}
	}

	searchHiddenNodeIds.set(hiddenIds);
	searchMatchNodeIds.set([...matchingSet]);
	searchActiveIndex.set(0);
}

/**
 * Recompute navigation list by resolving matches against collapse state.
 * Replaces element IDs inside collapsed containers with their outermost collapsed ancestor.
 */
export function recomputeSearchNavigation(
	matchNodeIds: string[],
	collapsed: Set<string>,
	nodes: TopologyNode[]
) {
	if (matchNodeIds.length === 0 || collapsed.size === 0) {
		searchMatchContainerMap.set(new Map());
		searchNavigableNodeIds.set(matchNodeIds);
		return;
	}

	const parentMap = buildFullParentMap(nodes);
	const containerMap = new Map<string, string[]>();
	const navigable: string[] = [];
	const seenContainers = new Set<string>();

	for (const id of matchNodeIds) {
		const ancestor = resolveCollapsedAncestor(id, collapsed, parentMap);
		if (ancestor) {
			if (!containerMap.has(ancestor)) containerMap.set(ancestor, []);
			containerMap.get(ancestor)!.push(id);
			if (!seenContainers.has(ancestor)) {
				seenContainers.add(ancestor);
				navigable.push(ancestor);
			}
		} else {
			navigable.push(id);
		}
	}

	searchMatchContainerMap.set(containerMap);
	searchNavigableNodeIds.set(navigable);

	// Clamp active index if navigable list shrank
	const currentIndex = get(searchActiveIndex);
	if (navigable.length > 0 && currentIndex >= navigable.length) {
		searchActiveIndex.set(navigable.length - 1);
	}
}

/**
 * Clear all search state.
 */
export function clearSearch() {
	searchHiddenNodeIds.set(new Set());
	searchMatchNodeIds.set([]);
	searchActiveIndex.set(0);
	searchOpen.set(false);
	searchMatchContainerMap.set(new Map());
	searchNavigableNodeIds.set([]);
}
