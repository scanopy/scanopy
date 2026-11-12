import type { Snippet } from 'svelte';
import type { Host, IPAddress, Port } from '../types/base';
import { hostDisplayName } from '#lib/features/hosts/host-display-name.ts';
import { hostOsFamilyIds, hostOsFamilyName } from '#lib/features/hosts/host-os.ts';
import { entityRef } from '#lib/shared/components/data/types.ts';
import { defineRegistryFields } from '#lib/shared/entity-fields/table.ts';
import type { EntityLookup } from '#lib/shared/entity-fields/index.ts';
import { tagNames } from '#lib/features/tags/columns.ts';
import { credentialItems } from '#lib/features/credentials/columns.ts';
import { deviceTypes, entities, entitySources, concepts } from '#lib/shared/stores/metadata.ts';
import { deviceTypeSourceTag } from '#lib/features/hosts/device-type-source.ts';
import {
	common_description,
	common_hostname,
	common_interfaces,
	common_ports,
	common_ipAddresses,
	common_manufacturer,
	common_model,
	common_name,
	common_site,
	common_serialNumber,
	common_assetTag,
	common_source,
	common_firmwareRevision,
	common_softwareRevision,
	common_operatingSystem,
	common_host,
	common_service,
	common_services,
	common_tags,
	common_unknownEntity,
	common_unknownSite,
	common_contact,
	common_location,
	hosts_fields_presentedBy,
	hosts_fields_virtualizedBy,
	hosts_notVirtualized,
	hosts_snmp_chassisId,
	hosts_snmp_managementUrl,
	hosts_snmp_sysDescr,
	hosts_snmp_sysName,
	hosts_snmp_sysObjectId
} from '#lib/paraglide/messages.js';
import { entitySourceItems } from '#lib/shared/utils/entity-source.ts';
import {
	virtualizationGroupKey,
	virtualizationGroupLabel,
	virtualizationTree
} from '../virtualization-tree';
import { formatPort } from '#lib/shared/utils/formatting.ts';
import { discoveryRunItems } from '#lib/features/discovery/columns.ts';
import { isContainerSubnet } from '#lib/features/subnets/queries.ts';
import type { Credential } from '#lib/features/credentials/types/base.ts';
import type { Interface } from '#lib/features/credentials/types/base.ts';
import { formatIPAddress } from '../address-labels';
import type { components, operations } from '#lib/api/schema';
import { NO_VALUE, valuesOrNoneCodec } from '#lib/shared/components/data/controls/listQuery.ts';
import {
	fieldValueOptions,
	hasEmptyFieldValue,
	type FieldValueCount
} from '#lib/shared/api/field-values.ts';
import type { Service } from '#lib/features/services/types/base.ts';
import type { Subnet } from '#lib/features/subnets/types/base.ts';
import type { Site } from '#lib/features/sites/types.ts';
import type { Tag } from '#lib/features/tags/types/base.ts';
import type { Discovery } from '#lib/features/discovery/types/base.ts';

type HostOrderField = components['schemas']['HostOrderField'];
/** The host list endpoint's query params, which the filters are sent and linked as. */
type HostListParam = keyof NonNullable<operations['list_hosts']['parameters']['query']>;

/** What the host columns read from the tab, each as a getter so values stay live. */
export interface HostColumnsContext {
	lookup: () => EntityLookup;
	/** The virtualizing services of the hosts on this page. */
	services: () => Service[];
	/** Every service in the cache. */
	allServices: () => Service[];
	ipAddresses: () => IPAddress[];
	ports: () => Port[];
	interfaces: () => Interface[];
	credentials: () => Credential[];
	subnets: () => Subnet[];
	sites: () => Site[];
	tags: () => Tag[];
	discoveryRuns: () => Discovery[];
	/** The hosts that head a virtualization tree, by id. */
	virtualizationRoots: () => ReadonlyMap<string, Host>;
	/** The host's IP addresses; the tab's row actions use it too. */
	hostIPAddresses: (host: Host) => IPAddress[];
	siteValues: () => FieldValueCount[] | undefined;
	virtualizedByValues: () => FieldValueCount[] | undefined;
	sourceValues: () => FieldValueCount[] | undefined;
	osFamilyValues: () => FieldValueCount[] | undefined;
	manufacturerValues: () => FieldValueCount[] | undefined;
	modelValues: () => FieldValueCount[] | undefined;
	sysLocationValues: () => FieldValueCount[] | undefined;
	serviceNameValues: () => FieldValueCount[] | undefined;
	osCell: Snippet<[Host, string]>;
}

/**
 * Columns from the entity field registry; this tab adds how each one searches, filters, sorts
 * and groups. Every HostOrderField value must be listed.
 */
export function hostColumns(ctx: HostColumnsContext) {
	// What a host holds, resolved here for the table's columns.
	function hostCredentials(host: Host): Credential[] {
		return (host.credential_assignments ?? [])
			.map((a) => ctx.credentials().find((c) => c.id === a.credential_id))
			.filter((c): c is Credential => c != null);
	}

	function hostInterfaces(host: Host): Interface[] {
		return ctx.interfaces().filter((i) => i.host_id === host.id);
	}

	function hostPorts(host: Host) {
		return ctx
			.ports()
			.filter((p) => p.host_id === host.id)
			.sort((a, b) => a.number - b.number);
	}

	/** The host's distinct MACs across its IP addresses and interfaces, lowest first — the
	 *  first is the one the server sorts the host by. */
	function hostMacAddresses(host: Host): string[] {
		const macs = [...ctx.hostIPAddresses(host), ...hostInterfaces(host)]
			.map((i) => i.mac_address?.toUpperCase())
			.filter((m): m is string => !!m);
		return [...new Set(macs)].sort();
	}

	function isContainerSubnetFn(subnetId: string): boolean {
		const subnet = ctx.subnets().find((s) => s.id === subnetId);
		return subnet ? isContainerSubnet(subnet) : false;
	}

	return defineRegistryFields<Host, HostOrderField, HostListParam>(
		'Host',
		ctx.lookup,
		{
			// Identity fields: grouping by one would render a header per host.
			name: {
				label: common_name(),
				searchable: true,
				groupable: false,
				// The title, not the stored `name`. `getValue` rather than `display.cell`
				// because this one accessor also feeds the row header cell and the row
				// checkbox's accessible name. A `cell` snippet would fix the cell and
				// leave those two rendering an empty string.
				//
				// The key stays `name`: it is the `HostOrderField` sent to the server, which
				// now orders by the same ladder this renders.
				getValue: (host) => hostDisplayName(host),
				display: {
					primary: true,
					width: 220,
					order: 0,
					// Only when the name shown is the stored one, not a rung further down the ladder.
					getSource: (host) => (host.display_name_rung === 'Name' ? host.name_source : null)
				}
			},
			hostname: {
				label: common_hostname(),
				searchable: true,
				groupable: false,
				display: { hiddenByDefault: true }
			},
			virtualized_by: {
				label: hosts_fields_virtualizedBy(),
				type: 'string',
				searchable: true,
				filterable: true,
				serverFiltered: true,
				// The names of the services that virtualize some host, and "Not Virtualized"
				// only when some host has no virtualizing service (counted as '').
				filterOptions: fieldValueOptions(ctx.virtualizedByValues()).concat(
					hasEmptyFieldValue(ctx.virtualizedByValues()) ? [NO_VALUE] : []
				),
				filterLabel: (value) => (value === NO_VALUE ? hosts_notVirtualized() : value),
				filterCodec: valuesOrNoneCodec<HostListParam>(
					'virtualization_service_names',
					'include_unvirtualized'
				),
				groupable: true,
				// Grouped as a tree: every host under the host at the top of its chain, parent
				// first. The server groups, orders and counts by the root's id ('' for a host in no
				// tree); the column itself still shows, sorts and filters on the immediate runtime.
				getGroupValue: virtualizationGroupKey,
				getGroupLabel: (host) =>
					virtualizationGroupLabel(host, ctx.virtualizationRoots(), {
						notVirtualized: hosts_notVirtualized(),
						unknownRoot: common_unknownEntity({ entity: common_host() })
					}),
				tree: virtualizationTree,
				getValue: (host) => {
					if (host.virtualization_service_id) {
						const virtualizationService = ctx
							.services()
							.find((s) => s.id === host.virtualization_service_id);
						if (virtualizationService) {
							return (
								virtualizationService?.name || common_unknownEntity({ entity: common_service() })
							);
						}
					}
					return hosts_notVirtualized();
				}
				// The registry's chips: none when a host isn't virtualized, so the cell renders
				// the em dash rather than repeating "Not Virtualized" down the column.
				// `getValue` keeps the phrase, so the filter still offers it.
			},
			interface_ip: {
				label: common_ipAddresses(),
				type: 'string',
				searchable: true,
				// Near-unique per host, so grouping by it is one header per host.
				groupable: false,
				getValue: (host) => {
					const iface = ctx
						.ipAddresses()
						.filter((i) => i.host_id === host.id)
						.sort((a, b) => (a.position ?? 0) - (b.position ?? 0))[0];
					return iface?.ip_address ?? '';
				},
				display: {
					order: 5,
					// The server orders on the primary address, but a host usually has
					// several — showing only the first would misrepresent the row.
					getItems: (host) =>
						ctx
							.hostIPAddresses(host)
							.sort((a, b) => (a.position ?? 0) - (b.position ?? 0))
							.map((i) => ({
								id: i.id,
								label: formatIPAddress(i, isContainerSubnetFn),
								color: entities.getColorHelper('IPAddress').color,
								entityRef: entityRef('IPAddress', i.id, i)
							}))
				}
			},
			mac_address: {
				type: 'string',
				searchable: true,
				groupable: false,
				getValue: (host) => hostMacAddresses(host)[0] ?? null,
				display: {
					order: 5,
					getItems: (host) =>
						hostMacAddresses(host).map((mac) => ({
							id: mac,
							label: mac
						}))
				}
			},
			site_id: {
				label: common_site(),
				type: 'string',
				searchable: true,
				filterable: true,
				serverFiltered: true,
				queryParam: 'site_ids',
				groupable: true,
				// The sites some host is on.
				filterOptions: fieldValueOptions(ctx.siteValues()),
				filterLabel: (id) => ctx.sites().find((n) => n.id === id)?.name ?? common_unknownSite(),
				// Displayed as a name, but grouped by id on the server.
				getGroupValue: (item) => item.site_id,
				getValue: (item) =>
					ctx.sites().find((n) => n.id == item.site_id)?.name || common_unknownSite(),
				// The registry's site chip.
				display: { order: 2 }
			},
			// Audit dates stay available but off by default: 12 columns at once
			// is unreadable, and these are rarely what someone is scanning for.
			created_at: { type: 'date', display: { hiddenByDefault: true } },
			updated_at: { type: 'date', display: { hiddenByDefault: true } },
			last_seen_at: {
				type: 'date',
				staleFilter: true,
				display: { recency: true, order: 1 }
			},
			// How the host came to exist, read from `source.type`. An inferred host (one a
			// neighbour advertised and nothing scanned) looks the same as a down device by its
			// ports and services, so the chip and filter read the stamped source, never those.
			source: {
				label: common_source(),
				filterable: true,
				serverFiltered: true,
				queryParam: 'sources',
				// The sources some host was stamped with.
				filterOptions: fieldValueOptions(ctx.sourceValues()),
				filterLabel: (id) => entitySources.getName(id),
				// The server groups on the raw `source.type`.
				getGroupValue: (host) => host.source.type,
				display: { order: 1, getItems: (host) => entitySourceItems(host.source) }
			},
			// Hardware identity, off by default: populated only for hosts a credentialed scan
			// reached. The options come from every stored value, and the server groups on the
			// stored string, which is also what renders.
			manufacturer: {
				label: common_manufacturer(),
				filterable: true,
				serverFiltered: true,
				queryParam: 'manufacturers',
				filterOptions: fieldValueOptions(ctx.manufacturerValues()),
				display: { hiddenByDefault: true }
			},
			model: {
				label: common_model(),
				filterable: true,
				serverFiltered: true,
				queryParam: 'models',
				filterOptions: fieldValueOptions(ctx.modelValues()),
				display: { hiddenByDefault: true }
			},
			sys_location: {
				label: common_location(),
				filterable: true,
				serverFiltered: true,
				queryParam: 'sys_locations',
				filterOptions: fieldValueOptions(ctx.sysLocationValues()),
				display: { hiddenByDefault: true }
			},
			// Sorted, grouped and filtered by family; the cell shows the full product and release.
			os_family: {
				label: common_operatingSystem(),
				filterable: true,
				serverFiltered: true,
				queryParam: 'os_families',
				// The families some host runs.
				filterOptions: fieldValueOptions(ctx.osFamilyValues()),
				filterLabel: (value) => {
					const family = hostOsFamilyIds.find((id) => id === value);
					return family ? hostOsFamilyName(family) : value;
				},
				getValue: (host) => (host.os ? hostOsFamilyName(host.os.family) : null),
				getGroupValue: (host) => host.os?.family ?? null,
				display: { hiddenByDefault: true, cell: ctx.osCell }
			}
		},
		[
			// Computed by the server, so it can be shown but not sorted or filtered.
			{
				key: 'device_type',
				type: 'string',
				getValue: (host: Host) => deviceTypes.getName(host.device_type ?? 'Unidentified'),
				// A tag, as the subnets table shows a subnet's type. Its hover text says where the type
				// came from, as other columns' hover text names their value's source.
				display: {
					getItems: (host: Host) => {
						const deviceType = host.device_type ?? 'Unidentified';
						const source = host.device_type_source
							? deviceTypeSourceTag(host.device_type_source)
							: null;
						return [
							{
								id: deviceType,
								...deviceTypes.getTag(deviceType),
								icon: deviceTypes.getIconComponent(deviceType),
								title: source ? `${source.label}\n${source.title}` : undefined
							}
						];
					}
				}
			},
			{
				key: 'description',
				label: common_description(),
				searchable: true,
				display: { hiddenByDefault: true }
			},
			// Off by default: populated only for hosts a credentialed scan reached. Identifiers,
			// vendor version strings, opaque OIDs and free text, so none sorts, groups or filters.
			{
				key: 'serial_number',
				label: common_serialNumber(),
				display: { hiddenByDefault: true }
			},
			{ key: 'asset_tag', label: common_assetTag(), display: { hiddenByDefault: true } },
			{
				key: 'firmware_revision',
				label: common_firmwareRevision(),
				display: { hiddenByDefault: true }
			},
			{
				key: 'software_revision',
				label: common_softwareRevision(),
				display: { hiddenByDefault: true }
			},
			{ key: 'sys_name', label: hosts_snmp_sysName(), display: { hiddenByDefault: true } },
			{ key: 'sys_descr', label: hosts_snmp_sysDescr(), display: { hiddenByDefault: true } },
			{
				key: 'sys_object_id',
				label: hosts_snmp_sysObjectId(),
				display: { hiddenByDefault: true }
			},
			{ key: 'sys_contact', label: common_contact(), display: { hiddenByDefault: true } },
			{ key: 'chassis_id', label: hosts_snmp_chassisId(), display: { hiddenByDefault: true } },
			{
				key: 'management_url',
				label: hosts_snmp_managementUrl(),
				display: { hiddenByDefault: true }
			},
			// The registry's chip for the interface on the virtualizing host.
			{
				key: 'presented_by',
				label: hosts_fields_presentedBy(),
				display: { hiddenByDefault: true }
			},
			{
				key: 'first_found_by',
				type: 'string',
				display: {
					hiddenByDefault: true,
					getItems: (host) => discoveryRunItems(host.first_discovery_id, ctx.discoveryRuns())
				}
			},
			{
				key: 'last_found_by',
				type: 'string',
				display: {
					hiddenByDefault: true,
					getItems: (host) => discoveryRunItems(host.last_discovery_id, ctx.discoveryRuns())
				}
			},
			{
				key: 'tags',
				label: common_tags(),
				type: 'array',
				searchable: true,
				filterable: true,
				// Search and filter match the same names the cell renders, so a row
				// can never show a tag the filter above it disagrees with.
				getValue: (entity) => tagNames(entity.tags, ctx.tags())
			},
			{
				key: 'credentials',
				type: 'array',
				searchable: true,
				filterable: true,
				serverFiltered: true,
				queryParam: 'credential_ids',
				// Only credentials assigned to some host: the rest match nothing.
				filterOptions: ctx
					.credentials()
					.filter((c) => (c.host_assignments ?? []).length > 0)
					.map((c) => c.id),
				filterLabel: (id) => ctx.credentials().find((c) => c.id === id)?.name ?? id,
				getValue: (host) => hostCredentials(host).map((c) => c.name),
				getFilterValue: (host) => hostCredentials(host).map((c) => c.id),
				display: {
					order: 3,
					getItems: (host) => credentialItems(hostCredentials(host))
				}
			},
			// The registry's interface names and chips.
			{
				key: 'interfaces',
				label: common_interfaces(),
				searchable: true,
				display: { order: 4 }
			},
			{
				// Off by default: Services already says what answers on a port, and the open ports
				// alone matter when nothing was matched to them.
				key: 'ports',
				label: common_ports(),
				searchable: true,
				// Lowest port first, which the registry's collection order doesn't give.
				getValue: (host) => hostPorts(host).map((p) => formatPort(p)),
				display: {
					hiddenByDefault: true,
					getItems: (host) =>
						hostPorts(host).map((port) => ({
							id: port.id,
							label: formatPort(port),
							color: entities.getColorHelper('Port').color,
							// What the popover needs to name the service on the port and its addresses.
							entityRef: entityRef('Port', port.id, port)
						}))
				}
			},
			{
				key: 'services',
				label: common_services(),
				searchable: true,
				filterable: true,
				serverFiltered: true,
				queryParam: 'service_names',
				// Names, not ids: the server matches a host's services by name. The options are
				// every live service name the server counts; the services cache only holds the
				// loaded page's.
				filterOptions: fieldValueOptions(ctx.serviceNameValues()),
				display: {
					order: 6,
					// Containers are services too, so they share this column rather
					// than getting one of their own; the colour carries the
					// distinction instead of a second header.
					getItems: (host) =>
						ctx
							.allServices()
							.filter((s) => s.host_id === host.id)
							.map((s) => ({
								id: s.id,
								label: s.name,
								color: s.virtualization_metadata
									? concepts.getColorHelper('Containerization').color
									: entities.getColorHelper('Service').color,
								entityRef: entityRef('Service', s.id, s)
							}))
				}
			}
		]
	);
}
