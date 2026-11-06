import { defineRegistryFields } from '$lib/shared/entity-fields/table';
import type { EntityLookup } from '$lib/shared/entity-fields';
import { siteItems } from '$lib/features/sites/columns';
import { entities, permissions } from '$lib/shared/stores/metadata';
import type { LabelledCardFieldItem } from '$lib/shared/components/data/types';
import { NO_VALUE } from '$lib/shared/components/data/controls/listQuery';
import type { components, operations } from '$lib/api/schema';
import type { Site } from '$lib/features/sites/types';
import type { Tag } from '$lib/features/tags/types/base';
import type { UserApiKey } from '../queries';
import {
	common_allSites,
	common_enabled,
	common_expired,
	common_expires,
	common_lastUsed,
	common_never,
	common_name,
	common_permissions,
	common_tags
} from '$lib/paraglide/messages';

type UserApiKeyOrderField = components['schemas']['UserApiKeyOrderField'];
/** The user API key list endpoint's query params, which the filters are sent and linked as. */
type UserApiKeyListParam = keyof NonNullable<
	operations['list_user_api_keys']['parameters']['query']
>;

/** What the user API key columns read from the tab, each as a getter so values stay live. */
export interface UserApiKeyColumnsContext {
	/** Related entities the registry's key fields read; none beyond the key itself. */
	lookup: () => EntityLookup;
	sites: () => Site[];
	tags: () => Tag[];
}

/**
 * The sites a key reaches. An All sites key (`site_ids` null) reads as one chip rather than a list
 * of today's sites, since it also reaches sites created later.
 */
function keySiteItems(ctx: UserApiKeyColumnsContext, key: UserApiKey): LabelledCardFieldItem[] {
	if (key.site_ids === null) {
		return [
			{ id: NO_VALUE, label: common_allSites(), color: entities.getColorHelper('Site').color }
		];
	}
	return siteItems(key.site_ids, ctx.sites());
}

/** Every UserApiKeyOrderField value is listed; the server sorts, groups and filters each. */
export function userApiKeyColumns(ctx: UserApiKeyColumnsContext) {
	return defineRegistryFields<UserApiKey, UserApiKeyOrderField, UserApiKeyListParam>(
		'UserApiKey',
		ctx.lookup,
		{
			name: {
				label: common_name(),
				type: 'string',
				searchable: true,
				// One header per key.
				groupable: false
			},
			permissions: {
				type: 'string',
				label: common_permissions(),
				filterable: true,
				serverFiltered: true,
				// Every role the backend defines.
				filterOptions: permissions.getItems().map((role) => role.id),
				filterLabel: (role) => permissions.getName(role) || role,
				getFilterValue: (item) => item.permissions ?? null,
				// The server groups on the stored role.
				getGroupValue: (item) => item.permissions ?? null,
				getValue: (item) =>
					item.permissions ? permissions.getName(item.permissions) || item.permissions : null,
				display: {
					getItems: (item) => {
						const role = item.permissions;
						if (!role) return [];
						return [
							{
								id: role,
								label: permissions.getName(role) || role,
								color: permissions.getColorHelper(role).color
							}
						];
					}
				}
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
				key: 'site_ids',
				type: 'array',
				// Keys share sites, so this filters; as an array it neither sorts nor groups.
				filterable: true,
				serverFiltered: true,
				filterOptions: ctx.sites().map((site) => site.id),
				// An All sites key reaches every site, and the server matches it on any of them.
				getFilterValue: (item) => item.site_ids ?? ctx.sites().map((site) => site.id),
				filterLabel: (id) => ctx.sites().find((n) => n.id === id)?.name ?? id,
				getValue(item) {
					if (item.site_ids === null) return [common_allSites()];
					return item.site_ids
						.map((id) => ctx.sites().find((n) => n.id === id)?.name)
						.filter((name): name is string => !!name);
				},
				display: { getItems: (item) => keySiteItems(ctx, item) }
			},
			{
				key: 'tags',
				label: common_tags(),
				type: 'array',
				filterable: true,
				getValue: (entity) => {
					return (entity.tags ?? [])
						.map((id) => ctx.tags().find((t) => t.id === id)?.name)
						.filter((name): name is string => !!name);
				}
			}
		],
		// Every key on this list is the viewer's: the list handler filters to the caller's own keys.
		{ omit: ['user_id'] }
	);
}
