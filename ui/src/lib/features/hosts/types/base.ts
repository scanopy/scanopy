// Re-export generated types from OpenAPI schema
import type { components } from '$lib/api/schema';

// Entity primitive types
/**
 * A host row plus the computed title every surface has to agree on.
 *
 * `display_name` lives on the *response* schemas (`HostResponse`, and the topology bundle's
 * `TopologyHost`) rather than on `Host`, because it is derived, not stored. But every list, picker,
 * table and topology consumer holds a `Host`, and `toHostPrimitive` already carries the field
 * through at runtime — so without this it is present in the payload and invisible to the compiler.
 * The rung it came from and the ladder behind it ride along the same way, for the host editor.
 *
 * Both halves come from the generated schema. Read it with `hostDisplayName()`, never directly, and
 * never read `name` for display.
 */
export type Host = components['schemas']['Host'] &
	Pick<components['schemas']['HostResponse'], 'display_name' | 'display_name_rung' | 'name_ladder'>;
export type HostNameRung = components['schemas']['HostNameRung'];
export type HostNameLadderEntry = components['schemas']['HostNameLadderEntry'];
export type HostVirtualization = components['schemas']['HostVirtualization'];
export type ProxmoxVirtualization = components['schemas']['ProxmoxVirtualization'];
export type IPAddress = components['schemas']['IPAddress'];
export type Interface = components['schemas']['Interface'];
/**
 * GH #701: a port's resolved adjacencies, as a `Vec` rather than the old `Interface.neighbor`
 * scalar. Carried on the `TopologyData` bundle (`neighbours`), never on `Interface` itself.
 */
export type InterfaceNeighborRow = components['schemas']['InterfaceNeighborRow'];
/** The two neighbour-resolution states a row can be in — same tagged shape `Neighbor` always had. */
export type Neighbor = components['schemas']['Neighbor'];
/**
 * Raw LLDP/CDP evidence behind `InterfaceNeighborRow` — a port's unresolved candidates, one per
 * distinct record heard on it. Carried on the `TopologyData` bundle (`candidates`), never on
 * `Interface` itself.
 */
export type InterfaceNeighborCandidate = components['schemas']['InterfaceNeighborCandidate'];
export type Port = components['schemas']['Port'];
export type Service = components['schemas']['Service'];
export type TransportProtocol = components['schemas']['TransportProtocol'];

// API response type (host with hydrated children)
export type HostResponse = components['schemas']['HostResponse'];

// API request types - consolidated input types (used for both create and update)
export type CreateHostRequest = components['schemas']['CreateHostRequest'];
export type UpdateHostRequest = components['schemas']['UpdateHostRequest'];
export type IPAddressInput = components['schemas']['IPAddressInput'];
export type PortInput = components['schemas']['PortInput'];
export type ServiceInput = components['schemas']['ServiceInput'];
export type BindingInput = components['schemas']['BindingInput'];

// SNMP types
export type IfAdminStatus = components['schemas']['IfAdminStatus'];
export type IfOperStatus = components['schemas']['IfOperStatus'];

// Credential assignment for a host, optionally limited to specific IP addresses
export interface CredentialAssignment {
	credential_id: string;
	/** IP address IDs to limit this credential to. null = all host IP addresses. */
	ip_address_ids: string[] | null;
}

/** Every `*_source` key the host response carries, derived rather than listed. */
type HostSourceKeys = Extract<keyof HostResponse, `${string}_source`>;

// Form state type for creating/editing hosts
// Includes children arrays for form editing - distinct from HostResponse (API response type)
//
// The read-only naming and provenance fields are partial: a host being created has none of them
// yet, and an edited one carries them from `hydrateHostToFormData`'s spread of the host.
export interface HostFormData
	extends Partial<
		Pick<HostResponse, 'display_name' | 'display_name_rung' | 'name_ladder' | HostSourceKeys>
	> {
	// Host primitive fields
	id: string;
	created_at: string;
	updated_at: string;
	name: string;
	network_id: string;
	// Optional rather than nullable, like the discovered attributes below: it travels with the
	// source that produced it, and absence is the pair missing, not a `null` value.
	hostname?: string;
	description: string | null;
	source: components['schemas']['EntitySource'];
	virtualization_metadata: HostVirtualization | null;
	virtualization_service_id: string | null;
	hidden: boolean;
	tags: string[];

	// SNMP fields (populated by discovery, read-only in UI).
	//
	// Optional rather than nullable: each of these now travels with the source that produced it,
	// and an attribute with no value has no source either — the pair is present or it is not,
	// with nothing in between for a `null` to mean.
	sys_descr?: string;
	sys_object_id?: string;
	sys_location?: string;
	sys_contact?: string;
	management_url?: string;
	chassis_id?: string;
	sys_name?: string;

	// Hardware identity (ENTITY-MIB or a controller integration, read-only in UI)
	manufacturer?: string;
	model?: string;
	serial_number?: string;
	firmware_revision?: string;
	software_revision?: string;

	// Credential assignments (user-editable, from junction table)
	credential_assignments: CredentialAssignment[];

	// Children for form editing (managed separately from host in stores)
	ip_addresses: IPAddress[];
	ports: Port[];
	services: Service[];

	// Interface list (populated by discovery, read-only)
	interfaces: Interface[];
}

// Request type for creating a host (needs form data with children)
export interface CreateHostWithServicesRequest {
	host: HostFormData;
	services: Service[] | null;
}

// Request type for updating a host with children
export interface UpdateHostWithServicesRequest {
	host: Host;
	/** IP addresses to sync - if provided, will create/update/delete to match */
	ip_addresses: IPAddress[] | null;
	/** Ports to sync - if provided, will create/update/delete to match */
	ports: Port[] | null;
	/** Services to sync - if provided, will create/update/delete to match */
	services: Service[] | null;
}

// Frontend-specific types
export interface AllIPAddresses {
	id: null;
	name: string;
}

export const ALL_IP_ADDRESSES: AllIPAddresses = {
	id: null,
	name: 'All IP Addresses'
};
