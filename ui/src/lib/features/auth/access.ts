/**
 * What the signed-in user can do. `can` is the one permission question the UI asks: every control
 * that depends on a role reads it, so a control shows what the server allows.
 */

import type { components } from '#lib/api/schema';
import { useOrganizationQuery } from '#lib/features/organizations/queries.ts';
import { isPlanLapsed } from '#lib/features/organizations/types.ts';
import { common_notAllowedHere, common_notAllowedOnSelection } from '#lib/paraglide/messages.js';
import { disabledWhen, type Unavailable } from '#lib/shared/components/data/types.ts';
import { billingPlans, resources } from '#lib/shared/stores/metadata.ts';
import { useCurrentUserQuery } from './queries';

export type Resource = components['schemas']['Resource'];
export type Action = components['schemas']['Action'];
export type AccessGrants = components['schemas']['AccessGrants'];
type Grant = components['schemas']['Grant'];

export type Can = (resource: Resource, action: Action, siteId?: string) => boolean;

/** What `can` reads: the user's grants, and the org whose plan can limit them. */
export interface AccessContext {
	access: AccessGrants | null | undefined;
	organization: components['schemas']['Organization'] | null | undefined;
}

/**
 * The server refuses every write a non-Owner makes in a demo org. The UI shows that only here, and
 * leaves the rest of the app enabled for the product tour, with the refusal as a toast.
 */
const DEMO_READ_ONLY: Resource[] = ['Credential', 'Tag'];

function holds(grants: Grant[], resource: Resource, action: Action): boolean {
	return grants.some((grant) => grant.resource === resource && grant.action === action);
}

/** Whether the grants hold this on All sites, so on sites created later too. */
export function holdsOnAllSites(
	access: AccessGrants | null | undefined,
	resource: Resource,
	action: Action
): boolean {
	return !!access && holds(access.all_sites, resource, action);
}

/**
 * Whether the grants allow this, as the server's `Can` requirement answers. A site-keyed resource
 * asks that site, or any site when none is named. `all_sites` answers for sites created since the
 * grants were read. A principal on no site reads empty lists rather than being refused.
 */
function isGranted(
	access: AccessGrants,
	resource: Resource,
	action: Action,
	siteId?: string
): boolean {
	if (!resources.getMetadata(resource).per_site) {
		return holds(access.organization, resource, action);
	}
	if (holds(access.all_sites, resource, action)) return true;
	if (siteId !== undefined) {
		const site = access.sites.find((held) => held.site_id === siteId);
		return site !== undefined && holds(site.grants, resource, action);
	}
	if (access.sites.length === 0) return action === 'Read';
	return access.sites.some((site) => holds(site.grants, resource, action));
}

/** `can`, on the data it reads. A lapsed or demo org limits writes on top of the role's grants. */
export function isAllowed(
	{ access, organization }: AccessContext,
	resource: Resource,
	action: Action,
	siteId?: string
): boolean {
	if (!access || !isGranted(access, resource, action, siteId)) return false;
	if (action === 'Read' || !organization) return true;
	if (isPlanLapsed(organization) && !resources.getMetadata(resource).writable_when_lapsed) {
		return false;
	}
	const isDemoOrg = billingPlans.getMetadata(organization.plan?.type ?? null).is_demo === true;
	return !(isDemoOrg && isDemoReadOnly(access, resource));
}

function isDemoReadOnly(access: AccessGrants, resource: Resource): boolean {
	return DEMO_READ_ONLY.includes(resource) && !holds(access.organization, 'Organization', 'Delete');
}

/**
 * `can(resource, action, siteId?)` for this component. It reads queries, so calling it inside
 * `$derived` or a template stays reactive.
 */
export function useCan(): Can {
	const currentUserQuery = useCurrentUserQuery();
	const organizationQuery = useOrganizationQuery();
	return (resource, action, siteId) =>
		isAllowed(
			{ access: currentUserQuery.data?.access, organization: organizationQuery.data },
			resource,
			action,
			siteId
		);
}

/**
 * `RowAction.unavailable` for a row the user can't act on, when they can on other rows. A user
 * who can't act on any row gets no actions column instead.
 */
export function refusedUnless(allowed: boolean): Unavailable | undefined {
	return disabledWhen(!allowed, common_notAllowedHere(), common_notAllowedOnSelection());
}
