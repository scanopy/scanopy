/**
 * Option lists for the IP address and port binding inline editors: each candidate paired with
 * the reason it can't be picked, or null when it can.
 */
import {
	ALL_IP_ADDRESSES,
	type AllIPAddresses,
	type IPAddress,
	type Port
} from '$lib/features/hosts/types/base';
import type { IPAddressBinding, PortBinding, Service } from '$lib/features/services/types/base';
import {
	hosts_bindings_boundBy,
	hosts_bindings_hasIPAddressBindings,
	hosts_bindings_ipAddressBindingHere,
	hosts_bindings_portBindingOnAllIPAddresses,
	hosts_bindings_portBindingsOnIPAddress
} from '$lib/paraglide/messages';

export interface BindingOption<T> {
	item: T;
	disabledReason: string | null;
}

/** IP addresses an IP address binding can move to. The binding's own address stays pickable. */
export function ipAddressBindingOptions(
	ipAddresses: IPAddress[],
	binding: IPAddressBinding,
	service: Service | undefined
): BindingOption<IPAddress>[] {
	const portBindings = service?.bindings.filter((b) => b.type === 'Port') ?? [];
	const hasPortBindingOnAll = portBindings.some((b) => b.ip_address_id === null);

	return ipAddresses.map((ipAddr) => {
		let disabledReason: string | null = null;
		if (ipAddr.id !== binding.ip_address_id) {
			if (hasPortBindingOnAll) {
				disabledReason = hosts_bindings_portBindingOnAllIPAddresses();
			} else if (portBindings.some((b) => b.ip_address_id === ipAddr.id)) {
				disabledReason = hosts_bindings_portBindingsOnIPAddress();
			}
		}
		return { item: ipAddr, disabledReason };
	});
}

/** Interfaces a port binding can sit on: each IP address, then "All IP Addresses". */
export function portBindingInterfaceOptions(
	ipAddresses: IPAddress[],
	binding: PortBinding,
	service: Service | undefined
): BindingOption<IPAddress | AllIPAddresses>[] {
	const ipAddressBindings = service?.bindings.filter((b) => b.type === 'IPAddress') ?? [];

	const options: BindingOption<IPAddress | AllIPAddresses>[] = ipAddresses.map((ipAddr) => ({
		item: ipAddr,
		disabledReason: ipAddressBindings.some(
			(b) => b.ip_address_id === ipAddr.id && b.id !== binding.id
		)
			? hosts_bindings_ipAddressBindingHere()
			: null
	}));

	// "All IP Addresses" would include every address this service already binds by IP.
	options.push({
		item: ALL_IP_ADDRESSES,
		disabledReason: ipAddressBindings.length > 0 ? hosts_bindings_hasIPAddressBindings() : null
	});

	return options;
}

/** Two port bindings on the same port conflict when either covers all IP addresses or both
 *  name the same one. */
function interfacesOverlap(a: string | null, b: string | null): boolean {
	return a === null || b === null || a === b;
}

/**
 * The service already binding `portId` on an interface overlapping `interfaceId`: another
 * service first (only those whose bindings are all Port bindings), then another binding of
 * `service` itself. Null when the port is free there.
 */
export function conflictingPortService(
	portId: string,
	interfaceId: string | null,
	binding: PortBinding,
	service: Service | undefined,
	services: Service[]
): Service | null {
	const other = services.find(
		(s) =>
			s.id !== service?.id &&
			s.bindings.every((b) => b.type === 'Port') &&
			s.bindings.some(
				(b) =>
					b.type === 'Port' &&
					b.port_id === portId &&
					interfacesOverlap(b.ip_address_id, interfaceId)
			)
	);
	if (other) return other;

	const selfConflict = service?.bindings.some(
		(b) =>
			b.type === 'Port' &&
			b.id !== binding.id &&
			b.port_id === portId &&
			interfacesOverlap(b.ip_address_id, interfaceId)
	);
	return selfConflict ? (service ?? null) : null;
}

/** Ports a port binding can use on its current interface. The binding's own port stays pickable. */
export function portBindingPortOptions(
	ports: Port[],
	binding: PortBinding,
	service: Service | undefined,
	services: Service[]
): BindingOption<Port>[] {
	return ports.map((port) => {
		const bound =
			port.id === binding.port_id
				? null
				: conflictingPortService(port.id, binding.ip_address_id, binding, service, services);
		return {
			item: port,
			disabledReason: bound ? hosts_bindings_boundBy({ service: bound.name }) : null
		};
	});
}
