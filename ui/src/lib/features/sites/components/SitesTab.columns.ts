import { credentialItems } from '#lib/features/credentials/columns.ts';
import { tagNames } from '#lib/features/tags/columns.ts';
import { entityRef } from '#lib/shared/components/data/types.ts';
import { defineRegistryFields } from '#lib/shared/entity-fields/table.ts';
import type { EntityLookup } from '#lib/shared/entity-fields/index.ts';
import type { components, operations } from '#lib/api/schema';
import { fieldValueOptions, type FieldValueCount } from '#lib/shared/api/field-values.ts';
import { entities } from '#lib/shared/stores/metadata.ts';
import type { Site } from '../types';
import type { Credential } from '#lib/features/credentials/types/base.ts';
import type { Daemon } from '#lib/features/daemons/types/base.ts';
import type { Subnet } from '#lib/features/subnets/types/base.ts';
import type { Tag } from '#lib/features/tags/types/base.ts';
import type { Vlan } from '#lib/features/vlans/types/base.ts';
import {
	common_name,
	common_tags,
	common_hoursCount,
	sites_staleAfterDefault
} from '#lib/paraglide/messages.js';

/** What the site columns read from the tab, each as a getter so values stay live. */
export interface SiteColumnsContext {
	/** Related entities the registry's site fields read; none beyond the site itself. */
	lookup: () => EntityLookup;
	tags: () => Tag[];
	daemons: () => Daemon[];
	/** The hosts the daemons run on, for each daemon chip's host name. */
	/** Already narrowed to user-managed subnets. */
	subnets: () => Subnet[];
	vlans: () => Vlan[];
	credentials: () => Credential[];
	staleWindowValues: () => FieldValueCount[] | undefined;
}

type SiteOrderField = components['schemas']['SiteOrderField'];
/** The sites list endpoint's query params, which the filters are sent and linked as. */
type SiteListParam = keyof NonNullable<operations['list_sites']['parameters']['query']>;

// What a site contains, one resolver per column.
function siteDaemons(ctx: SiteColumnsContext, site: Site): Daemon[] {
	return ctx.daemons().filter((daemon) => daemon.site_id === site.id);
}

function siteSubnets(ctx: SiteColumnsContext, site: Site): Subnet[] {
	return ctx.subnets().filter((subnet) => subnet.site_id === site.id);
}

function siteVlans(ctx: SiteColumnsContext, site: Site): Vlan[] {
	return ctx.vlans().filter((vlan) => vlan.site_id === site.id);
}

function siteCredentials(ctx: SiteColumnsContext, site: Site): Credential[] {
	return (site.credential_ids ?? [])
		.map((id) => ctx.credentials().find((c) => c.id === id))
		.filter((c): c is Credential => Boolean(c));
}

/** A site's staleness window for its cell: its hours, prefixed when the server default applies. */
const DEFAULT_WINDOW_PREFIX = 'default:';

function staleWindowValue(site: Site): string | null {
	if (site.effective_stale_after_hours == null) return null;
	const hours = String(site.effective_stale_after_hours);
	return site.stale_after_hours == null ? `${DEFAULT_WINDOW_PREFIX}${hours}` : hours;
}

function staleWindowLabel(value: string): string {
	const isDefault = value.startsWith(DEFAULT_WINDOW_PREFIX);
	const hours = common_hoursCount({
		hours: Number(isDefault ? value.slice(DEFAULT_WINDOW_PREFIX.length) : value)
	});
	return isDefault ? sites_staleAfterDefault({ hours }) : hours;
}

/** Every SiteOrderField value is listed; the server sorts, groups and filters each. */
export function siteColumns(ctx: SiteColumnsContext) {
	return defineRegistryFields<Site, SiteOrderField, SiteListParam>(
		'Site',
		ctx.lookup,
		{
			name: {
				label: common_name(),
				type: 'string',
				searchable: true,
				// One header per site.
				groupable: false,
				display: { primary: true, width: 220, order: 0 }
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
			// The effective window, with the server default applied, so a site that
			// never set its own still shows the number staleness is judged by.
			effective_stale_after_hours: {
				type: 'string',
				filterable: true,
				serverFiltered: true,
				// The windows some site judges by.
				filterOptions: fieldValueOptions(ctx.staleWindowValues()),
				filterLabel: (hours) => common_hoursCount({ hours: Number(hours) }),
				getFilterValue: (site) => String(site.effective_stale_after_hours),
				// The server groups on the hours.
				getGroupValue: (site) => String(site.effective_stale_after_hours),
				// Marked when no override is set, so an override equal to the default still reads as one.
				getValue: (site) => {
					const value = staleWindowValue(site);
					return value === null ? null : staleWindowLabel(value);
				},
				display: { hiddenByDefault: true }
			}
		},
		[
			{
				key: 'vlans',
				type: 'array',
				getValue: (site) => siteVlans(ctx, site).map((v) => v.name),
				display: {
					order: 1,
					getItems: (site) =>
						siteVlans(ctx, site).map((vlan) => ({
							id: vlan.id,
							label: vlan.name,
							color: entities.getColorHelper('Vlan').color,
							entityRef: entityRef('Vlan', vlan.id, vlan)
						}))
				}
			},
			{
				key: 'tags',
				label: common_tags(),
				type: 'array',
				filterable: true,
				getValue: (entity) => tagNames(entity.tags, ctx.tags())
			},
			{
				key: 'daemons',
				type: 'array',
				getValue: (site) => siteDaemons(ctx, site).map((d) => d.name),
				display: {
					order: 3,
					getItems: (site) =>
						siteDaemons(ctx, site).map((daemon) => ({
							id: daemon.id,
							label: daemon.name,
							color: entities.getColorHelper('Daemon').color,
							entityRef: entityRef('Daemon', daemon.id, daemon)
						}))
				}
			},
			{
				key: 'credentials',
				type: 'array',
				// Credentials are shared across sites, so this filters. The other arrays here hold
				// members of one site each, which search already finds.
				filterable: true,
				serverFiltered: true,
				queryParam: 'credential_ids',
				filterOptions: ctx.credentials().map((credential) => credential.id),
				getValue: (site) => siteCredentials(ctx, site).map((c) => c.name),
				getFilterValue: (site) => siteCredentials(ctx, site).map((c) => c.id),
				filterLabel: (id) => ctx.credentials().find((c) => c.id === id)?.name ?? id,
				display: {
					order: 4,
					getItems: (site) => credentialItems(siteCredentials(ctx, site))
				}
			},
			{
				key: 'subnets',
				type: 'array',
				getValue: (site) => siteSubnets(ctx, site).map((s) => s.name),
				display: {
					order: 2,
					getItems: (site) =>
						siteSubnets(ctx, site).map((subnet) => ({
							id: subnet.id,
							label: subnet.name,
							color: entities.getColorHelper('Subnet').color,
							entityRef: entityRef('Subnet', subnet.id, subnet)
						}))
				}
			}
		],
		// The table shows the window in effect, the site's own or the organization default, as
		// `effective_stale_after_hours`.
		{ omit: ['stale_after_hours'] }
	);
}
