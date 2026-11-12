/**
 * Enriched topology helpers.
 *
 * The slim backend `Topology` row carries only the user's grouping
 * `options` (plus `id`/`site_id`). The per-view graph (`nodes`/`edges`)
 * is built on request and returned on the `TopologyData` bundle alongside
 * the entity arrays (`hosts`, `services`, `subnets`, …). This module wraps
 * the row + bundle so consumers can keep reading `topology.nodes` /
 * `topology.hosts` etc. uniformly.
 *
 * Snapshot vs live: the bundle is snapshot-aware (its `nodes`/`edges` are
 * built from the snapshot's closed copies when a snapshot is selected); the
 * entity arrays it carries are the as-of-T set for that snapshot.
 */

import type {
	RenderableTopology,
	Topology,
	TopologyNode,
	TopologyEdge,
	Binding,
	Vlan,
	FilteredOutCounts,
	TopologyHost,
	TopologyService,
	TopologyInterface
} from './types/base';
import type { TopologyView } from './queries';
import type { components } from '#lib/api/schema';
import type { IPAddress, Port, InterfaceNeighborRow } from '#lib/features/hosts/types/base.ts';
import type { Subnet } from '#lib/features/subnets/types/base.ts';
import type { Dependency } from '#lib/features/dependencies/types/base.ts';
import type { Tag } from '#lib/features/tags/types/base.ts';

type TopologyData = components['schemas']['TopologyData'];

export interface EntityBundle {
	hosts: TopologyHost[];
	services: TopologyService[];
	subnets: Subnet[];
	ip_addresses: IPAddress[];
	ports: Port[];
	bindings: Binding[];
	interfaces: TopologyInterface[];
	/**
	 * GH #701: resolved adjacencies, built on request alongside `nodes`/`edges` — see
	 * `TopologyData.neighbours`.
	 *
	 * Required, not optional. These were optional "for a caller that never asked for them", and
	 * that is exactly how the topology tab came to build its bundle without them: the literal
	 * compiled, every interface then classified `Unlinked` for want of a single neighbour row, and
	 * hiding `Unlinked` emptied every host container in L2. Build a bundle with
	 * `entityBundleFrom` rather than by hand.
	 */
	neighbours: InterfaceNeighborRow[];
	/** Server-side filter drop tally — see `RenderableTopology.filtered_out`. */
	filtered_out: FilteredOutCounts;
	dependencies: Dependency[];
	vlans: Vlan[];
	entity_tags: Tag[];
	/** The organization's Application tag group: its tags place entities in applications. */
	application_group_id: string | null;
	/** Per view, the Visibility-hidden entities it would draw — see `TopologyData.hidden_by_tag`. */
	hidden_by_tag?: TopologyData['hidden_by_tag'];
	/** Per-view graph built on request by the backend (keyed by view). */
	nodes?: Partial<Record<TopologyView, TopologyNode[]>>;
	edges?: Partial<Record<TopologyView, TopologyEdge[]>>;
	/** Per view, the entities each node also stands for — see `TopologyData.node_entities`. */
	node_entities?: TopologyData['node_entities'];
}

export const EMPTY_ENTITY_BUNDLE: EntityBundle = {
	hosts: [],
	services: [],
	subnets: [],
	ip_addresses: [],
	ports: [],
	bindings: [],
	interfaces: [],
	neighbours: [],
	filtered_out: {},
	dependencies: [],
	vlans: [],
	entity_tags: [],
	application_group_id: null
};

/**
 * The entity bundle for a `TopologyData` response — the one place that response is mapped.
 *
 * The app tab and the share viewer each used to spell the mapping out field by field, and the two
 * copies drifted: the share viewer passed `neighbours` through and the tab did not. A spread
 * carries every field the backend adds without anyone having to remember to list it; the only
 * real translation is `tags` → `entity_tags`.
 */
export function entityBundleFrom(data: TopologyData): EntityBundle {
	const { tags, neighbours, filtered_out, ...rest } = data;
	return {
		...rest,
		// Absent only on a response from a backend older than the field — `#[serde(default)]`
		// makes them optional in the generated schema, not in what a current server sends.
		neighbours: neighbours ?? [],
		filtered_out: filtered_out ?? {},
		entity_tags: tags,
		application_group_id: data.application_group_id ?? null
	};
}

/**
 * Combine a slim `Topology` row with the entity arrays + built graph from the
 * `TopologyData` bundle.
 *
 * `name` is a UI-side display string supplied by the caller (site
 * name for live view, formatted `taken_at` for snapshots, share name
 * for read-only shared topologies).
 *
 * Filters entity arrays to the topology's site so a multi-site
 * cache doesn't leak into the inspector.
 *
 * `view` selects which per-view node/edge slice (built on request, carried on
 * the bundle) to flatten onto the result. Switching `view` is a pure slice
 * selection (no fetch, no rebuild).
 */
export function toRenderableTopology(
	topology: Topology,
	bundle: EntityBundle,
	name: string,
	view: TopologyView
): RenderableTopology {
	const siteId = topology.site_id;
	const nodes = bundle.nodes?.[view] ?? [];
	const edges = bundle.edges?.[view] ?? [];
	const nodeEntities = bundle.node_entities?.[view] ?? {};
	const hosts = bundle.hosts.filter((h) => h.site_id === siteId);
	const subnets = bundle.subnets.filter((s) => s.site_id === siteId);
	const dependencies = bundle.dependencies.filter((d) => d.site_id === siteId);
	const vlans = bundle.vlans.filter((v) => v.site_id === siteId);
	const hostIds = new Set(hosts.map((h) => h.id));
	const services = bundle.services.filter((s) => hostIds.has(s.host_id));
	const ipAddresses = bundle.ip_addresses.filter((i) => hostIds.has(i.host_id));
	const ports = bundle.ports.filter((p) => hostIds.has(p.host_id));
	const interfaces = bundle.interfaces.filter((i) => hostIds.has(i.host_id));
	// Resolved adjacencies are scoped through the interfaces they belong to, the
	// same way ports/ip_addresses are scoped through hosts above — the row carries
	// no host id to filter against directly.
	const interfaceIds = new Set(interfaces.map((i) => i.id));
	const neighbours = (bundle.neighbours ?? []).filter((n) => interfaceIds.has(n.interface_id));
	const bindings = bundle.bindings.filter((b) => b.site_id === siteId);
	// Tags are org-scoped; keep the ones referenced by entities here, plus tags
	// referenced by grouping rules (ByTag / ByApplication) — those may apply to no
	// entity but still need their name/color to label the group (the backend ships
	// them in the bundle; see TopologyService::augment_grouping_rule_tags).
	const referencedTagIds = new Set<string>();
	for (const h of hosts) for (const t of h.tags ?? []) referencedTagIds.add(t);
	for (const s of services) for (const t of s.tags ?? []) referencedTagIds.add(t);
	for (const s of subnets) for (const t of s.tags ?? []) referencedTagIds.add(t);
	for (const r of topology.options?.request?.element_rules ?? []) {
		if (typeof r.rule === 'object' && 'ByTag' in r.rule)
			for (const t of r.rule.ByTag.tag_ids ?? []) referencedTagIds.add(t);
	}
	for (const rules of Object.values(topology.options?.request?.container_rules ?? {})) {
		for (const r of rules ?? []) {
			if (typeof r.rule === 'object' && 'ByApplication' in r.rule)
				for (const t of r.rule.ByApplication.tag_ids ?? []) referencedTagIds.add(t);
		}
	}
	const entityTags = bundle.entity_tags.filter((t) => referencedTagIds.has(t.id));

	return {
		...topology,
		nodes,
		edges,
		node_entities: nodeEntities,
		hosts,
		services,
		subnets,
		ip_addresses: ipAddresses,
		ports,
		bindings,
		interfaces,
		neighbours,
		// Site-scoped already: the bundle is fetched per site, so its tally needs no
		// filtering the way the entity arrays above do.
		filtered_out: bundle.filtered_out ?? {},
		dependencies,
		hidden_by_tag: bundle.hidden_by_tag?.[view] ?? {},
		vlans,
		entity_tags: entityTags,
		application_group_id: bundle.application_group_id,
		name
	};
}
