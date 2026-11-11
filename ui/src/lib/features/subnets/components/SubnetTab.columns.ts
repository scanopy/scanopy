import type { Snippet } from 'svelte';
import { utilizationRatio } from '../nesting';
import { isUserManagedSubnet } from '../queries';
import type { Subnet, SubnetResponse } from '../types/base';
import { entityRef } from '#lib/shared/components/data/types.ts';
import { defineRegistryFields } from '#lib/shared/entity-fields/table.ts';
import type { EntityLookup } from '#lib/shared/entity-fields/index.ts';
import { tagNames } from '#lib/features/tags/columns.ts';
import { siteItems } from '#lib/features/sites/columns.ts';
import { discoveryRunItems } from '#lib/features/discovery/columns.ts';
import type { Service } from '#lib/features/services/types/base.ts';
import type { Site } from '#lib/features/sites/types.ts';
import type { Tag } from '#lib/features/tags/types/base.ts';
import type { Discovery } from '#lib/features/discovery/types/base.ts';
import type { components, operations } from '#lib/api/schema';
import { fieldValueOptions, type FieldValueCount } from '#lib/shared/api/field-values.ts';
import {
	common_nested,
	common_site,
	common_source,
	common_tags,
	common_unknownSite,
	common_unknownEntity,
	common_allSites,
	subnets_managedBy,
	subnets_subnetType
} from '#lib/paraglide/messages.js';
import { concepts, entities, entitySources, subnetTypes } from '#lib/shared/stores/metadata.ts';
import { entitySourceItems } from '#lib/shared/utils/entity-source.ts';

type SubnetOrderField = components['schemas']['SubnetOrderField'];
/** The subnet list endpoint's query params, which the filters are sent and linked as. */
type SubnetListParam = keyof NonNullable<operations['list_subnets']['parameters']['query']>;

/** What the subnet columns read from the tab, each as a getter so values stay live. */
export interface SubnetColumnsContext {
	lookup: () => EntityLookup;
	sites: () => Site[];
	tags: () => Tag[];
	discoveryRuns: () => Discovery[];
	/** The ranges heading a nesting group, by id, including those on other pages. */
	nestingRoots: () => ReadonlyMap<string, Pick<Subnet, 'cidr' | 'name'>>;
	/** The container runtime that manages a subnet, if it has one. */
	runtimeOf: (subnet: Subnet) => Service | undefined;
	/** The container runtimes that manage some subnet. */
	runtimes: () => Service[];
	/** The utilization column's cell. */
	utilizationCell: Snippet<[SubnetResponse, string]>;
	siteValues: () => FieldValueCount[] | undefined;
	typeValues: () => FieldValueCount[] | undefined;
	sourceValues: () => FieldValueCount[] | undefined;
	managedByValues: () => FieldValueCount[] | undefined;
}

/**
 * Columns from the entity field registry; this adds how each one searches, filters and sorts.
 * Every SubnetOrderField value must be listed.
 */
export function subnetColumns(ctx: SubnetColumnsContext) {
	return defineRegistryFields<SubnetResponse, SubnetOrderField, SubnetListParam>(
		'Subnet',
		ctx.lookup,
		{
			// Identity fields: grouping by one would render a header per subnet.
			name: {
				searchable: true,
				groupable: false,
				display: { order: 0, primary: true, width: 220 }
			},
			cidr: {
				searchable: true,
				// Groups each range with every range nested inside it, as nested sections. The server
				// groups, orders and counts by the outermost range's id ('' for a range that nests
				// with nothing, which falls in the ungrouped bucket rather than heading a group of
				// one), in address order, each range directly before the ranges inside it.
				getGroupValue: (subnet) => subnet.nesting_root_id ?? '',
				getGroupLabel: (subnet) => {
					if (!subnet.nesting_root_id) return null;
					// Named as the root's own row is, so a tree continued from an earlier page reads as
					// the same tree.
					const root = ctx.nestingRoots().get(subnet.nesting_root_id);
					if (!root) return common_unknownEntity({ entity: entities.getName('Subnet') });
					return root.name || root.cidr;
				},
				tree: {
					key: (subnet) => subnet.id,
					parentKey: (subnet) => subnet.parent_subnet_id ?? null,
					depth: (subnet) => subnet.nesting_depth,
					rootKey: (subnet) => subnet.nesting_root_id ?? null,
					rootsLabel: () => common_nested()
				},
				display: { order: 3 }
			},
			subnet_type: {
				label: subnets_subnetType(),
				searchable: true,
				filterable: true,
				serverFiltered: true,
				queryParam: 'subnet_types',
				// The types some subnet has.
				filterOptions: fieldValueOptions(ctx.typeValues()),
				filterLabel: (id) => subnetTypes.getName(id),
				getGroupValue: (subnet) => subnet.subnet_type,
				display: {
					order: 4,
					getItems: (subnet) => [
						{
							id: subnet.subnet_type,
							...subnetTypes.getTag(subnet.subnet_type),
							icon: subnetTypes.getIconComponent(subnet.subnet_type)
						}
					]
				}
			},
			site_id: {
				label: common_site(),
				type: 'string',
				searchable: true,
				filterable: true,
				serverFiltered: true,
				queryParam: 'site_ids',
				// The sites some subnet is on.
				filterOptions: fieldValueOptions(ctx.siteValues()),
				filterLabel: (id) => ctx.sites().find((n) => n.id === id)?.name ?? common_unknownSite(),
				// Displayed as a name, but grouped by id on the server. The Internet and Remote rows stand
				// for the copy every site holds, so they read "All sites", not the listed copy's site.
				getGroupValue: (item) => item.site_id,
				getValue: (item) =>
					isUserManagedSubnet(item)
						? ctx.sites().find((n) => n.id == item.site_id)?.name || common_unknownSite()
						: common_allSites(),
				display: {
					order: 2,
					getItems: (item) =>
						isUserManagedSubnet(item)
							? siteItems(item.site_id, ctx.sites())
							: [
									{
										id: item.site_id,
										label: common_allSites(),
										color: entities.getColorHelper('Site').color
									}
								]
				}
			},
			created_at: {
				type: 'date',
				groupable: false,
				display: { hiddenByDefault: true }
			},
			updated_at: {
				type: 'date',
				groupable: false,
				display: { hiddenByDefault: true }
			},
			last_seen_at: {
				type: 'date',
				// Identity-like: one header per timestamp.
				groupable: false,
				staleFilter: true,
				display: {
					recency: true,
					order: 1
				}
			},
			utilization: {
				type: 'string',
				// Ordered by the server, which derives it across the whole site; a share per row
				// would group into a header per value.
				groupable: false,
				getValue: (subnet) => `${Math.round(utilizationRatio(subnet) * 100)}%`,
				display: { order: 5, cell: ctx.utilizationCell }
			},
			source: {
				label: common_source(),
				type: 'string',
				filterable: true,
				serverFiltered: true,
				queryParam: 'sources',
				// The sources some subnet was stamped with.
				filterOptions: fieldValueOptions(ctx.sourceValues()),
				filterLabel: (id) => entitySources.getName(id),
				// The server groups on the raw `source.type`.
				getGroupValue: (subnet) => subnet.source.type,
				getValue: (subnet) => entitySources.getName(subnet.source.type),
				display: {
					hiddenByDefault: true,
					getItems: (subnet) => entitySourceItems(subnet.source)
				}
			},
			managed_by: {
				// The container runtime that created this bridge network. Only bridge subnets have one.
				label: subnets_managedBy(),
				type: 'string',
				filterable: true,
				serverFiltered: true,
				queryParam: 'managed_by',
				// The runtimes that created some subnet.
				filterOptions: fieldValueOptions(ctx.managedByValues()),
				filterLabel: (id) => ctx.runtimes().find((s) => s.id === id)?.name ?? id,
				// Displayed as the runtime's name, but grouped by its id on the server.
				getGroupValue: (subnet) => subnet.virtualization_service_id ?? null,
				getValue: (subnet) => ctx.runtimeOf(subnet)?.name ?? null,
				display: {
					hiddenByDefault: true,
					getItems: (subnet) => {
						const runtime = ctx.runtimeOf(subnet);
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
			}
		},
		[
			{
				key: 'description',
				searchable: true
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
