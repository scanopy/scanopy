import { defineRegistryFields } from '#lib/shared/entity-fields/table.ts';
import type { EntityLookup } from '#lib/shared/entity-fields/index.ts';
import { siteItems } from '#lib/features/sites/columns.ts';
import type { components, operations } from '#lib/api/schema';
import type { FieldValueCount } from '#lib/shared/api/field-values.ts';
import { fieldValueOptions } from '#lib/shared/api/field-values.ts';
import type { Site } from '#lib/features/sites/types.ts';
import type { Tag } from '#lib/features/tags/types/base.ts';
import type { ApiKey } from '../types/base';
import {
	common_enabled,
	common_expired,
	common_expires,
	common_lastUsed,
	common_never,
	common_name,
	common_site,
	common_tags,
	common_unknownSite
} from '#lib/paraglide/messages.js';

type DaemonApiKeyOrderField = components['schemas']['DaemonApiKeyOrderField'];
/** The daemon API key list endpoint's query params, which the filters are sent and linked as. */
type DaemonApiKeyListParam = keyof NonNullable<
	operations['list_daemon_api_keys']['parameters']['query']
>;

/** What the daemon API key columns read from the tab, each as a getter so values stay live. */
export interface ApiKeyColumnsContext {
	/** Related entities the registry's key fields read: the site. */
	lookup: () => EntityLookup;
	sites: () => Site[];
	tags: () => Tag[];
	siteValues: () => FieldValueCount[] | undefined;
}

/** Every DaemonApiKeyOrderField value is listed; the server sorts, groups and filters each. */
export function apiKeyColumns(ctx: ApiKeyColumnsContext) {
	return defineRegistryFields<ApiKey, DaemonApiKeyOrderField, DaemonApiKeyListParam>(
		'DaemonApiKey',
		ctx.lookup,
		{
			name: {
				label: common_name(),
				type: 'string',
				searchable: true,
				// One header per key.
				groupable: false
			},
			site_id: {
				type: 'string',
				label: common_site(),
				filterable: true,
				serverFiltered: true,
				queryParam: 'site_ids',
				// The sites some key is on.
				filterOptions: fieldValueOptions(ctx.siteValues()),
				filterLabel: (id) => ctx.sites().find((n) => n.id === id)?.name ?? common_unknownSite(),
				// Displayed as a name, but grouped by id on the server.
				getGroupValue: (item) => item.site_id,
				getValue(item) {
					return ctx.sites().find((n) => n.id == item.site_id)?.name || common_unknownSite();
				},
				display: { getItems: (item) => siteItems(item.site_id, ctx.sites()) }
			},
			is_enabled: {
				label: common_enabled(),
				type: 'boolean',
				filterable: true,
				serverFiltered: true,
				groupable: true,
				getGroupValue: (key) => String(key.is_enabled ?? false),
				getValue: (key) => key.is_enabled ?? false
			},
			last_used: {
				label: common_lastUsed(),
				type: 'date',
				groupable: false,
				getValue: (key) => key.last_used ?? null,
				display: { recency: true }
			},
			expires_at: {
				label: common_expires(),
				type: 'date',
				groupable: false,
				getValue: (key) => key.expires_at ?? null,
				display: {
					// Expired reads as a state, not a date — the date has stopped being the useful
					// part once it has passed. Returning undefined shows the date itself.
					getItems: (key) =>
						!key.expires_at
							? [{ id: 'never', label: common_never(), color: 'Gray' }]
							: new Date(key.expires_at) < new Date()
								? [{ id: 'expired', label: common_expired(), color: 'Red' }]
								: undefined
				}
			},
			created_at: { type: 'date', groupable: false },
			updated_at: {
				type: 'date',
				groupable: false,
				display: { hiddenByDefault: true }
			}
		},
		[
			{
				key: 'tags',
				label: common_tags(),
				type: 'array',
				filterable: true,
				getValue: (entity) =>
					entity.tags
						.map((id) => ctx.tags().find((t) => t.id === id)?.name)
						.filter((name): name is string => !!name)
			}
		],
		// The tab lists only keys not bound to a daemon, so the daemon would be empty on every row.
		{ omit: ['daemon_id'] }
	);
}
