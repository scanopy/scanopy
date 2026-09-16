import type { components } from '$lib/api/schema';
import type { Service } from '$lib/features/services/types/base';
import type {
	Host,
	IPAddress,
	Interface,
	Port,
	InterfaceNeighborRow,
	InterfaceNeighborCandidate
} from '$lib/features/hosts/types/base';
import type { Subnet } from '$lib/features/subnets/types/base';
import type { Dependency } from '$lib/features/dependencies/types/base';
import type { Tag } from '$lib/features/tags/types/base';

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

/** `entity type → filter type → entities the server-side filter dropped`. */
export type FilteredOutCounts = Record<string, Record<string, number>>;

/**
 * Topology row plus the built graph + entity arrays needed for inspectors,
 * resolvers, and rendering. The slim backend `Topology` only carries
 * `{ id, network_id, options, ... }`. The per-view graph (`nodes`/`edges`) is
 * built on request and the entity arrays (hosts, services, subnets, etc.) are
 * loaded via the `TopologyData` bundle; both are merged here so consumers can
 * read everything off a single object — preserving the field shape used across
 * many components.
 *
 * Snapshot-aware: when a snapshot is selected, the bundle's graph + entities are
 * the snapshot's (built from its closed copies).
 *
 * `name` is a UI-side display string (network name for live, formatted
 * `taken_at` for snapshots, share name for shared topologies).
 *
 * The built graph carries `nodes`/`edges` keyed per view; `toRenderableTopology`
 * selects the active view's slice, so `RenderableTopology` holds those as plain
 * arrays — the shape every pipeline/layout/renderer consumer already expects.
 */
export interface RenderableTopology extends Topology {
	nodes: TopologyNode[];
	edges: TopologyEdge[];
	hosts: Host[];
	services: Service[];
	subnets: Subnet[];
	ip_addresses: IPAddress[];
	ports: Port[];
	bindings: Binding[];
	interfaces: Interface[];
	/** GH #701: resolved adjacencies for `interfaces` above — see `InterfaceNeighborRow`. */
	neighbours: InterfaceNeighborRow[];
	/** Raw LLDP/CDP evidence behind `neighbours` — see `InterfaceNeighborCandidate`. */
	candidates: InterfaceNeighborCandidate[];
	/**
	 * What the server-side metadata filters removed, by entity type and filter — see
	 * `TopologyData.filtered_out`. These entities are absent from the arrays above, so this is the
	 * only record that they exist at all.
	 */
	filtered_out: FilteredOutCounts;
	dependencies: Dependency[];
	vlans: Vlan[];
	entity_tags: Tag[];
	name: string;
}

// Variant types from Node union
export type ElementNode = Extract<TopologyNode, { node_type: 'Element' }>;
export type ContainerNode = Extract<TopologyNode, { node_type: 'Container' }>;

// Frontend-specific render types (not from backend)
export interface PortStatus {
	operStatus: 'Up' | 'Down' | string;
	speed: string | null;
	macAddress: string | null;
}

type ElementEntityTypeDiscriminant = components['schemas']['ElementEntityType']['element_type'];

export interface ElementRenderData {
	elementType: ElementEntityTypeDiscriminant;
	headerText: string | null;
	subtitleText?: string | null;
	footerText: string | null;
	bodyText: string | null;
	showServices: boolean;
	isVirtualized: boolean;
	isContainerized: boolean;
	services: Service[];
	hiddenOpenPorts: Service[];
	ip_address_id: string;
	isCategoryHidden?: boolean;
	portStatus?: PortStatus;
}

// ContainerRenderData removed — ContainerNode now reads icon/color directly
// from node data (set by backend graph builder) and metadata.
