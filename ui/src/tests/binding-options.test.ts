import { describe, it, expect } from 'vitest';
import {
	ipAddressBindingOptions,
	portBindingInterfaceOptions,
	portBindingPortOptions
} from '$lib/shared/components/forms/selection/display/bindingOptions';
import type { IPAddress, Port } from '$lib/features/hosts/types/base';
import type { IPAddressBinding, PortBinding, Service } from '$lib/features/services/types/base';

const ip = (id: string) => ({ id, ip_address: id }) as unknown as IPAddress;
const port = (id: string) => ({ id, number: 80, protocol: 'Tcp' }) as unknown as Port;
const portBinding = (id: string, port_id: string, ip_address_id: string | null) =>
	({ id, type: 'Port', port_id, ip_address_id }) as unknown as PortBinding;
const ipBinding = (id: string, ip_address_id: string) =>
	({ id, type: 'IPAddress', ip_address_id }) as unknown as IPAddressBinding;
const service = (id: string, bindings: (PortBinding | IPAddressBinding)[]) =>
	({ id, name: `svc-${id}`, bindings }) as unknown as Service;

const pickable = (options: { item: { id: string | null }; disabledReason: string | null }[]) =>
	options.filter((o) => o.disabledReason === null).map((o) => o.item.id);

describe('ipAddressBindingOptions', () => {
	const ips = [ip('a'), ip('b'), ip('c')];

	it('blocks every other address when the service binds a port on all addresses', () => {
		const binding = ipBinding('ib', 'a');
		const svc = service('s', [binding, portBinding('pb', 'p1', null)]);
		expect(pickable(ipAddressBindingOptions(ips, binding, svc))).toEqual(['a']);
	});

	it('blocks only the addresses the service binds ports on', () => {
		const binding = ipBinding('ib', 'a');
		const svc = service('s', [binding, portBinding('pb', 'p1', 'b')]);
		expect(pickable(ipAddressBindingOptions(ips, binding, svc))).toEqual(['a', 'c']);
	});
});

describe('portBindingInterfaceOptions', () => {
	it('blocks addresses held by another IP binding of the service, and all-addresses', () => {
		const binding = portBinding('pb', 'p1', 'a');
		const svc = service('s', [binding, ipBinding('ib', 'b')]);
		expect(pickable(portBindingInterfaceOptions([ip('a'), ip('b')], binding, svc))).toEqual(['a']);
	});

	it('offers all-addresses when the service has no IP bindings', () => {
		const binding = portBinding('pb', 'p1', 'a');
		const svc = service('s', [binding]);
		expect(pickable(portBindingInterfaceOptions([ip('a')], binding, svc))).toEqual(['a', null]);
	});
});

describe('portBindingPortOptions', () => {
	const ports = [port('p1'), port('p2'), port('p3')];

	it('blocks ports another service binds on an overlapping interface', () => {
		const binding = portBinding('pb', 'p1', 'a');
		const svc = service('s', [binding]);
		const others = [
			service('o1', [portBinding('x', 'p2', null)]), // all addresses overlaps 'a'
			service('o2', [portBinding('y', 'p3', 'b')]) // a different address
		];
		const options = portBindingPortOptions(ports, binding, svc, [svc, ...others]);
		expect(pickable(options)).toEqual(['p1', 'p3']);
		expect(options[1].disabledReason).toContain('svc-o1');
	});

	it("keeps the binding's own port pickable even when it conflicts", () => {
		const binding = portBinding('pb', 'p1', 'a');
		const svc = service('s', [binding, portBinding('pb2', 'p1', 'a')]);
		expect(pickable(portBindingPortOptions(ports, binding, svc, [svc]))).toEqual([
			'p1',
			'p2',
			'p3'
		]);
	});

	it('blocks a port the same service binds elsewhere on the same address', () => {
		const binding = portBinding('pb', 'p1', 'a');
		const svc = service('s', [binding, portBinding('pb2', 'p2', 'a')]);
		expect(pickable(portBindingPortOptions(ports, binding, svc, [svc]))).toEqual(['p1', 'p3']);
	});
});
