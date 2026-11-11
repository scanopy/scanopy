/**
 * What an entity's tooltip says, the one definition behind a topology card or container and an
 * EntityRef chip. Every builder reads related entities through an `EntityLookup`: topology passes
 * one over its snapshot, and a chip passes one over the query caches. A related entity the lookup
 * lacks leaves its row out, or falls back to the metadata name where one exists.
 */

import type { components } from '#lib/api/schema';
import type { Daemon } from '#lib/features/daemons/types/base.ts';
import { runOutcomeTag } from '#lib/features/discovery/utils/outcome.ts';
import type { Host, HostVirtualization, IPAddress, Port } from '#lib/features/hosts/types/base.ts';
import type { Service, ServiceVirtualization } from '#lib/features/services/types/base.ts';
import type { Site } from '#lib/features/sites/types.ts';
import type { Tag, TagGroup } from '#lib/features/tags/types/base.ts';
import { hostDisplayName } from '#lib/features/hosts/host-display-name.ts';
import { hostOsIcon, hostOsLabel } from '#lib/features/hosts/host-os.ts';
import { applicationGroupId, tagIcon } from '#lib/features/tags/groups.ts';
import { cachedEntities, queryClient, queryKeys } from '#lib/api/query-client.ts';
import type {
	TooltipChip,
	TooltipContent,
	TooltipItem,
	TooltipRow,
	TooltipSection
} from '#lib/shared/actions/tooltip.ts';
import { entityRef, type EntityRef } from '#lib/shared/components/data/types.ts';
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
} from '#lib/shared/entity-fields/index.ts';
import {
	containerNetworkTypes,
	deviceTypes,
	entities,
	guestStates,
	hostVirtualizations,
	metadataColor,
	proxmoxGuestTypes,
	serviceCategories,
	serviceDefinitions,
	serviceVirtualizations
} from '#lib/shared/stores/metadata.ts';
import { formatPort, formatTimestamp } from '#lib/shared/utils/formatting.ts';
import type { Color } from '#lib/shared/utils/styling.ts';
import {
	common_category,
	common_created,
	common_daemons,
	common_host,
	common_hosts,
	common_hoursCount,
	common_lastRun,
	common_macAddress,
	common_manager,
	common_moreItems,
	common_name,
	common_operatingSystem,
	common_services,
	common_site,
	common_status,
	common_subnets,
	common_tags,
	common_virtualization,
	dependencies_memberCount,
	hosts_deviceFacts_composeProject,
	hosts_deviceFacts_guestType,
	hosts_deviceFacts_networkType,
	hosts_deviceType,
	sites_staleAfter
} from '#lib/paraglide/messages.js';

type DashboardSummary = components['schemas']['DashboardSummary'];
type Dependency = components['schemas']['Dependency'];
type Discovery = components['schemas']['Discovery'];
type Topology = components['schemas']['Topology'];

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

/** The most services a tooltip lists. It can't be scrolled, so the rest are counted. */
const MAX_TOOLTIP_SERVICES = 12;

/** A titled tooltip: one row per value present, then the services under their own heading. */
function titledTooltip(
	title: string,
	rows: MaybeRow[],
	services: (string | TooltipItem)[] = []
): TooltipContent {
	const present: TooltipRow[] = rows.flatMap(({ label, value, chips }) =>
		value?.trim() ? [{ label, value, chips }] : []
	);
	const sections: TooltipSection[] = [];
	if (present.length > 0) sections.push({ rows: present });
	if (services.length > 0) {
		const hidden = services.length - MAX_TOOLTIP_SERVICES;
		sections.push({
			heading: common_services(),
			items:
				hidden > 0
					? [...services.slice(0, MAX_TOOLTIP_SERVICES), common_moreItems({ count: hidden })]
					: services
		});
	}
	return { title, sections };
}

function hostRow(host: Host | null | undefined): MaybeRow {
	if (!host) return { label: common_host(), value: null };
	const name = hostDisplayName(host);
	return { label: common_host(), value: name, chips: [entityChip('Host', host, name)] };
}

/**
 * A service's hover text: its name as the title, the host it runs on when the card does not
 * already say it, its category and tags, and how it runs when it is containerized.
 */
export function serviceTooltip(
	service: Service,
	{
		host,
		name,
		lookup = NO_ENTITIES
	}: { host?: Host | null; name?: string; lookup?: EntityLookup } = {}
): TooltipContent {
	const category = serviceDefinitions.getCategory(service.service_definition);
	return titledTooltip(name ?? service.name, [
		hostRow(host),
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
 * An address's hover text: the address as the title, its name, MAC, host and subnet, then the
 * services bound there, each with its ports at that address.
 */
export function ipAddressTooltip(
	address: IPAddress,
	{
		name,
		services,
		lookup = NO_ENTITIES
	}: { name?: string; services: Service[]; lookup?: EntityLookup }
): TooltipContent {
	return titledTooltip(
		name ?? address.ip_address,
		[
			{ label: common_name(), value: address.name },
			{ label: common_macAddress(), value: address.mac_address },
			hostRow(byId<Host>(lookup, 'Host', address.host_id)),
			entityRow(
				entities.getName('Subnet'),
				'Subnet',
				byId<object>(lookup, 'Subnet', address.subnet_id)
			)
		],
		services.map((service) => serviceWithPorts(service, address.id, lookup))
	);
}

/**
 * A host's hover text: its name as the title, its device type and how it runs as a guest, its
 * tags, then its services.
 */
export function hostTooltip(
	host: Host,
	{
		name,
		deviceType,
		services,
		lookup = NO_ENTITIES
	}: {
		name?: string;
		deviceType?: string | null;
		services: Service[];
		lookup?: EntityLookup;
	}
): TooltipContent {
	const os = host.os;
	return titledTooltip(
		name ?? hostDisplayName(host),
		[
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
		services.map(serviceListItem)
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

/**
 * An entity's summary and tooltip fields, then the related entities `lookup` carries, then `rows`
 * and `services`: what a type adds that the registry has no field for.
 */
function registryTooltip(
	type: EntityType,
	entity: object,
	title: string,
	lookup: EntityLookup,
	{ rows = [], services = [] }: { rows?: MaybeRow[]; services?: TooltipItem[] } = {}
) {
	// A relation into a collection the lookup does not load would read as empty, so it is left out.
	// A list of related entities is shown only where the registry marks it a summary (a subnet's
	// VLANs); a long one (a VLAN's interfaces) is the inspector's.
	const fields = entityFields(type);
	const reachable = (f: EntityField) =>
		f.path.every((hop) => hop.type === 'Embedded' || lookup.loads(hop.entity));
	const fieldRows = fields
		.filter(
			(f) =>
				((f.summary || f.tooltip) && f.path.length === 0) ||
				(f.format.type === 'Entity' && (!f.many || f.summary) && reachable(f))
		)
		.map((f) => fieldRow(type, f, entity, lookup))
		.filter((row) => row.value !== title);
	return titledTooltip(title, [...fieldRows, ...rows], services);
}

/** A port's services, each with the addresses it listens on through this port. */
function portServices(port: Port, lookup: EntityLookup): TooltipItem[] {
	const services = lookup.byColumn('Service', 'host_id', port.host_id) as Service[];
	return services.flatMap((service) => {
		const bindings = (service.bindings ?? []).filter(
			(b) => b.type === 'Port' && b.port_id === port.id
		);
		if (bindings.length === 0) return [];
		const addresses = bindings.flatMap(
			(b) => byId<IPAddress>(lookup, 'IPAddress', b.ip_address_id)?.ip_address ?? []
		);
		return [{ ...serviceListItem(service), detail: addresses.join(', ') || undefined }];
	});
}

/** The subnets a daemon has an interface on. */
function daemonSubnetsRow(daemon: Daemon, lookup: EntityLookup): MaybeRow {
	const chips = (daemon.interfaced_subnet_ids ?? []).flatMap((id) => {
		const subnet = byId<object>(lookup, 'Subnet', id);
		return subnet ? [entityChip('Subnet', subnet)] : [];
	});
	return {
		label: common_subnets(),
		value: chips.map((chip) => chip.label).join(', '),
		chips
	};
}

/** When a discovery was created, and how its run ended if it has one. */
function discoveryRows(discovery: Discovery): MaybeRow[] {
	const outcome = runOutcomeTag(
		discovery.run_type.type === 'Historical' ? discovery.run_type.results : null,
		{ includeCompleted: true }
	);
	return [
		{ label: common_created(), value: formatTimestamp(discovery.created_at) },
		tagRow(common_lastRun(), outcome)
	];
}

/** What a site holds, from the home page's summary, and how long until its entities go stale. */
function siteRows(site: Site): MaybeRow[] {
	const dashboard = queryClient.getQueryData<DashboardSummary>(queryKeys.dashboard.summary());
	const summary = dashboard?.sites.find((s) => s.id === site.id);
	const hours = site.effective_stale_after_hours;
	return [
		{ label: common_hosts(), value: summary?.host_count.toString() },
		{ label: common_services(), value: summary?.service_count.toString() },
		{ label: common_subnets(), value: summary?.subnet_count.toString() },
		{ label: common_daemons(), value: summary?.daemon_count.toString() },
		{
			label: sites_staleAfter(),
			value: hours == null ? null : common_hoursCount({ hours })
		}
	];
}

/** A dependency's member count, as the sentence the app shows it in. */
function dependencyMemberCount(dependency: Dependency): TooltipSection {
	const members = dependency.members;
	const count =
		members?.type === 'Services'
			? members.service_ids.length
			: members?.type === 'Bindings'
				? members.binding_ids.length
				: 0;
	return { text: dependencies_memberCount({ count }) };
}

/**
 * Any entity's hover text, by its type: a host, service, port or address the way its card shows
 * it, and any other entity as its registry fields plus what its type adds. `name` replaces the
 * entity's own display text as the title.
 */
export function entityTooltip(
	entityType: string,
	entity: object,
	{ lookup, name }: { lookup: EntityLookup; name?: string }
): TooltipContent {
	const type = entityType as EntityType;
	const title = name ?? entityDisplayText(type, entity);
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
				name,
				lookup
			});
		}
		case 'Interface':
			return interfaceTooltip(entity, title, lookup);
		case 'IPAddress': {
			const address = entity as IPAddress;
			const services = lookup.byColumn('Service', 'host_id', address.host_id) as Service[];
			return ipAddressTooltip(address, {
				name,
				services: services.filter((s) =>
					(s.bindings ?? []).some((b) => b.ip_address_id == null || b.ip_address_id === address.id)
				),
				lookup
			});
		}
		case 'Port':
			return registryTooltip(type, entity, title, lookup, {
				services: portServices(entity as Port, lookup)
			});
		case 'Daemon':
			return registryTooltip(type, entity, title, lookup, {
				rows: [daemonSubnetsRow(entity as Daemon, lookup)]
			});
		case 'Discovery':
			return registryTooltip(type, entity, title, lookup, {
				rows: discoveryRows(entity as Discovery)
			});
		case 'Dependency': {
			const content = registryTooltip(type, entity, title, lookup);
			const members = dependencyMemberCount(entity as Dependency);
			return { ...content, sections: [...content.sections, members] };
		}
		case 'Site':
			return registryTooltip(type, entity, title, lookup, { rows: siteRows(entity as Site) });
		case 'Topology': {
			const sites = queryClient.getQueryData<Site[]>(queryKeys.sites.all) ?? [];
			const site = sites.find((s) => s.id === (entity as Topology).site_id);
			return registryTooltip(type, entity, title, lookup, {
				rows: [{ label: common_site(), value: site?.name }]
			});
		}
		default:
			return registryTooltip(type, entity, title, lookup);
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
		Tag: cachedEntities(queryKeys.tags.all),
		TagGroup: cachedEntities(queryKeys.tagGroups.all),
		Daemon: cachedEntities(queryKeys.daemons.all),
		Discovery: cachedEntities(queryKeys.discovery.all),
		Credential: cachedEntities(queryKeys.credentials.all),
		User: cachedEntities(queryKeys.users.all),
		DaemonApiKey: cachedEntities(queryKeys.apiKeys.all),
		UserApiKey: cachedEntities(queryKeys.userApiKeys.all)
	});
}

/**
 * The tooltip of a chip, titled as the chip reads: the cached entity when there is one, as a ref
 * can carry a partial object, else the ref's own data. Null when the ref carries nothing to
 * describe.
 */
export function entityRefTooltip(ref: EntityRef, label?: string): TooltipContent | null {
	const lookup = entityCacheLookup();
	const entity = lookup.byColumn(ref.entityType, 'id', ref.entityId)[0] ?? ref.data;
	return entity
		? entityTooltip(ref.entityType, entity, { lookup, name: label || undefined })
		: null;
}
