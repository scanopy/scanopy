import { entityRef, type LabelledCardFieldItem } from '#lib/shared/components/data/types.ts';
import { tagNames } from '#lib/features/tags/columns.ts';
import {
	entities,
	entitySources,
	guestStates,
	matchConfidences
} from '#lib/shared/stores/metadata.ts';
import { entitySourceItems } from '#lib/shared/utils/entity-source.ts';
import { discoveryRunItems } from '#lib/features/discovery/columns.ts';
import { hostDisplayName } from '#lib/features/hosts/host-display-name.ts';
import { defineRegistryFields } from '#lib/shared/entity-fields/table.ts';
import type { EntityLookup } from '#lib/shared/entity-fields/index.ts';
import type { components, operations } from '#lib/api/schema';
import { NO_VALUE, valuesOrNoneCodec } from '#lib/shared/components/data/controls/listQuery.ts';
import type { Host } from '#lib/features/hosts/types/base.ts';
import type { Site } from '#lib/features/sites/types.ts';
import type { Tag } from '#lib/features/tags/types/base.ts';
import type { Discovery } from '#lib/features/discovery/types/base.ts';
import type { Service } from '../types/base';
import {
	common_containerizedBy,
	common_host,
	common_tags,
	common_type,
	common_portBindings,
	common_ipAddressBindings,
	common_unknown,
	common_unknownEntity,
	common_unknownSite,
	services_notContainerized,
	services_notDiscovered,
	meta_entities_Service_field_guest_state_label
} from '#lib/paraglide/messages.js';
import {
	serviceDefinitions,
	serviceCategories as serviceCategoryMeta,
	concepts
} from '#lib/shared/stores/metadata.ts';
import {
	fieldValueOptions,
	hasEmptyFieldValue,
	type FieldValueCount
} from '#lib/shared/api/field-values.ts';

type ServiceOrderField = components['schemas']['ServiceOrderField'];
/** The service list endpoint's query params, which the filters are sent and linked as. */
type ServiceListParam = keyof NonNullable<operations['list_services']['parameters']['query']>;

/** What the service columns read from the tab, each as a getter so values stay live. */
export interface ServiceColumnsContext {
	lookup: () => EntityLookup;
	/** The host each visible service runs on, by service id. */
	serviceHosts: () => Map<string, Host | undefined>;
	sites: () => Site[];
	tags: () => Tag[];
	discoveryRuns: () => Discovery[];
	/** The runtimes containerizing the visible services. */
	runtimes: () => Service[];
	/** Server-counted filter options, one per server-filtered field. */
	/** The hosts some service runs on, by id, and each one's title. */
	hostOptions: () => string[];
	hostLabel: (id: string) => string;
	siteValues: () => FieldValueCount[] | undefined;
	definitionValues: () => FieldValueCount[] | undefined;
	containerizedByValues: () => FieldValueCount[] | undefined;
	matchConfidenceValues: () => FieldValueCount[] | undefined;
	sourceValues: () => FieldValueCount[] | undefined;
	wellKnownPortNumbers: () => string[];
	serviceCategories: () => string[];
	portBindingItems: (service: Service) => LabelledCardFieldItem[];
	ipBindingItems: (service: Service) => LabelledCardFieldItem[];
}

/**
 * Columns from the entity field registry; this adds how each one searches, filters and sorts,
 * and keeps the tab's own cells where they show more than the registry's. Every
 * ServiceOrderField value must be listed.
 */
export function serviceColumns(ctx: ServiceColumnsContext) {
	return defineRegistryFields<Service, ServiceOrderField, ServiceListParam>(
		'Service',
		ctx.lookup,
		{
			// Identity field: grouping by it would render a header per service.
			name: {
				searchable: true,
				groupable: false,
				display: { order: 0, primary: true, width: 220 }
			},
			host: {
				type: 'string',
				searchable: true,
				filterable: true,
				serverFiltered: true,
				queryParam: 'host_ids',
				// The hosts some service runs on.
				filterOptions: ctx.hostOptions(),
				filterLabel: ctx.hostLabel,
				groupable: true,
				// The server groups on the host's title — the same full ladder rendered below —
				// coalescing services with no host to an empty string. Both sides have to walk
				// every rung or two hosts the server lumps into one group get two alternating
				// headers drawn over their interleaved rows.
				getGroupValue: (service) => {
					const host = ctx.serviceHosts().get(service.id);
					return host ? hostDisplayName(host) : '';
				},
				getValue: (service) => {
					const host = ctx.serviceHosts().get(service.id);
					return host ? hostDisplayName(host) : common_unknownEntity({ entity: common_host() });
				},
				display: {
					order: 3,
					getItems: (service) => {
						const host = ctx.serviceHosts().get(service.id);
						if (!host) return [];
						return [
							{
								id: host.id,
								label: hostDisplayName(host),
								color: entities.getColorHelper('Host').color,
								entityRef: entityRef('Host', host.id, host)
							}
						];
					}
				}
			},
			site_id: {
				type: 'string',
				searchable: true,
				filterable: true,
				serverFiltered: true,
				queryParam: 'site_ids',
				// The sites some service is on.
				filterOptions: fieldValueOptions(ctx.siteValues()),
				filterLabel: (id) => ctx.sites().find((n) => n.id === id)?.name ?? common_unknownSite(),
				groupable: true,
				// Displayed as a name, but grouped by id on the server.
				getGroupValue: (item) => item.site_id,
				getValue: (item) =>
					ctx.sites().find((n) => n.id == item.site_id)?.name || common_unknownSite(),
				display: { order: 2 }
			},
			// Per-service ordinal, so grouping by it is one header per service.
			position: {
				groupable: false,
				display: { hiddenByDefault: true, align: 'right' }
			},
			service_definition: {
				label: common_type(),
				searchable: true,
				filterable: true,
				serverFiltered: true,
				queryParam: 'service_definitions',
				// The definitions some service matched.
				filterOptions: fieldValueOptions(ctx.definitionValues()),
				filterLabel: (id) => serviceDefinitions.getName(id),
				groupable: true,
				// The server groups on the raw definition id; the UI renders its
				// friendly name, so the group key has to be supplied separately.
				getGroupValue: (service) => service.service_definition,
				display: {
					order: 4,
					getItems: (service) => [
						{
							id: service.service_definition,
							...serviceDefinitions.getTag(service.service_definition),
							icon: serviceDefinitions.getIconComponent(service.service_definition)
						}
					]
				}
			},
			created_at: { type: 'date', display: { hiddenByDefault: true } },
			updated_at: { type: 'date', display: { hiddenByDefault: true } },
			// Staleness rides on the date rather than a Status column of its own: a
			// service has no status, and `getFreshnessTag` returns a tag only when
			// the row is past its site's window — so the column was empty on
			// every healthy service. `getItems` returning undefined falls back to
			// the date.
			last_seen_at: {
				type: 'date',
				staleFilter: true,
				display: {
					recency: true,
					order: 1
				}
			},
			containerized_by: {
				type: 'string',
				label: common_containerizedBy(),
				searchable: true,
				filterable: true,
				serverFiltered: true,
				// The names of the runtimes that containerize some service, and "Not
				// Containerized" only when some service has none (counted as '').
				filterOptions: fieldValueOptions(ctx.containerizedByValues()).concat(
					hasEmptyFieldValue(ctx.containerizedByValues()) ? [NO_VALUE] : []
				),
				filterLabel: (value) => (value === NO_VALUE ? services_notContainerized() : value),
				filterCodec: valuesOrNoneCodec<ServiceListParam>(
					'virtualization_service_names',
					'include_uncontainerized'
				),
				getValue: (item) =>
					ctx.runtimes().find((s) => s.id == item.virtualization_service_id)?.name ||
					services_notContainerized(),
				// The server groups on the containerizing service's name, coalescing
				// services without one to an empty string.
				getGroupValue: (item) =>
					ctx.runtimes().find((s) => s.id == item.virtualization_service_id)?.name ?? '',
				display: {
					hiddenByDefault: true,
					// No chip when a service isn't containerized, so the cell shows an
					// em dash rather than repeating the phrase down the column. The
					// phrase stays in `getValue`, so the filter still offers it.
					getItems: (item) => {
						const runtime = ctx.runtimes().find((s) => s.id == item.virtualization_service_id);
						if (!runtime) return [];
						return [
							{
								id: runtime.id,
								label: runtime.name,
								color: concepts.getColorHelper('Containerization').color,
								entityRef: entityRef('Service', runtime.id, runtime)
							}
						];
					}
				}
			},
			match_confidence: {
				type: 'string',
				searchable: true,
				filterable: true,
				serverFiltered: true,
				queryParam: 'match_confidences',
				// The confidences some match carries.
				filterOptions: fieldValueOptions(ctx.matchConfidenceValues()),
				filterLabel: (id) => matchConfidences.getName(id),
				getValue: (item) =>
					item.source.type == 'DiscoveryWithMatch'
						? matchConfidences.getName(item.source.details.confidence)
						: services_notDiscovered(),
				// The server groups on the raw confidence id, null for services matched
				// without one.
				getGroupValue: (item) =>
					item.source.type == 'DiscoveryWithMatch' ? item.source.details.confidence : null,
				display: { hiddenByDefault: true }
			},
			source: {
				filterable: true,
				serverFiltered: true,
				queryParam: 'sources',
				// The sources some service was stamped with.
				filterOptions: fieldValueOptions(ctx.sourceValues()),
				filterLabel: (id) => entitySources.getName(id),
				// The server groups on the raw `source.type`.
				getGroupValue: (service) => service.source.type,
				display: {
					hiddenByDefault: true,
					getItems: (service) => entitySourceItems(service.source)
				}
			}
		},
		[
			{
				key: 'guest_state',
				label: meta_entities_Service_field_guest_state_label(),
				filterable: true,
				serverFiltered: true,
				queryParam: 'guest_states',
				// Every state the backend defines.
				filterOptions: guestStates.getItems().map((state) => state.id),
				filterLabel: (id) => guestStates.getName(id),
				getValue: (service) => {
					const state = service.virtualization_metadata?.details.state;
					return state ? guestStates.getName(state) : null;
				},
				display: {
					hiddenByDefault: true,
					getItems: (service) => {
						const state = service.virtualization_metadata?.details.state;
						return state
							? [
									{
										id: state,
										...guestStates.getTag(state),
										icon: guestStates.getIconComponent(state)
									}
								]
							: [];
					}
				}
			},
			{
				key: 'port_bindings',
				label: common_portBindings(),
				type: 'array',
				searchable: true,
				// Filters by port number, which the server matches against each
				// binding's port, so the options are numbers while the cells show
				// the full binding.
				filterable: true,
				serverFiltered: true,
				queryParam: 'ports',
				filterOptions: ctx.wellKnownPortNumbers(),
				getValue: (service) => ctx.portBindingItems(service).map((b) => b.label),
				display: { hiddenByDefault: true, getItems: ctx.portBindingItems }
			},
			{
				key: 'ip_bindings',
				label: common_ipAddressBindings(),
				type: 'array',
				searchable: true,
				getValue: (service) => ctx.ipBindingItems(service).map((b) => b.label),
				display: { hiddenByDefault: true, getItems: ctx.ipBindingItems }
			},
			{
				key: 'category',
				type: 'string',
				searchable: true,
				filterable: true,
				serverFiltered: true,
				queryParam: 'exclude_categories',
				filterMode: 'exclude',
				filterOptions: ctx.serviceCategories(),
				filterLabel: (category) => serviceCategoryMeta.getName(category),
				filterDefaults: ['OpenPorts'],
				getValue: (item) =>
					serviceDefinitions.getCategory(item.service_definition) || common_unknown(),
				display: {
					order: 5,
					getItems: (item) => {
						const category = serviceDefinitions.getCategory(item.service_definition);
						if (!category) return [];
						// Categories carry their own colour in the metadata fixture, so
						// use it rather than rendering every category identically grey.
						return [
							{
								id: category,
								...serviceCategoryMeta.getTag(category),
								icon: serviceCategoryMeta.getIconComponent(category)
							}
						];
					}
				}
			},
			{
				key: 'first_found_by',
				type: 'string',
				display: {
					hiddenByDefault: true,
					getItems: (item) => discoveryRunItems(item.first_discovery_id, ctx.discoveryRuns())
				}
			},
			{
				key: 'last_found_by',
				type: 'string',
				display: {
					hiddenByDefault: true,
					getItems: (item) => discoveryRunItems(item.last_discovery_id, ctx.discoveryRuns())
				}
			},
			{
				key: 'tags',
				label: common_tags(),
				type: 'array',
				searchable: true,
				filterable: true,
				getValue: (entity) => tagNames(entity.tags, ctx.tags())
			}
		]
	);
}
