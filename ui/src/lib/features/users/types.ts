import type { components } from '$lib/api/schema';

// Re-export generated types
export type User = components['schemas']['User'];
export type UserOrgPermissions = components['schemas']['UserOrgPermissions'];

/** A row of the members list: a user, or an invite still waiting to be accepted. */
export type UserOrInvite = components['schemas']['Member'];
type MemberUser = Extract<UserOrInvite, { type: 'user' }>;
type MemberInvite = Extract<UserOrInvite, { type: 'invite' }>;

export function isUser(item: UserOrInvite): item is MemberUser {
	return item.type === 'user';
}

export function isInvite(item: UserOrInvite): item is MemberInvite {
	return item.type === 'invite';
}
