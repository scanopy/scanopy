import { describe, it, expect } from 'vitest';
import {
	collectionLookup,
	entityFields,
	fieldText,
	resolveField,
	type EntityField,
	type EntityType
} from '$lib/shared/entity-fields';
import { defineRegistryFields } from '$lib/shared/entity-fields/table';
import { getFieldKey } from '$lib/shared/components/data/types';
import { ENTITY_COLLECTIONS, nodeEntity, topologyLookup } from '$lib/features/topology/resolvers';
import { containerTypes } from '$lib/shared/stores/metadata';
import containerTypesJson from '$lib/data/container-types.json';
import type { RenderableTopology, TopologyNode } from '$lib/features/topology/types/base';

/** A field built inline, so the walker is tested on paths rather than on the registry's contents. */
function field(path: EntityField['path'], value: string | null, many = false): EntityField {
	return {
		id: 'f',
		label: 'F',
		group: 'identity',
		path,
		field: value,
		format: { type: 'Text' },
		summary: false,
		tooltip: false,
		many,
		searchable: false
	};
}

const host = { id: 'h1', name: 'web-01', virtualization_service_id: 's9' };
const ips = [
	{ id: 'ip1', host_id: 'h1', ip_address: '10.0.0.5' },
	{ id: 'ip2', host_id: 'h1', ip_address: '10.0.0.6' },
	{ id: 'ip3', host_id: 'h2', ip_address: '10.0.0.7' }
];
const vlans = [
	{ id: 'v10', vlan_number: 10 },
	{ id: 'v20', vlan_number: 20 }
];
const ports = [{ id: 'p1', number: 443, protocol: 'Tcp' }];
const services = [{ id: 's9', name: 'Proxmox', bindings: [{ type: 'Port', port_id: 'p1' }] }];
const lookup = collectionLookup({
	IPAddress: ips,
	Vlan: vlans,
	Port: ports,
	Service: services,
	Host: [host]
});

describe('entity field walker', () => {
	it('reads a field on the entity itself, following a dotted path', () => {
		expect(
			resolveField(field([], 'source.type'), { source: { type: 'Discovery' } }, lookup)
		).toEqual(['Discovery']);
	});

	it('follows a parent key holding one id or an array of ids', () => {
		const single = field(
			[{ type: 'Parent', entity: 'Service', key: 'virtualization_service_id' }],
			'name'
		);
		expect(resolveField(single, host, lookup)).toEqual(['Proxmox']);
		const many = field([{ type: 'Parent', entity: 'Vlan', key: 'vlan_ids' }], 'vlan_number', true);
		expect(resolveField(many, { vlan_ids: ['v10', 'v20'] }, lookup)).toEqual([10, 20]);
	});

	it('collects children by their foreign key, and only the entity’s own', () => {
		const children = field(
			[{ type: 'Children', entity: 'IPAddress', key: 'host_id' }],
			'ip_address',
			true
		);
		expect(resolveField(children, host, lookup)).toEqual(['10.0.0.5', '10.0.0.6']);
	});

	it('walks an embedded array, then a parent key on each element', () => {
		const ports = field(
			[
				{ type: 'Embedded', key: 'bindings' },
				{ type: 'Parent', entity: 'Port', key: 'port_id' }
			],
			null,
			true
		);
		ports.format = { type: 'Port' };
		expect(fieldText(ports, services[0], lookup)).toBe('443/tcp');
	});

	it('reads nothing for a relation whose collection the page did not load', () => {
		const children = field(
			[{ type: 'Children', entity: 'Interface', key: 'host_id' }],
			'if_name',
			true
		);
		expect(resolveField(children, host, lookup)).toEqual([]);
	});
});

describe('registry-backed table columns', () => {
	type Row = { id: string; name: string; subnet_ids: string[]; interface_ids?: string[] };
	const subnets = [{ id: 'sn1', name: 'LAN' }];
	const interfaces = [{ id: 'if1', host_id: 'h1', if_name: 'Gi1/0/1' }];
	const columns = () =>
		defineRegistryFields<Row, 'name'>(
			'Vlan',
			() => collectionLookup({ Subnet: subnets, Interface: interfaces }),
			{ name: { searchable: true } },
			[{ key: 'description', display: { hiddenByDefault: false } }],
			{ omit: ['source'] }
		);

	it('keeps the tab’s settings on listed columns and takes the rest from the registry', () => {
		const name = columns().find((c) => getFieldKey(c) === 'name')!;
		expect(name.searchable).toBe(true);
		expect(name.label).toBeTruthy();
		expect(name.display?.hiddenByDefault).toBeUndefined();
	});

	it('appends registry fields the tab neither lists nor omits, hidden unless they are summary', () => {
		const keys = columns().map(getFieldKey);
		const registry = entityFields('Vlan');
		for (const f of registry) {
			if (f.id === 'source') expect(keys).not.toContain('source');
			else expect(keys).toContain(f.id);
		}
		const appended = columns().filter((c) => !['name', 'description'].includes(getFieldKey(c)));
		for (const column of appended) {
			const f = registry.find((r) => r.id === getFieldKey(column))!;
			expect(column.display?.hiddenByDefault).toBe(!f.summary);
		}
	});

	it('draws an entity field as linked chips of the related entities', () => {
		const subnetsColumn = columns().find((c) => getFieldKey(c) === 'subnet_ids')!;
		const items = subnetsColumn.display?.getItems?.({ id: 'v1', name: 'v', subnet_ids: ['sn1'] });
		expect(items?.map((i) => [i.label, i.entityRef?.entityType, i.entityRef?.entityId])).toEqual([
			['LAN', 'Subnet', 'sn1']
		]);
	});

	it('draws the interfaces serving a VLAN as linked interface chips', () => {
		const interfacesColumn = columns().find((c) => getFieldKey(c) === 'interface_ids')!;
		const items = interfacesColumn.display?.getItems?.({
			id: 'v1',
			name: 'v',
			subnet_ids: [],
			interface_ids: ['if1', 'missing']
		});
		expect(items?.map((i) => [i.entityRef?.entityType, i.entityRef?.entityId])).toEqual([
			['Interface', 'if1']
		]);
		expect(items?.[0].label).toContain('Gi1/0/1');
	});

	it('reads values from the entity a row wraps when the tab names its root', () => {
		type Wrapped = { data: { id: string; subnet_ids: string[] } };
		const wrapped = defineRegistryFields<Wrapped, never>(
			'Vlan',
			() => collectionLookup({ Subnet: subnets }),
			{} as Record<never, never>,
			[],
			{ root: (item) => item.data }
		);
		const subnetsColumn = wrapped.find((c) => getFieldKey(c) === 'subnet_ids')!;
		const row = { data: { id: 'v1', subnet_ids: ['sn1'] } };
		expect(subnetsColumn.getValue?.(row)).toEqual(['LAN']);
		expect(subnetsColumn.display?.getItems?.(row)?.map((i) => i.entityRef?.entityId)).toEqual([
			'sn1'
		]);
	});
});

describe('node entity lookup', () => {
	const topology = {
		hosts: [{ id: 'h1', name: 'web-01' }],
		ip_addresses: [{ id: 'ip1', host_id: 'h1', ip_address: '10.0.0.5' }],
		vlans: [{ id: 'v10', vlan_number: 10, name: 'users' }],
		services: [],
		subnets: [],
		ports: [],
		bindings: [],
		interfaces: [],
		dependencies: [],
		entity_tags: []
	} as unknown as RenderableTopology;

	it('resolves a card through its element type and entity id', () => {
		const card = {
			node_type: 'Element',
			element_type: 'IPAddress',
			entity_id: 'ip1',
			host_id: 'h1'
		} as unknown as TopologyNode;
		expect(nodeEntity(card, topology)?.entity).toMatchObject({ ip_address: '10.0.0.5' });
	});

	it('resolves a container through its type’s entity metadata, and a grouping to nothing', () => {
		const vlanBox = {
			node_type: 'Container',
			container_type: 'VLAN',
			entity_id: 'v10'
		} as unknown as TopologyNode;
		expect(nodeEntity(vlanBox, topology)).toMatchObject({
			entityType: 'Vlan',
			entity: { vlan_number: 10 }
		});
		const trunks = {
			node_type: 'Container',
			container_type: 'TrunkPort'
		} as unknown as TopologyNode;
		expect(nodeEntity(trunks, topology)).toBeNull();
		expect(
			fieldText(
				entityFields('Vlan')[0],
				nodeEntity(vlanBox, topology)!.entity,
				topologyLookup(topology)
			)
		).toBe('10');
	});

	it('can find the entity of every shape type the backend says stands for one', () => {
		const elementTypes: EntityType[] = ['IPAddress', 'Service', 'Host', 'Interface'];
		const containerEntityTypes = containerTypesJson
			.map((item) => containerTypes.getMetadata(item.id).entity_type)
			.filter((type): type is EntityType => !!type);
		for (const type of [...elementTypes, ...containerEntityTypes]) {
			expect(ENTITY_COLLECTIONS, `no topology collection for ${type}`).toHaveProperty(type);
		}
	});
});
