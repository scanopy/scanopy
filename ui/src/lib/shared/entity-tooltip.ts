/**
 * What an entity's tooltip says, the one definition behind a topology card or container and an
 * EntityRef chip. Every builder reads related entities through an `EntityLookup`: topology passes
 * one over its snapshot, and a chip passes one over the query caches. A related entity the lookup
 * lacks leaves its row out, or falls back to the metadata name where one exists.
 */

import type { Host, HostVirtualization, IPAddress } from '$lib/features/hosts/types/base';
import type { Service, ServiceVirtualization } from '$lib/features/services/types/base';
import type { Tag, TagGroup } from '$lib/features/tags/types/base';
import { hostDisplayName } from '$lib/features/hosts/host-display-name';
import { hostOsIcon, hostOsLabel } from '$lib/features/hosts/host-os';
import { applicationGroupId, tagIcon } from '$lib/features/tags/groups';
import { cachedEntities, queryClient, queryKeys } from '$lib/api/query-client';
import type {
	TooltipChip,
	TooltipContent,
	TooltipItem,
	TooltipRow,
	TooltipSection
} from '$lib/shared/actions/tooltip';
import { entityRef, type EntityRef } from '$lib/shared/components/data/types';
import {
	collectionLookup,
	entityDisplayText,
	entityFieldLabel,
	entityFields,
	fieldEntityType,
	fieldText,
	formatFieldValue,
	resolveField,
	type EntityField,
	type EntityLookup,
	type EntityType
} from '$lib/shared/entity-fields';
import {
	containerNetworkTypes,
	containerTypes,
	deviceTypes,
	entities,
	guestStates,
	hostVirtualizations,
	metadataColor,
	proxmoxGuestTypes,
	serviceCategories,
	serviceDefinitions,
	serviceVirtualizations,
	views
} from '$lib/shared/stores/metadata';
import { formatPort } from '$lib/shared/utils/formatting';
import type { Color } from '$lib/shared/utils/styling';
import {
	common_category,
	common_host,
	common_ipAddress,
	common_manager,
	common_name,
	common_operatingSystem,
	common_services,
	common_status,
	common_tags,
	common_virtualization,
	hosts_deviceFacts_composeProject,
	hosts_deviceFacts_guestType,
	hosts_deviceFacts_networkType,
	hosts_deviceType
} from '$lib/paraglide/messages';

/** A tooltip row before empty values are dropped. */
type MaybeRow = { label: string; value: string | null | undefined; chips?: TooltipChip[] };

/** The name and colour of a metadata value, as every metadata helper's `getTag` gives them. */
type TagValue = { label: string; color?: Color | null };

const NO_ENTITIES = collectionLookup({});

function tagChip({ label, color }: TagValue): TooltipChip {
	return { type: 'Tag', label, color: color ?? 'Gray' };
}

/** A value the app shows as a tag elsewhere. */
function tagRow(label: string, tag: TagValue | null): MaybeRow {
	return { label, value: tag?.label, chips: tag ? [tagChip(tag)] : undefined };
}

function entityChip(type: EntityType, entity: object, label?: string): TooltipChip {
	return {
		type: 'Entity',
		label: label ?? entityDisplayText(type, entity),
		color: entities.getColorHelper(type).color,
		ref: entityRef(type, (entity as { id: string }).id, entity)
	};
}

function entityRow(label: string, type: EntityType, related: object | null | undefined): MaybeRow {
	if (!related) return { label, value: null };
	const chip = entityChip(type, related);
	return { label, value: chip.label, chips: [chip] };
}

/** One entity by id, or undefined when the lookup lacks it. */
function byId<T>(lookup: EntityLookup, type: EntityType, id: string | null | undefined) {
	return id ? (lookup.byColumn(type, 'id', id)[0] as T | undefined) : undefined;
}

function statusRow(state: string | null | undefined): MaybeRow {
	return tagRow(common_status(), state ? guestStates.getTag(state) : null);
}

/** The service managing a guest, or the type of manager when that service isn't loaded. */
function managerRow(
	serviceId: string | null | undefined,
	type: TagValue,
	lookup: EntityLookup
): MaybeRow {
	const service = byId<Service>(lookup, 'Service', serviceId);
	return service
		? { ...entityRow(common_manager(), 'Service', service), value: service.name }
		: tagRow(common_manager(), type);
}

/** An entity's tags, each drawn the way its chip is everywhere else. */
function tagsRow(tagIds: string[] | undefined, lookup: EntityLookup): MaybeRow {
	const tags = (tagIds ?? []).flatMap((id) => lookup.byColumn('Tag', 'id', id)) as Tag[];
	if (tags.length === 0) return { label: common_tags(), value: null };
	const groups = queryClient.getQueryData<TagGroup[]>(queryKeys.tagGroups.all) ?? [];
	const application = applicationGroupId(groups);
	return {
		label: common_tags(),
		value: tags.map((tag) => tag.name).join(', '),
		chips: tags.map((tag) => ({
			type: 'Tag',
			label: tag.name,
			color: tag.color,
			icon: tagIcon(tag, application)
		}))
	};
}

/**
 * What a guest host's tooltip adds about how it runs: its manager, the modifiers that set it apart
 * from a plain guest (a Proxmox guest's type, a container's macvlan or ipvlan network, its Compose
 * project) and its run state. The same facts the host modal's virtualization section lists.
 */
function hostGuestRows(
	virtualization: HostVirtualization | null | undefined,
	managerId: string | null | undefined,
	lookup: EntityLookup
): MaybeRow[] {
	if (!virtualization) return [];
	const manager = managerRow(managerId, hostVirtualizations.getTag(virtualization.type), lookup);
	switch (virtualization.type) {
		case 'Proxmox': {
			const { guest_type, state } = virtualization.details;
			return [
				manager,
				tagRow(
					hosts_deviceFacts_guestType(),
					guest_type ? proxmoxGuestTypes.getTag(guest_type) : null
				),
				statusRow(state)
			];
		}
		case 'Libvirt':
			return [manager, statusRow(virtualization.details.state)];
		case 'VCenter':
		case 'ESXi':
			return [manager];
		case 'Docker':
		case 'Podman': {
			const { network_type, compose_project, state } = virtualization.details;
			return [
				manager,
				tagRow(hosts_deviceFacts_networkType(), containerNetworkTypes.getTag(network_type)),
				{ label: hosts_deviceFacts_composeProject(), value: compose_project },
				statusRow(state)
			];
		}
		// An identity on its guest's network, not a guest of its own: no manager and no run state.
		case 'NetworkIdentity':
			return [tagRow(common_virtualization(), hostVirtualizations.getTag(virtualization.type))];
	}
}

/** A containerized service's rows: its runtime, Compose project and run state. */
function serviceGuestRows(
	virtualization: ServiceVirtualization | null | undefined,
	managerId: string | null | undefined,
	lookup: EntityLookup
): MaybeRow[] {
	if (!virtualization) return [];
	const { compose_project, state } = virtualization.details;
	return [
		managerRow(managerId, serviceVirtualizations.getTag(virtualization.type), lookup),
		{ label: hosts_deviceFacts_composeProject(), value: compose_project },
		statusRow(state)
	];
}

/** A titled tooltip: one row per value present, then the services under their own heading. */
function titledTooltip(
	title: string,
	rows: MaybeRow[],
	services: (string | TooltipItem)[] = []
): TooltipContent {
	const present: TooltipRow[] = rows.flatMap(({ label, value, chips }) =>
		value ? [{ label, value, chips }] : []
	);
	const sections: TooltipSection[] = [];
	if (present.length > 0) sections.push({ rows: present });
	if (services.length > 0) sections.push({ heading: common_services(), items: services });
	return { title, sections };
}

/**
 * A service's hover text: its name as the title, the host it runs on when the card does not
 * already say it, its category and tags, and how it runs when it is containerized.
 */
export function serviceTooltip(
	service: Service,
	{ host, lookup = NO_ENTITIES }: { host?: Host | null; lookup?: EntityLookup } = {}
): TooltipContent {
	const category = serviceDefinitions.getCategory(service.service_definition);
	return titledTooltip(service.name, [
		host
			? {
					label: common_host(),
					value: hostDisplayName(host),
					chips: [entityChip('Host', host, hostDisplayName(host))]
				}
			: { label: common_host(), value: null },
		tagRow(common_category(), category ? serviceCategories.getTag(category) : null),
		...serviceGuestRows(service.virtualization_metadata, service.virtualization_service_id, lookup),
		tagsRow(service.tags, lookup)
	]);
}

/** A service as a list entry, naming it by its chip. */
function serviceListItem(service: Service): TooltipItem {
	return { text: service.name, chip: entityChip('Service', service, service.name) };
}

/** A service with the ports it listens on at one address, set apart as the item's detail. */
function serviceWithPorts(
	service: Service,
	ipAddressId: string,
	lookup: EntityLookup
): TooltipItem {
	const ports = (service.bindings ?? []).flatMap((b) => {
		if (b.type !== 'Port' || !b.port_id) return [];
		if (b.ip_address_id != null && b.ip_address_id !== ipAddressId) return [];
		const port = lookup.byColumn('Port', 'id', b.port_id)[0];
		return port ? [formatPort(port as never)] : [];
	});
	return {
		...serviceListItem(service),
		detail: ports.length > 0 ? ports.join(', ') : undefined
	};
}

/**
 * A host's hover text: its name as the title, then the address the tooltip is for and its subnet
 * when there is one, its device type and how it runs as a guest, its tags, then its services, each
 * with its ports at that address.
 */
export function hostTooltip(
	host: Host,
	{
		name,
		ipAddress,
		deviceType,
		services,
		lookup = NO_ENTITIES
	}: {
		name?: string;
		ipAddress?: IPAddress | null;
		deviceType?: string | null;
		services: Service[];
		lookup?: EntityLookup;
	}
): TooltipContent {
	const os = host.os;
	return titledTooltip(
		name ?? hostDisplayName(host),
		[
			{ label: common_ipAddress(), value: ipAddress?.ip_address },
			{ label: common_name(), value: ipAddress?.name },
			entityRow(
				entities.getName('Subnet'),
				'Subnet',
				byId<object>(lookup, 'Subnet', ipAddress?.subnet_id)
			),
			tagRow(hosts_deviceType(), deviceType ? deviceTypes.getTag(deviceType) : null),
			{
				label: common_operatingSystem(),
				value: os ? hostOsLabel(os) : null,
				chips: os
					? [
							{
								type: 'Tag',
								label: hostOsLabel(os),
								color: 'Gray',
								icon: hostOsIcon(os.family, { framed: false })
							}
						]
					: undefined
			},
			...hostGuestRows(host.virtualization_metadata, host.virtualization_service_id, lookup),
			tagsRow(host.tags, lookup)
		],
		services.map((service) =>
			ipAddress ? serviceWithPorts(service, ipAddress.id, lookup) : serviceListItem(service)
		)
	);
}

/** One registry field as a row: chips for the tags and entities it names, and its plain text. */
function fieldRow(
	type: EntityType,
	field: EntityField,
	entity: object,
	lookup: EntityLookup
): MaybeRow {
	return {
		label: entityFieldLabel(type, field),
		value: fieldText(field, entity, lookup),
		chips: fieldChips(field, entity, lookup)
	};
}

function fieldChips(
	field: EntityField,
	entity: object,
	lookup: EntityLookup
): TooltipChip[] | undefined {
	const format = field.format;
	if (format.type === 'Metadata') {
		return resolveField(field, entity, lookup).map((value) =>
			tagChip({
				label: formatFieldValue(field, value),
				color: metadataColor(format.fixture, String(value))
			})
		);
	}
	const target = format.type === 'Entity' ? fieldEntityType(field) : null;
	if (!target) return undefined;
	return (resolveField(field, entity, lookup) as { id?: string }[]).flatMap((related) =>
		related?.id ? [entityChip(target, related)] : []
	);
}

/** The interface fields a port card's tooltip lists, in order: what the card has no room for. */
const PORT_TOOLTIP_FIELDS = [
	'oper_status',
	'speed_bps',
	'mac_address',
	'ip_address_id',
	'native_vlan_id',
	'vlan_ids'
];

/**
 * A port's hover text: its name as the title, then its status, speed, MAC, address, the address's
 * subnet and its VLANs. Labels and formatting are the backend field registry's, as the inspector
 * shows them.
 */
export function interfaceTooltip(
	iface: object,
	name: string,
	lookup: EntityLookup = NO_ENTITIES
): TooltipContent {
	const fields = entityFields('Interface');
	const address = byId<IPAddress>(
		lookup,
		'IPAddress',
		(iface as { ip_address_id?: string | null }).ip_address_id
	);
	return titledTooltip(
		name,
		PORT_TOOLTIP_FIELDS.flatMap((id) => {
			const field = fields.find((f) => f.id === id);
			if (!field) return [];
			const row = fieldRow('Interface', field, iface, lookup);
			return id === 'ip_address_id'
				? [
						row,
						entityRow(
							entities.getName('Subnet'),
							'Subnet',
							byId<object>(lookup, 'Subnet', address?.subnet_id)
						)
					]
				: [row];
		})
	);
}

/** An entity's summary fields, then the related entities `lookup` carries. */
function registryTooltip(type: EntityType, entity: object, title: string, lookup: EntityLookup) {
	// A relation into a collection the lookup does not load would read as empty, so it is left out.
	// A list of related entities is shown only where the registry marks it a summary (a subnet's
	// VLANs); a long one (a VLAN's interfaces) is the inspector's.
	const fields = entityFields(type);
	const reachable = (f: EntityField) =>
		f.path.every((hop) => hop.type === 'Embedded' || lookup.loads(hop.entity));
	const rows = fields
		.filter(
			(f) =>
				(f.summary && f.path.length === 0) ||
				(f.format.type === 'Entity' && (!f.many || f.summary) && reachable(f))
		)
		.map((f) => fieldRow(type, f, entity, lookup))
		.filter((row) => row.value !== title);
	return titledTooltip(title, rows);
}

/**
 * Any entity's hover text, by its type: a host, service, port or address the way its card shows
 * it, and any other entity (a subnet, a VLAN) as its summary fields, the ones the inspector shows
 * by default. `name` replaces the entity's own display text as the title.
 */
export function entityTooltip(
	entityType: string,
	entity: object,
	{ lookup, name }: { lookup: EntityLookup; name?: string }
): TooltipContent {
	const type = entityType as EntityType;
	switch (type) {
		case 'Host': {
			const host = entity as Host;
			return hostTooltip(host, {
				name,
				deviceType: host.device_type ?? 'Unidentified',
				services: lookup.byColumn('Service', 'host_id', host.id) as Service[],
				lookup
			});
		}
		case 'Service': {
			const service = entity as Service;
			return serviceTooltip(service, {
				host: byId<Host>(lookup, 'Host', service.host_id),
				lookup
			});
		}
		case 'Interface':
			return interfaceTooltip(entity, name ?? entityDisplayText(type, entity), lookup);
		case 'IPAddress': {
			const address = entity as IPAddress;
			const host = byId<Host>(lookup, 'Host', address.host_id);
			if (!host) {
				return registryTooltip(type, entity, name ?? entityDisplayText(type, entity), lookup);
			}
			const services = lookup.byColumn('Service', 'host_id', host.id) as Service[];
			return hostTooltip(host, {
				name,
				ipAddress: address,
				deviceType: host.device_type ?? 'Unidentified',
				services: services.filter((s) =>
					(s.bindings ?? []).some((b) => b.ip_address_id == null || b.ip_address_id === address.id)
				),
				lookup
			});
		}
		default:
			return registryTooltip(type, entity, name ?? entityDisplayText(type, entity), lookup);
	}
}

/** The collections a chip's tooltip reads, as the app has them cached. */
export function entityCacheLookup(): EntityLookup {
	return collectionLookup({
		Host: cachedEntities(queryKeys.hosts.all),
		Service: cachedEntities(queryKeys.services.all),
		Port: cachedEntities(queryKeys.ports.all),
		Interface: cachedEntities(queryKeys.interfaces.all),
		IPAddress: cachedEntities(queryKeys.ipAddresses.all),
		Subnet: cachedEntities(queryKeys.subnets.all),
		Vlan: cachedEntities(queryKeys.vlans.all),
		Tag: cachedEntities(queryKeys.tags.all)
	});
}

let canvasTypes: Set<string> | null = null;

/**
 * Whether topology draws this entity type, as a card or as a container, and so owns its tooltip.
 * Read from the views and container types the backend declares.
 */
export function hasEntityTooltip(entityType: string): boolean {
	canvasTypes ??= new Set(
		[
			...views.getItems().flatMap((view) => {
				const config = (
					view.metadata as {
						element_config?: {
							container_entity?: string | null;
							element_entities?: { entity_type: string }[];
						};
					} | null
				)?.element_config;
				return [
					config?.container_entity,
					...(config?.element_entities ?? []).map((e) => e.entity_type)
				];
			}),
			...containerTypes.getItems().map((type) => type.metadata?.entity_type)
		].filter((type): type is string => !!type)
	);
	return canvasTypes.has(entityType);
}

/**
 * The tooltip of a chip: the cached entity when there is one, as a ref can carry a partial object,
 * else the ref's own data. Null when the ref carries nothing to describe.
 */
export function entityRefTooltip(ref: EntityRef): TooltipContent | null {
	const lookup = entityCacheLookup();
	const entity = lookup.byColumn(ref.entityType, 'id', ref.entityId)[0] ?? ref.data;
	return entity ? entityTooltip(ref.entityType, entity, { lookup }) : null;
}
