import { describe, it, expect } from 'vitest';
import { formatIPAddress, formatPortBinding } from '$lib/features/hosts/address-labels';
import { ALL_IP_ADDRESSES, type IPAddress, type Port } from '$lib/features/hosts/types/base';

const notContainer = () => false;
const container = () => true;

const address = (name: string | null, ip_address = '192.168.4.20') =>
	({ id: 'a1', subnet_id: 's1', name, ip_address }) as IPAddress;

/**
 * A narrow row cuts its label from the end, so the value that tells rows apart has to lead.
 */
describe('formatIPAddress', () => {
	it('leads with the address, so truncation costs the name', () => {
		expect(formatIPAddress(address('mv-snmp9'), notContainer).startsWith('192.168.4.20')).toBe(
			true
		);
		const ipv6 = 'fe80::1c2b:3aff:fe4d:5e6f';
		expect(formatIPAddress(address('mv-snmp9', ipv6), notContainer).startsWith(ipv6)).toBe(true);
	});

	it('is the bare address when the address has no name', () => {
		expect(formatIPAddress(address(null), notContainer)).toBe('192.168.4.20');
	});

	it('names a container-subnet address by its name', () => {
		expect(formatIPAddress(address('web-1'), container)).toBe('web-1');
		expect(formatIPAddress(address(null), container)).toBe('192.168.4.20');
	});

	it('names the all-addresses entry by its own name', () => {
		expect(formatIPAddress(ALL_IP_ADDRESSES, notContainer)).toBe(ALL_IP_ADDRESSES.name);
	});
});

describe('formatPortBinding', () => {
	const port = { id: 'p1', number: 443, protocol: 'Tcp' } as Port;

	it('leads with the port, then the address it listens on', () => {
		const label = formatPortBinding(port, address('eth0'), notContainer);
		expect(label.startsWith('443/tcp')).toBe(true);
		expect(label.indexOf('443/tcp')).toBeLessThan(label.indexOf('192.168.4.20'));
	});
});
