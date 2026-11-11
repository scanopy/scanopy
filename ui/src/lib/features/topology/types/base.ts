import type { components } from '#lib/api/schema';
import type { Service } from '#lib/features/services/types/base.ts';
import type {
	Host,
	IPAddress,
	Port,
	InterfaceNeighborRow
} from '#lib/features/hosts/types/base.ts';
import type { Subnet } from '#lib/features/subnets/types/base.ts';
import type { Dependency } from '#lib/features/dependencies/types/base.ts';
import type { Tag } from '#lib/features/tags/types/base.ts';

// Re-export generated types
export type Topology = components['schemas']['Topology'];
export type TopologyBase = components['schemas']['TopologyBase'];
export type TopologyOptions = components['schemas']['TopologyOptions'];
export type TopologyLocalOptions = components['schemas']['TopologyLocalOptions'];
export type TopologyRequestOptions = components['schemas']['TopologyRequestOptions'];
export type TopologyEdge = components['schemas']['Edge'];
export type TopologyNode = components['schemas']['Node'];
export type EdgeHandle = components['schemas']['EdgeHandle'];
export type Binding = components['schemas']['Binding'];
export type Vlan = components['schemas']['Vlan'];

/**
 * Bundle entities carrying the metadata filter values the backend stamps on them
 * (`filter_values`, read through `filterValueOf`).
 */
export type TopologyHost = Host &
	Pick<components['schemas']['TopologyHost'], 'filter_values' | 'device_type'>;
export type TopologyService = components['schemas']['TopologyService'];
export type TopologyInterface = components['schemas']['TopologyInterface'];

/** `entity type → filter type → entities the server-side filter dropped`. */
export type FilteredOutCounts = Record<string, Record<string, number>>;

/**
 * Topology row plus the built graph + entity arrays needed for inspectors,
 * resolvers, and rendering. The slim backend `Topology` only carries
 * `{ id, site_id, options, ... }`. The per-view graph (`nodes`/`edges`) is
 * built on request and the entity arrays (hosts, services, subnets, etc.) are
 * loaded via the `TopologyData` bundle; both are merged here so consumers can
 * read everything off a single object — preserving the field shape used across
 * many components.
 *
 * Snapshot-aware: when a snapshot is selected, the bundle's graph + entities are
 * the snapshot's (built from its closed copies).
 *
 * `name` is a UI-side display string (site name for live, formatted
 * `taken_at` for snapshots, share name for shared topologies).
 *
 * The built graph carries `nodes`/`edges` keyed per view; `toRenderableTopology`
 * selects the active view's slice, so `RenderableTopology` holds those as plain
 * arrays — the shape every pipeline/layout/renderer consumer already expects.
 */
export interface RenderableTopology extends Topology {
	nodes: TopologyNode[];
	edges: TopologyEdge[];
	/** For this view, node id → the entity ids the node also stands for (the search reads it). */
	node_entities: Record<string, string[]>;
	hosts: TopologyHost[];
	services: TopologyService[];
	subnets: Subnet[];
	ip_addresses: IPAddress[];
	ports: Port[];
	bindings: Binding[];
	interfaces: TopologyInterface[];
	/** GH #701: resolved adjacencies for `interfaces` above — see `InterfaceNeighborRow`. */
	neighbours: InterfaceNeighborRow[];
	/**
	 * What the server-side metadata filters removed, by entity type and filter — see
	 * `TopologyData.filtered_out`. These entities are absent from the arrays above, so this is the
	 * only record that they exist at all.
	 */
	filtered_out: FilteredOutCounts;
	/** For this view, how many entities of each type a Visibility tag left out that it would draw. */
	hidden_by_tag: Partial<Record<string, number>>;
	dependencies: Dependency[];
	vlans: Vlan[];
	entity_tags: Tag[];
	/** The organization's Application tag group: its tags place entities in applications. */
	application_group_id: string | null;
	name: string;
}

// Variant types from Node union
export type ElementNode = Extract<TopologyNode, { node_type: 'Element' }>;
export type ContainerNode = Extract<TopologyNode, { node_type: 'Container' }>;

type ElementEntityTypeDiscriminant = components['schemas']['ElementEntityType']['element_type'];

// Frontend-specific render types (not from backend)
export interface ElementRenderData {
	elementType: ElementEntityTypeDiscriminant;
	headerText: string | null;
	/**
	 * An address card names its address: true unless the address is the container subnet's
	 * placeholder, which names nothing.
	 */
	namesAddress?: boolean;
	/** Text naming what failed to resolve, drawn when the element has no icon card. */
	bodyText: string | null;
	showServices: boolean;
	services: Service[];
	/** The card's selection key: the node id, or an IP card's address id (null with no address). */
	ip_address_id: string | null;
	/**
	 * Entities the backend inlined on this card under a manager (`inline_groups`): a runtime with
	 * its containers, a guest's Network Identities with its identity hosts. Their services are
	 * not in `services`.
	 */
	inlineGroups: ElementInlineGroup[];
}

/** One manager's group on a host card, drawn as a dashed box. */
export interface ElementInlineGroup {
	groupId: string;
	/** The viewer collapsed this box: only its header row (with a member count) renders. */
	collapsed: boolean;
	/** The manager service heading the box, when it is on this card. */
	header: Service | null;
	/** Members that are services (bridge containers). */
	services: Service[];
	/** Members that are hosts (macvlan containers, network identities), with their own services. */
	hosts: { host: Host; services: Service[] }[];
}

// ContainerRenderData removed — ContainerNode now reads icon/color directly
// from node data (set by backend graph builder) and metadata.
