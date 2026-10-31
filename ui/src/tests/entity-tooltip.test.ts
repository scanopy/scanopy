import { afterEach, describe, it, expect } from 'vitest';
import { queryClient, queryKeys } from '$lib/api/query-client';
import type { Host, IPAddress } from '$lib/features/hosts/types/base';
import type { Service } from '$lib/features/services/types/base';
import type { RenderableTopology } from '$lib/features/topology/types/base';
import { topologyLookup } from '$lib/features/topology/resolvers';
import type { TooltipChip, TooltipContent, TooltipRow } from '$lib/shared/actions/tooltip';
import { entityRef } from '$lib/shared/components/data/types';
import { collectionLookup } from '$lib/shared/entity-fields';
import {
	entityCacheLookup,
	entityRefTooltip,
	entityTooltip,
	hasEntityTooltip,
	hostTooltip,
	interfaceTooltip,
	serviceTooltip
} from '$lib/shared/entity-tooltip';
import { entities, guestStates, ifOperStatuses, subnetTypes } from '$lib/shared/stores/metadata';

const rows = (content: TooltipContent): TooltipRow[] =>
	content.sections.flatMap((s) => s.rows ?? []);
const chipsOf = (content: TooltipContent, label: string): TooltipChip[] =>
	rows(content).find((r) => r.label === label)?.chips ?? [];

const subnetType = subnetTypes.getItems()[0].id;
const vlan = { id: 'vlan-10', vlan_number: 10, name: 'users' };
const subnet = {
	id: 'lan',
	name: 'LAN',
	cidr: '10.0.0.0/24',
	subnet_type: subnetType,
	vlan_ids: [vlan.id]
};
const address = {
	id: 'ip',
	host_id: 'guest',
	subnet_id: subnet.id,
	ip_address: '10.0.0.5',
	name: 'mgmt'
} as unknown as IPAddress;
const manager = { id: 'pve', name: 'Proxmox VE', host_id: 'hypervisor', bindings: [] };
const web = {
	id: 'web',
	name: 'nginx',
	host_id: 'guest',
	tags: ['prod'],
	bindings: [{ type: 'Port', port_id: 'p80', ip_address_id: address.id }]
};
const guest = {
	id: 'guest',
	name: 'guest',
	display_name: 'guest',
	tags: ['prod'],
	device_type: 'Server',
	virtualization_service_id: manager.id,
	virtualization_metadata: {
		type: 'Proxmox',
		details: { guest_type: 'Lxc', state: 'Running' }
	}
} as unknown as Host;
const iface = {
	id: 'if',
	display_name: 'Gi1/0/1',
	oper_status: 'Up',
	ip_address_id: address.id,
	vlan_ids: [vlan.id]
};
const tag = { id: 'prod', name: 'prod', color: 'Red' };
const port = { id: 'p80', number: 80, protocol: 'Tcp' };

const collections = {
	Host: [guest],
	Service: [manager, web],
	Port: [port],
	Interface: [iface],
	IPAddress: [address],
	Subnet: [subnet],
	Vlan: [vlan],
	Tag: [tag]
};
const lookup = collectionLookup(collections);

describe('off the canvas, with an entity and nothing else', () => {
	const none = collectionLookup({});

	it('builds every covered type, leaving out what needs a related entity', () => {
		const host = entityTooltip('Host', guest, { lookup: none });
		expect(host.title).toBe('guest');
		expect(host.sections.some((s) => s.items)).toBe(false);
		expect(chipsOf(host, 'Tags')).toEqual([]);

		const service = entityTooltip('Service', web, { lookup: none });
		expect(rows(service).some((r) => r.chips?.some((c) => c.type === 'Entity'))).toBe(false);

		expect(entityTooltip('Interface', iface, { lookup: none }).title).toBe('Gi1/0/1');
		expect(entityTooltip('IPAddress', address, { lookup: none }).sections).toBeDefined();
		expect(entityTooltip('Subnet', subnet, { lookup: none }).title).toBeDefined();
	});

	it("names a guest's manager by its type when the service isn't loaded", () => {
		const [chip] = chipsOf(hostTooltip(guest, { services: [] }), 'Manager');
		expect(chip).toMatchObject({ type: 'Tag', label: 'Proxmox' });
	});

	it('shows nothing for a ref with no data, and survives a partial one', () => {
		expect(entityRefTooltip(entityRef('Host', 'missing', undefined as never))).toBeNull();
		const partial = entityRefTooltip(entityRef('Host', 'missing', { id: 'missing', name: 'x' }));
		expect(partial?.sections).toBeDefined();
	});
});

describe('tag-valued rows', () => {
	it("draws a guest's run state in the colour its tag has elsewhere, with its plain text", () => {
		const content = hostTooltip(guest, { services: [], lookup });
		const row = rows(content).find((r) => r.value === guestStates.getName('Running'));
		expect(row?.chips).toEqual([
			{
				type: 'Tag',
				label: guestStates.getName('Running'),
				color: guestStates.getTag('Running').color
			}
		]);
	});

	it("takes a metadata field's tag from the fixture the registry names", () => {
		const content = interfaceTooltip(iface, 'Gi1/0/1', lookup);
		const row = rows(content).find((r) => r.value === ifOperStatuses.getName('Up'));
		expect(row?.chips?.[0]).toMatchObject({
			type: 'Tag',
			color: ifOperStatuses.getTag('Up').color
		});

		const types = rows(entityTooltip('Subnet', subnet, { lookup })).flatMap((r) => r.chips ?? []);
		expect(types).toContainEqual(
			expect.objectContaining({ type: 'Tag', color: subnetTypes.getTag(subnetType).color })
		);
	});

	it('draws tags in their own colours', () => {
		expect(chipsOf(hostTooltip(guest, { services: [], lookup }), 'Tags')[0]).toMatchObject({
			type: 'Tag',
			label: 'prod',
			color: 'Red'
		});
	});
});

describe('entity-valued rows', () => {
	it("names a service's host and a guest's manager as refs, keeping plain text", () => {
		const service = serviceTooltip(web as unknown as Service, { host: guest, lookup });
		const [host] = rows(service).flatMap((r) => r.chips ?? []);
		expect(host).toMatchObject({
			type: 'Entity',
			label: 'guest',
			color: entities.getColorHelper('Host').color,
			ref: { entityType: 'Host', entityId: 'guest' }
		});

		const managerRow = rows(hostTooltip(guest, { services: [], lookup })).find(
			(r) => r.label === 'Manager'
		);
		expect(managerRow?.value).toBe(manager.name);
		expect(managerRow?.chips?.[0]).toMatchObject({ type: 'Entity', label: manager.name });
	});

	it("names an interface's address, subnet and VLANs", () => {
		const refs = rows(interfaceTooltip(iface, 'Gi1/0/1', lookup))
			.flatMap((r) => r.chips ?? [])
			.flatMap((c) => (c.type === 'Entity' ? [c.ref.entityType] : []));
		expect(refs).toEqual(expect.arrayContaining(['IPAddress', 'Subnet', 'Vlan']));
	});

	it('shows an address as its card does: the host, its subnet, its services with ports', () => {
		const content = entityTooltip('IPAddress', address, { lookup });
		expect(content.title).toBe('guest');
		expect(rows(content).map((r) => r.value)).toEqual(
			expect.arrayContaining(['10.0.0.5', 'mgmt', 'LAN'])
		);
		const items = content.sections.flatMap((s) => s.items ?? []);
		expect(items).toEqual([
			expect.objectContaining({
				text: 'nginx',
				detail: expect.stringContaining('80'),
				chip: expect.objectContaining({ type: 'Entity' })
			})
		]);
	});
});

describe('one definition on both surfaces', () => {
	afterEach(() => queryClient.clear());

	it('gives a chip reading the caches the content a card reading its topology gets', () => {
		// Hosts sit in paginated results, the child collections in plain arrays.
		queryClient.setQueryData(queryKeys.hosts.lists(), { items: [guest], pagination: null });
		queryClient.setQueryData(queryKeys.services.all, collections.Service);
		queryClient.setQueryData(queryKeys.ports.all, collections.Port);
		queryClient.setQueryData(queryKeys.ipAddresses.all, collections.IPAddress);
		queryClient.setQueryData(queryKeys.interfaces.all, collections.Interface);
		queryClient.setQueryData(queryKeys.subnets.all, collections.Subnet);
		queryClient.setQueryData(queryKeys.vlans.all, collections.Vlan);
		queryClient.setQueryData(queryKeys.tags.all, collections.Tag);

		const topology = {
			hosts: collections.Host,
			services: collections.Service,
			ports: collections.Port,
			interfaces: collections.Interface,
			ip_addresses: collections.IPAddress,
			subnets: collections.Subnet,
			vlans: collections.Vlan,
			entity_tags: collections.Tag,
			bindings: [],
			dependencies: []
		} as unknown as RenderableTopology;

		for (const [type, entity] of [
			['Host', guest],
			['Service', web],
			['Interface', iface],
			['IPAddress', address],
			['Subnet', subnet]
		] as const) {
			const onCanvas = entityTooltip(type, entity, { lookup: topologyLookup(topology) });
			expect(entityTooltip(type, entity, { lookup: entityCacheLookup() })).toEqual(onCanvas);
		}
	});

	it('prefers the cached entity over the partial object a ref carries', () => {
		queryClient.setQueryData(queryKeys.hosts.lists(), { items: [guest], pagination: null });
		const content = entityRefTooltip(entityRef('Host', guest.id, { id: guest.id, name: 'stub' }));
		expect(content?.title).toBe('guest');
	});

	it('covers what topology draws, and leaves the rest to their display component', () => {
		for (const type of ['Host', 'Service', 'Interface', 'IPAddress', 'Subnet', 'Vlan', 'Tag']) {
			expect(hasEntityTooltip(type)).toBe(true);
		}
		for (const type of ['Credential', 'Daemon', 'Site', 'User', 'Discovery']) {
			expect(hasEntityTooltip(type)).toBe(false);
		}
	});
});
