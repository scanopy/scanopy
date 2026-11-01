import { afterEach, describe, it, expect } from 'vitest';
import { queryClient, queryKeys } from '$lib/api/query-client';
import type { Host, IPAddress } from '$lib/features/hosts/types/base';
import type { Service } from '$lib/features/services/types/base';
import type { RenderableTopology } from '$lib/features/topology/types/base';
import { topologyLookup } from '$lib/features/topology/resolvers';
import type { TooltipChip, TooltipContent, TooltipRow } from '$lib/shared/actions/tooltip';
import { entityRef } from '$lib/shared/components/data/types';
import { collectionLookup, entityFieldLabel, entityFields } from '$lib/shared/entity-fields';
import {
	entityCacheLookup,
	entityRefTooltip,
	entityTooltip,
	hostTooltip,
	interfaceTooltip,
	serviceTooltip
} from '$lib/shared/entity-tooltip';
import {
	credentialTypes,
	entities,
	guestStates,
	ifOperStatuses,
	subnetTypes
} from '$lib/shared/stores/metadata';
import { formatTimestamp } from '$lib/shared/utils/formatting';
import {
	common_lastRun,
	common_manager,
	common_moreItems,
	common_operatingSystem,
	dependencies_memberCount,
	hosts_deviceType
} from '$lib/paraglide/messages';

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
	mac_address: 'AA:BB:CC:00:11:22',
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

	it('shows an address as an address: its MAC, host, subnet and services, no host facts', () => {
		const content = entityTooltip('IPAddress', address, { lookup });
		expect(content.title).toBe('10.0.0.5');
		expect(rows(content).map((r) => r.value)).toEqual(
			expect.arrayContaining(['mgmt', 'AA:BB:CC:00:11:22', 'guest', 'LAN'])
		);
		const labels = rows(content).map((r) => r.label);
		expect(labels).not.toContain(hosts_deviceType());
		expect(labels).not.toContain(common_operatingSystem());
		expect(labels).not.toContain(common_manager());
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

	it('titles a chip as it reads, for a type whose display field is an id', () => {
		const binding = { id: '00000000-uuid', service_id: web.id };
		queryClient.setQueryData(queryKeys.services.all, collections.Service);
		const content = entityRefTooltip(entityRef('Binding', binding.id, binding), '80/tcp on eth0');
		expect(content?.title).toBe('80/tcp on eth0');
		expect(rows(content!).map((r) => r.value)).toContain(web.name);
	});

	it('resolves relations through every collection a chip can name', () => {
		queryClient.setQueryData(queryKeys.hosts.lists(), { items: [guest], pagination: null });
		const daemon = { id: 'd', name: 'dmn', host_id: guest.id, interfaced_subnet_ids: [] };
		queryClient.setQueryData(queryKeys.daemons.all, [daemon]);
		const discovery = {
			id: 'run',
			name: 'nightly',
			daemon_id: daemon.id,
			created_at: '2026-01-02T03:04:05Z',
			run_type: { type: 'Scheduled' }
		};

		const content = entityRefTooltip(entityRef('Discovery', discovery.id, discovery));
		expect(rows(content!).flatMap((r) => r.chips ?? [])).toContainEqual(
			expect.objectContaining({ type: 'Entity', label: daemon.name })
		);
		expect(
			rows(entityRefTooltip(entityRef('Daemon', daemon.id, daemon))!).map((r) => r.value)
		).toContain('guest');
	});
});

describe('entities topology never draws', () => {
	const fromCache = (type: Parameters<typeof entityRef>[0], entity: object) =>
		entityTooltip(type, entity, { lookup });

	afterEach(() => queryClient.clear());

	it("shows a credential's own description, and no row without one", () => {
		const credential = {
			id: 'c',
			name: 'snmp-ro',
			credential_type: { type: credentialTypes.getItems()[0].id }
		};
		const label = entityFieldLabel(
			'Credential',
			entityFields('Credential').find((f) => f.id === 'description')!
		);
		const described = fromCache('Credential', { ...credential, description: 'Core switches' });
		expect(rows(described).find((r) => r.label === label)?.value).toBe('Core switches');
		for (const description of [null, '', '   ']) {
			const blank = fromCache('Credential', { ...credential, description } as never);
			expect(rows(blank).map((r) => r.label)).not.toContain(label);
		}
	});

	it('shows fields flagged for the tooltip, not the rest of what the registry holds', () => {
		const key = {
			id: 'k',
			name: 'ci',
			is_enabled: true,
			last_used: '2026-01-02T03:04:05Z',
			expires_at: '2027-01-02T03:04:05Z'
		};
		const labels = rows(fromCache('DaemonApiKey', key)).map((r) => r.label);
		const label = (id: string) =>
			entityFieldLabel(
				'DaemonApiKey',
				entityFields('DaemonApiKey').find((f) => f.id === id)!
			);
		expect(labels).toEqual(expect.arrayContaining([label('is_enabled'), label('last_used')]));
		expect(labels).not.toContain(label('expires_at'));
	});

	it('lists the services on a port with their addresses', () => {
		const content = fromCache('Port', { ...port, host_id: guest.id } as never);
		expect(content.sections.flatMap((s) => s.items ?? [])).toEqual([
			expect.objectContaining({ text: 'nginx', detail: '10.0.0.5' })
		]);
	});

	it("shows a daemon's subnets as chips", () => {
		const daemon = { id: 'd', name: 'dmn', interfaced_subnet_ids: [subnet.id] };
		const chips = rows(fromCache('Daemon', daemon)).flatMap((r) => r.chips ?? []);
		expect(chips).toContainEqual(expect.objectContaining({ type: 'Entity', label: 'LAN' }));
	});

	it("shows a discovery's date, and its last run as a tag when it has one", () => {
		const discovery = {
			id: 'run',
			name: 'nightly',
			created_at: '2026-01-02T03:04:05Z',
			run_type: { type: 'Historical', results: { phase: 'Complete' } }
		};
		const content = fromCache('Discovery', discovery);
		expect(rows(content).map((r) => r.value)).toContain(formatTimestamp(discovery.created_at));
		expect(chipsOf(content, common_lastRun())).toHaveLength(1);

		const scheduled = fromCache('Discovery', { ...discovery, run_type: { type: 'Scheduled' } });
		expect(chipsOf(scheduled, common_lastRun())).toEqual([]);
	});

	it("counts a dependency's members, and names a topology's site", () => {
		const dependency = {
			id: 'dep',
			name: 'checkout',
			dependency_type: 'RequestPath',
			members: { type: 'Services', service_ids: ['a', 'b'] }
		};
		const sections = fromCache('Dependency', dependency).sections;
		expect(sections.at(-1)?.text).toBe(dependencies_memberCount({ count: 2 }));

		queryClient.setQueryData(queryKeys.sites.all, [{ id: 'site', name: 'HQ' }]);
		const topology = entityTooltip(
			'Topology',
			{ id: 't', site_id: 'site' },
			{ lookup, name: 'Main' }
		);
		expect(rows(topology).map((r) => r.value)).toContain('HQ');
	});
});

describe('a read-only tooltip', () => {
	it('lists the first services and counts the rest, since it cannot scroll', () => {
		const many = Array.from({ length: 15 }, (_, i) => ({
			id: `s${i}`,
			name: `service-${i}`
		})) as unknown as Service[];
		const items = hostTooltip(guest, { services: many }).sections.flatMap((s) => s.items ?? []);
		expect(items).toHaveLength(13);
		expect(items.at(-1)).toBe(common_moreItems({ count: 3 }));

		const exact = hostTooltip(guest, { services: many.slice(0, 12) });
		expect(exact.sections.flatMap((s) => s.items ?? [])).toHaveLength(12);
	});
});
