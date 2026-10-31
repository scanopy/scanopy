import type { LabelledCardFieldItem } from '$lib/shared/components/data/types';
import { defineRegistryFields } from '$lib/shared/entity-fields/table';
import type { EntityLookup } from '$lib/shared/entity-fields';
import { NO_VALUE, valuesOrNoneCodec } from '$lib/shared/components/data/controls/listQuery';
import { siteItems } from '$lib/features/sites/columns';
import { permissions, entities } from '$lib/shared/stores/metadata';
import type { components, operations } from '$lib/api/schema';
import type { Site } from '$lib/features/sites/types';
import type { OrganizationInvite } from '$lib/features/organizations/types';
import { isInvite, isUser, type UserOrInvite } from '../types';
import {
	common_all,
	common_email,
	common_emailAndPassword,
	common_none,
	common_role,
	common_unknownEntity,
	common_user,
	common_you,
	invites_pendingInvite,
	users_authMethod,
	users_authMethodOidcAndPassword,
	users_emailVerified
} from '$lib/paraglide/messages';

type MemberOrderField = components['schemas']['MemberOrderField'];
type MemberStatus = components['schemas']['MemberStatus'];
/** The members list endpoint's query params, which the filters are sent and linked as. */
type MemberListParam = keyof NonNullable<operations['list_members']['parameters']['query']>;

/** What the user columns read from the tab, each as a getter so values stay live. */
export interface UserColumnsContext {
	/** Related entities the registry's user fields read; none beyond the user itself. */
	lookup: () => EntityLookup;
	/** Every user and invite the caller may see: the filter options come from these. */
	members: () => UserOrInvite[];
	sites: () => Site[];
	/** The invite's accept link. */
	inviteUrl: (invite: OrganizationInvite) => string;
}

/** Every status, for the filter's options. Keyed by the generated type, so a status the server
 * adds fails to compile until it is listed here. */
const ALL_STATUSES: Record<MemberStatus, true> = { pending: true, you: true, user: true };
const STATUSES = Object.keys(ALL_STATUSES) as MemberStatus[];

function statusLabel(status: string): string {
	return status === 'pending'
		? invites_pendingInvite()
		: status === 'you'
			? common_you()
			: common_user();
}

/** The password half of the server's auth method; the provider's name is the other half. */
const PASSWORD_METHOD = 'password';
const NO_METHOD = 'none';

function authMethodLabel(value: string): string {
	if (value === PASSWORD_METHOD) return common_emailAndPassword();
	if (value === NO_METHOD) return common_none();
	const suffix = `+${PASSWORD_METHOD}`;
	return value.endsWith(suffix)
		? users_authMethodOidcAndPassword({ provider: value.slice(0, -suffix.length) })
		: value;
}

function authMethodOf(item: UserOrInvite): string | null {
	return isUser(item) ? item.auth_method : null;
}

/**
 * The sites a user can reach.
 *
 * Admins and owners reach every site, which is a different statement from
 * being assigned all of them — so it reads as one "All" chip rather than a
 * list that would go stale the moment a site is added.
 */
function userSiteItems(ctx: UserColumnsContext, item: UserOrInvite): LabelledCardFieldItem[] {
	if (!isUser(item)) return [];
	const user = item.data;

	if (user.permissions === 'Admin' || user.permissions === 'Owner') {
		return [{ id: NO_VALUE, label: common_all(), color: entities.getColorHelper('Site').color }];
	}

	return siteItems(user.site_ids ?? [], ctx.sites());
}

/** The email of a member, for labelling who created an invite. */
function emailOf(ctx: UserColumnsContext, id: string): string {
	const member = ctx.members().find((m): m is Extract<UserOrInvite, { type: 'user' }> => {
		return isUser(m) && m.data.id === id;
	});
	return member?.data.email ?? common_unknownEntity({ entity: common_user() });
}

function distinct(values: (string | null)[]): string[] {
	return [...new Set(values.filter((v): v is string => v !== null))];
}

/**
 * The list holds two shapes, so every field resolves for both. A field that only one variant has
 * returns empty for the other. Every MemberOrderField value is listed; the server sorts, groups
 * and filters each.
 */
export function userColumns(ctx: UserColumnsContext) {
	return defineRegistryFields<UserOrInvite, MemberOrderField, MemberListParam>(
		'User',
		ctx.lookup,
		{
			email: {
				label: common_email(),
				type: 'string',
				searchable: true,
				// One header per member.
				groupable: false,
				getValue(item) {
					return isUser(item) ? item.data.email : item.data.send_to || invites_pendingInvite();
				},
				display: { primary: true, width: 220 }
			},
			invite_status: {
				type: 'string',
				filterable: true,
				serverFiltered: true,
				queryParam: 'statuses',
				filterOptions: STATUSES,
				getValue: (item) => statusLabel(item.status),
				getFilterValue: (item) => item.status,
				getGroupValue: (item) => item.status,
				filterLabel: statusLabel,
				display: {
					getItems: (item) =>
						item.status === 'user'
							? []
							: [{ id: item.status, label: statusLabel(item.status), color: 'Yellow' }]
				}
			},
			permissions: {
				label: common_role(),
				type: 'string',
				filterable: true,
				serverFiltered: true,
				// Every role the backend defines.
				filterOptions: permissions.getItems().map((role) => role.id),
				filterLabel: (role) => permissions.getName(role) || role,
				getFilterValue: (item) => item.data.permissions,
				getGroupValue: (item) => item.data.permissions,
				getValue(item) {
					return permissions.getName(item.data.permissions) || item.data.permissions;
				},
				display: {
					getItems: (item) => {
						const role = item.data.permissions;
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
			oidc_provider: {
				label: users_authMethod(),
				type: 'string',
				filterable: true,
				serverFiltered: true,
				queryParam: 'auth_methods',
				// The ways some member signs in.
				filterOptions: distinct(ctx.members().map(authMethodOf)),
				getValue: (item) => {
					const value = authMethodOf(item);
					return value === null ? null : authMethodLabel(value);
				},
				getFilterValue: authMethodOf,
				getGroupValue: authMethodOf,
				filterLabel: authMethodLabel
			},
			created_at: {
				type: 'date',
				groupable: false,
				getValue: (item) => item.data.created_at
			},
			invited_by: {
				type: 'string',
				filterable: true,
				serverFiltered: true,
				// The users who created some invite.
				filterOptions: distinct(ctx.members().map((m) => (isInvite(m) ? m.data.created_by : null))),
				getValue: (item) => (isUser(item) ? null : emailOf(ctx, item.data.created_by)),
				getFilterValue: (item) => (isUser(item) ? null : item.data.created_by),
				getGroupValue: (item) => (isUser(item) ? null : item.data.created_by),
				filterLabel: (id) => emailOf(ctx, id),
				display: { hiddenByDefault: true }
			},
			expires_at: {
				type: 'date',
				groupable: false,
				getValue: (item) => (isUser(item) ? null : item.data.expires_at),
				display: { hiddenByDefault: true }
			},
			updated_at: {
				type: 'date',
				groupable: false,
				getValue: (item) => item.data.updated_at,
				display: { hiddenByDefault: true }
			},
			// Account facts an invite does not have yet, so its cells stay empty.
			email_verified: {
				label: users_emailVerified(),
				type: 'boolean',
				filterable: true,
				serverFiltered: true,
				getValue: (item) => (isUser(item) ? (item.data.email_verified ?? null) : null),
				getGroupValue: (item) => (isUser(item) ? String(item.data.email_verified) : null),
				display: { hiddenByDefault: true }
			},
			oidc_linked_at: {
				type: 'date',
				groupable: false,
				getValue: (item) => (isUser(item) ? (item.data.oidc_linked_at ?? null) : null),
				display: { hiddenByDefault: true }
			}
		},
		[
			{
				key: 'site_ids',
				type: 'array',
				// Users share sites, so this filters; as an array it neither sorts nor groups.
				filterable: true,
				serverFiltered: true,
				filterOptions: [NO_VALUE, ...ctx.sites().map((site) => site.id)],
				filterCodec: valuesOrNoneCodec<MemberListParam>('site_ids', 'all_sites'),
				getValue: (item) => userSiteItems(ctx, item).map((n) => n.label),
				getFilterValue: (item) => userSiteItems(ctx, item).map((n) => n.id),
				filterLabel: (id) =>
					id === NO_VALUE ? common_all() : (ctx.sites().find((n) => n.id === id)?.name ?? id),
				display: { getItems: (item) => userSiteItems(ctx, item) }
			},
			{
				key: 'invite_url',
				type: 'string',
				getValue: (item) => (isUser(item) ? '' : ctx.inviteUrl(item.data)),
				display: { hiddenByDefault: true }
			}
		],
		// Each row wraps a user or an invite in `data`. An invite has none of the user-only fields,
		// so its cells for them stay empty.
		{ root: (item) => item.data }
	);
}
