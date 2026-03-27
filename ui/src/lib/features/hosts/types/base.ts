// Re-export generated types from OpenAPI schema
import type { components } from '$lib/api/schema';

// Entity primitive types
export type Host = components['schemas']['Host'];
export type HostVirtualization = components['schemas']['HostVirtualization'];
export type ProxmoxVirtualization = components['schemas']['ProxmoxVirtualization'];
export type Interface = components['schemas']['Interface'];
export type Port = components['schemas']['Port'];
export type Service = components['schemas']['Service'];
export type TransportProtocol = components['schemas']['TransportProtocol'];

// API response type (host with hydrated children)
export type HostResponse = components['schemas']['HostResponse'];

// API request types - consolidated input types (used for both create and update)
export type CreateHostRequest = components['schemas']['CreateHostRequest'];
export type UpdateHostRequest = components['schemas']['UpdateHostRequest'];
export type InterfaceInput = components['schemas']['InterfaceInput'];
export type PortInput = components['schemas']['PortInput'];
export type ServiceInput = components['schemas']['ServiceInput'];
export type BindingInput = components['schemas']['BindingInput'];

// SNMP types
export type IfEntry = components['schemas']['IfEntry'];
export type IfAdminStatus = components['schemas']['IfAdminStatus'];
export type IfOperStatus = components['schemas']['IfOperStatus'];

// Credential assignment for a host, optionally limited to specific interfaces
export interface CredentialAssignment {
	credential_id: string;
	/** Interface IDs to limit this credential to. null = all host interfaces. */
	interface_ids: string[] | null;
}

// Form state type for creating/editing hosts
// Includes children arrays for form editing - distinct from HostResponse (API response type)
export interface HostFormData {
	// Host primitive fields
	id: string;
	created_at: string;
	updated_at: string;
	name: string;
	network_id: string;
	hostname: string | null;
	description: string | null;
	source: components['schemas']['EntitySource'];
	virtualization: HostVirtualization | null;
	hidden: boolean;
	tags: string[];

	// SNMP fields (populated by discovery, read-only in UI)
	sys_descr: string | null;
	sys_object_id: string | null;
	sys_location: string | null;
	sys_contact: string | null;
	management_url: string | null;
	chassis_id: string | null;

	// Credential assignments (user-editable, from junction table)
	credential_assignments: CredentialAssignment[];

	// Children for form editing (managed separately from host in stores)
	interfaces: Interface[];
	ports: Port[];
	services: Service[];

	// IfEntry list (populated by discovery, read-only)
	if_entries: IfEntry[];
}

// Request type for creating a host (needs form data with children)
export interface CreateHostWithServicesRequest {
	host: HostFormData;
	services: Service[] | null;
}

// Request type for updating a host with children
export interface UpdateHostWithServicesRequest {
	host: Host;
	/** Interfaces to sync - if provided, will create/update/delete to match */
	interfaces: Interface[] | null;
	/** Ports to sync - if provided, will create/update/delete to match */
	ports: Port[] | null;
	/** Services to sync - if provided, will create/update/delete to match */
	services: Service[] | null;
}

// Frontend-specific types
export interface AllInterfaces {
	id: null;
	name: string;
}

export const ALL_INTERFACES: AllInterfaces = {
	id: null,
	name: 'All Interfaces'
};
