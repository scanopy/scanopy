import { defineRegistryFields } from '$lib/shared/entity-fields/table';
import type { EntityLookup } from '$lib/shared/entity-fields';
import { entitySources } from '$lib/shared/stores/metadata';
import { entitySourceItems } from '$lib/shared/utils/entity-source';
import { tagNames } from '$lib/features/tags/columns';
import { discoveryRunItems } from '$lib/features/discovery/columns';
import type { Site } from '$lib/features/sites/types';
import type { Subnet } from '$lib/features/subnets/types/base';
import type { Tag } from '$lib/features/tags/types/base';
import type { Discovery } from '$lib/features/discovery/types/base';
import type { Vlan, VlanOrderField } from '../types/base';
import type { operations } from '$lib/api/schema';
import { fieldValueOptions, type FieldValueCount } from '$lib/shared/api/field-values';
import {
	common_site,
	common_source,
	common_tags,
	common_unknownSite,
	vlans_vlanNumber
} from '$lib/paraglide/messages';

/** What the VLAN columns read from the tab, each as a getter so values stay live. */
export interface VlanColumnsContext {
	lookup: () => EntityLookup;
	sites: () => Site[];
	/** Every subnet a VLAN can be filtered by. */
	subnets: () => Subnet[];
	tags: () => Tag[];
	discoveryRuns: () => Discovery[];
	siteValues: () => FieldValueCount[] | undefined;
	sourceValues: () => FieldValueCount[] | undefined;
}

/** The VLAN list endpoint's query params, which the filters are sent and linked as. */
type VlanListParam = keyof NonNullable<operations['list_vlans']['parameters']['query']>;

/**
 * Columns from the entity field registry; this adds how each one searches, filters, sorts and
 * groups. Every VlanOrderField value must be listed.
 */
export function vlanColumns(ctx: VlanColumnsContext) {
	return defineRegistryFields<Vlan, VlanOrderField, VlanListParam>(
		'Vlan',
		ctx.lookup,
		{
			// Identity fields: grouping by one would render a header per VLAN.
			vlan_number: {
				label: vlans_vlanNumber(),
				searchable: true,
				groupable: false
			},
			name: {
				searchable: true,
				groupable: false,
				display: { primary: true, width: 220 }
			},
			site_id: {
				label: common_site(),
				type: 'string',
				filterable: true,
				serverFiltered: true,
				queryParam: 'site_ids',
				// The sites some VLAN is on.
				filterOptions: fieldValueOptions(ctx.siteValues()),
				filterLabel: (id) => ctx.sites().find((n) => n.id === id)?.name ?? common_unknownSite(),
				// Displayed as a name, but grouped by id on the server.
				getGroupValue: (item) => item.site_id,
				getValue: (item) =>
					ctx.sites().find((n) => n.id == item.site_id)?.name || common_unknownSite()
			},
			source: {
				label: common_source(),
				type: 'string',
				filterable: true,
				serverFiltered: true,
				queryParam: 'sources',
				// The sources some VLAN was stamped with.
				filterOptions: fieldValueOptions(ctx.sourceValues()),
				filterLabel: (id) => entitySources.getName(id),
				// The server groups on the raw `source.type`.
				getGroupValue: (vlan) => vlan.source?.type ?? null,
				getValue: (vlan) => (vlan.source ? entitySources.getName(vlan.source.type) : null),
				display: {
					hiddenByDefault: true,
					getItems: (vlan) => (vlan.source ? entitySourceItems(vlan.source) : [])
				}
			},
			last_seen_at: {
				type: 'date',
				// Identity-like: one header per timestamp.
				groupable: false,
				display: { recency: true }
			},
			created_at: { type: 'date', display: { hiddenByDefault: true } },
			updated_at: { type: 'date', display: { hiddenByDefault: true } }
		},
		[
			{ key: 'description', searchable: true },
			{
				key: 'subnet_ids',
				filterable: true,
				serverFiltered: true,
				queryParam: 'subnet_ids',
				filterOptions: ctx.subnets().map((subnet) => subnet.id)
			},
			// Not filterable: a VLAN is served by many interfaces, and the options would be every
			// interface in the organization.
			{ key: 'interface_ids' },
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
