import { afterEach, describe, expect, it, vi } from 'vitest';
import { QueryClient } from '@tanstack/svelte-query';
import type { components } from '$lib/api/schema';

vi.mock('$env/dynamic/public', () => ({
	env: { PUBLIC_SERVER_HOSTNAME: 'scanopy.test', PUBLIC_SERVER_PROTOCOL: 'http' }
}));
vi.mock('$lib/shared/stores/feedback', () => ({ pushError: vi.fn(), pushSuccess: vi.fn() }));
vi.mock('$lib/api/server-version', () => ({ checkServerVersion: vi.fn() }));

const { fetchCurrentUser, mergeCurrentUser } = await import('$lib/features/auth/queries');
const { isAllowed } = await import('$lib/features/auth/access');
const { queryKeys } = await import('$lib/api/query-client');

type AccessGrants = components['schemas']['AccessGrants'];
type Resource = components['schemas']['Resource'];
type Action = components['schemas']['Action'];
type Organization = components['schemas']['Organization'];

const SITE_A = '00000000-0000-0000-0000-00000000000a';
const SITE_B = '00000000-0000-0000-0000-00000000000b';
const NEW_SITE = '00000000-0000-0000-0000-00000000000c';

const rwd = (resource: Resource) =>
	(['Read', 'Write', 'Delete'] as Action[]).map((action) => ({ resource, action }));
const read = (resource: Resource) => [{ resource, action: 'Read' as Action }];

/** What `/api/auth/me` sends, so `can` runs on what the app caches. */
async function signedInWith(access: AccessGrants) {
	vi.stubGlobal(
		'fetch',
		vi.fn(
			async () =>
				new Response(
					JSON.stringify({
						success: true,
						data: { id: 'u1', email: 'u@example.com', permissions: 'Member', access },
						error: null,
						meta: { api_version: 1, server_version: '0.19.5' }
					}),
					{ status: 200, headers: { 'Content-Type': 'application/json' } }
				)
		)
	);
	const user = await fetchCurrentUser();
	return user?.access;
}

function org(plan: string, plan_status: string | null = 'active') {
	return { plan: { type: plan }, plan_status } as unknown as Organization;
}

const member: AccessGrants = {
	organization: [...read('Credential'), ...read('Organization'), ...rwd('UserApiKey')],
	sites: [
		{ site_id: SITE_A, grants: rwd('Host') },
		{ site_id: SITE_B, grants: read('Host') }
	],
	all_sites: []
};
const admin: AccessGrants = {
	organization: [...rwd('Credential'), ...rwd('Tag'), ...rwd('Member')],
	sites: [{ site_id: SITE_A, grants: rwd('Host') }],
	all_sites: rwd('Host')
};
const owner: AccessGrants = {
	organization: [...rwd('Credential'), ...rwd('Tag'), ...rwd('Organization'), ...rwd('Billing')],
	sites: [{ site_id: SITE_A, grants: rwd('Host') }],
	all_sites: rwd('Host')
};
const onNoSite: AccessGrants = {
	organization: [...read('Credential'), ...rwd('UserApiKey')],
	sites: [],
	all_sites: []
};

afterEach(() => {
	vi.unstubAllGlobals();
});

describe('can, on what /api/auth/me returns', () => {
	it('answers each site from that site', async () => {
		const access = await signedInWith(member);
		const can = (action: Action, siteId?: string) =>
			isAllowed({ access, organization: org('Free') }, 'Host', action, siteId);
		expect(can('Write', SITE_A)).toBe(true);
		expect(can('Write', SITE_B)).toBe(false);
		expect(can('Read', SITE_B)).toBe(true);
		expect(can('Write', NEW_SITE)).toBe(false);
	});

	it('answers any site when none is named', async () => {
		const access = await signedInWith(member);
		expect(isAllowed({ access, organization: org('Free') }, 'Host', 'Write')).toBe(true);
		expect(isAllowed({ access, organization: org('Free') }, 'Daemon', 'Read')).toBe(false);
	});

	it('answers org-keyed resources from the organization grants, whatever the site', async () => {
		const access = await signedInWith(member);
		const context = { access, organization: org('Free') };
		expect(isAllowed(context, 'Credential', 'Read', SITE_A)).toBe(true);
		expect(isAllowed(context, 'Credential', 'Write', SITE_A)).toBe(false);
		expect(isAllowed(context, 'Member', 'Read')).toBe(false);
	});

	it('reaches a site created after the grants loaded through an all-sites assignment', async () => {
		const access = await signedInWith(admin);
		expect(isAllowed({ access, organization: org('Free') }, 'Host', 'Delete', NEW_SITE)).toBe(true);
	});

	it('lets a principal on no site read empty lists, and nothing more', async () => {
		const access = await signedInWith(onNoSite);
		const context = { access, organization: org('Free') };
		expect(isAllowed(context, 'Host', 'Read')).toBe(true);
		expect(isAllowed(context, 'DaemonApiKey', 'Read')).toBe(true);
		expect(isAllowed(context, 'Host', 'Write')).toBe(false);
		expect(isAllowed(context, 'Host', 'Read', SITE_A)).toBe(false);
		expect(isAllowed(context, 'Member', 'Read')).toBe(false);
	});

	it('refuses everything before the grants load', () => {
		expect(isAllowed({ access: undefined, organization: undefined }, 'Host', 'Read')).toBe(false);
	});
});

describe('can on a lapsed org', () => {
	const lapsed = org('Pro', 'cancelled');

	it('stops writes and keeps reads', async () => {
		const access = await signedInWith(owner);
		expect(isAllowed({ access, organization: lapsed }, 'Host', 'Read', SITE_A)).toBe(true);
		expect(isAllowed({ access, organization: lapsed }, 'Host', 'Write', SITE_A)).toBe(false);
		expect(isAllowed({ access, organization: lapsed }, 'Tag', 'Delete')).toBe(false);
	});

	it('keeps the Settings pages writable', async () => {
		const access = await signedInWith(owner);
		expect(isAllowed({ access, organization: lapsed }, 'Organization', 'Write')).toBe(true);
		expect(isAllowed({ access, organization: lapsed }, 'Billing', 'Write')).toBe(true);
	});

	it('leaves an active paid org alone', async () => {
		const access = await signedInWith(owner);
		expect(isAllowed({ access, organization: org('Pro') }, 'Host', 'Write', SITE_A)).toBe(true);
	});
});

describe('can in a demo org', () => {
	const demo = org('Demo', null);

	it('stops a non-Owner writing credentials and tags, and leaves the rest of the tour', async () => {
		const access = await signedInWith(admin);
		expect(isAllowed({ access, organization: demo }, 'Credential', 'Write')).toBe(false);
		expect(isAllowed({ access, organization: demo }, 'Tag', 'Delete')).toBe(false);
		expect(isAllowed({ access, organization: demo }, 'Credential', 'Read')).toBe(true);
		expect(isAllowed({ access, organization: demo }, 'Host', 'Write', SITE_A)).toBe(true);
	});

	it('leaves an Owner writing them', async () => {
		const access = await signedInWith(owner);
		expect(isAllowed({ access, organization: demo }, 'Credential', 'Write')).toBe(true);
		expect(isAllowed({ access, organization: demo }, 'Tag', 'Write')).toBe(true);
	});
});

describe('mergeCurrentUser', () => {
	it('keeps the grants when an endpoint returns a plain user', async () => {
		const access = await signedInWith(member);
		const queryClient = new QueryClient();
		const key = queryKeys.auth.currentUser();
		queryClient.setQueryData(key, await fetchCurrentUser());

		const renamed = { id: 'u1', email: 'renamed@example.com' };
		mergeCurrentUser(queryClient, renamed as Parameters<typeof mergeCurrentUser>[1]);

		expect(queryClient.getQueryData(key)).toMatchObject({ email: renamed.email, access });
	});

	it('leaves a signed-out cache signed out', () => {
		const queryClient = new QueryClient();
		const key = queryKeys.auth.currentUser();
		queryClient.setQueryData(key, null);
		mergeCurrentUser(queryClient, { id: 'u1' } as Parameters<typeof mergeCurrentUser>[1]);
		expect(queryClient.getQueryData(key)).toBeNull();
	});
});
