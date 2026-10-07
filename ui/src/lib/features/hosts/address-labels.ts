import {
	common_port,
	common_unknownEntity,
	hosts_bindings_unknownIPAddress
} from '$lib/paraglide/messages';
import { formatPort } from '$lib/shared/utils/formatting';
import type { AllIPAddresses, IPAddress, Port } from './types/base';

const SEPARATOR = ' · ';

/**
 * What identifies an IP address on its own: the address. A container-subnet address is known by
 * its name, and the "all IP addresses" entry by its own.
 */
export function ipAddressKey(
	i: IPAddress | AllIPAddresses,
	isContainerSubnetFn: (subnetId: string) => boolean
): string {
	if (i.id == null) return i.name;
	if (isContainerSubnetFn(i.subnet_id)) return i.name ?? i.ip_address;
	return i.ip_address;
}

/**
 * How an IP address reads in a tag or a one-line label: the address first, then its name, so
 * that cutting the end costs the name and never the address (`192.168.4.20 · mv-snmp9`). A row
 * with a description line shows `ipAddressKey` and puts the name underneath instead.
 */
export function formatIPAddress(
	i: IPAddress | AllIPAddresses,
	isContainerSubnetFn: (subnetId: string) => boolean
): string {
	const key = ipAddressKey(i, isContainerSubnetFn);
	return i.id != null && i.name && key !== i.name ? key + SEPARATOR + i.name : key;
}

/**
 * How a port binding reads: the port first, since it's what tells one binding of a service from
 * another, then the address it listens on (`443/tcp · 192.168.4.20 · eth0`).
 */
export function formatPortBinding(
	port: Port | undefined,
	ipAddress: IPAddress | AllIPAddresses | undefined,
	isContainerSubnetFn: (subnetId: string) => boolean
): string {
	const portLabel = port ? formatPort(port) : common_unknownEntity({ entity: common_port() });
	const addressLabel = ipAddress
		? formatIPAddress(ipAddress, isContainerSubnetFn)
		: hosts_bindings_unknownIPAddress();
	return portLabel + SEPARATOR + addressLabel;
}
