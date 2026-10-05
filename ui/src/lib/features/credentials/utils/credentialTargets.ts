import type { components } from '$lib/api/schema';
import type { IntegrationTarget } from '../types/base';
import { isDaemonHostOnly } from '../types/base';

/**
 * Where a credential type may apply. The variants of the backend `IntegrationTarget`
 * ARE the scopes, so its discriminant is the capability enum `CredentialType::targets()`
 * returns — derived here rather than restated, so a new backend target can't drift.
 */
export type CredentialTarget = components['schemas']['IntegrationTarget']['scope'];

type HostAssignment = components['schemas']['CredentialHostAssignment'];

/** The address a daemon-host target is reached on — the daemon's own loopback. */
export const DAEMON_HOST_IP = '127.0.0.1';

/**
 * Whether a credential type's `targets` metadata permits a given scope.
 *
 * The single predicate every target-gating surface goes through. `targets` is optional
 * on the metadata, and an unknown type id yields `{}` from the store — an absent list
 * permits nothing, so a type whose metadata failed to load can't be broadcast by
 * accident.
 */
export function supportsTarget(
	targets: CredentialTarget[] | undefined,
	scope: CredentialTarget
): boolean {
	return targets?.includes(scope) ?? false;
}

function isLoopback(ip: string): boolean {
	const t = ip?.trim() ?? '';
	return t === '127.0.0.1' || t === '::1' || t === 'localhost';
}

/**
 * Drop the assignments a credential type's `targets` don't permit.
 *
 * Needed because the assignment surfaces are chosen by `targets`, so switching the type
 * on a half-filled form hides a surface without clearing what was entered into it — and
 * the hidden value is still submitted. A `Site` assignment on a type that excludes
 * `Site` (e.g. a UniFi controller) is dispatched to the daemon as the default
 * credential for every IP on the subnet, so it must never survive the switch.
 *
 * A type that targets `DaemonHost` but not `Hosts` (the local socket) is reachable only on
 * a host that actually runs a daemon, so `daemonHostIds` narrows its assignments to those.
 * Pass `undefined` when the daemon list hasn't loaded — an empty list would otherwise read
 * as "no daemon hosts exist" and strip every assignment the type legitimately holds.
 *
 * Returns the pruned lists plus whether anything was actually dropped, so the caller can
 * tell the user rather than silently discarding their input.
 */
export function pruneAssignmentsForTargets(
	targets: CredentialTarget[] | undefined,
	assignments: { assignedSiteIds: string[]; hostAssignments: HostAssignment[] },
	daemonHostIds?: string[]
): { assignedSiteIds: string[]; hostAssignments: HostAssignment[]; changed: boolean } {
	const assignedSiteIds = supportsTarget(targets, 'Site') ? assignments.assignedSiteIds : [];

	let hostAssignments: HostAssignment[];
	if (supportsTarget(targets, 'Hosts')) {
		hostAssignments = assignments.hostAssignments;
	} else if (supportsTarget(targets, 'DaemonHost')) {
		hostAssignments = daemonHostIds
			? assignments.hostAssignments.filter((a) => daemonHostIds.includes(a.host_id))
			: assignments.hostAssignments;
	} else {
		hostAssignments = [];
	}

	return {
		assignedSiteIds,
		hostAssignments,
		changed:
			assignedSiteIds.length !== assignments.assignedSiteIds.length ||
			hostAssignments.length !== assignments.hostAssignments.length
	};
}

/**
 * Whether the user has actually pointed this credential at something on this discovery.
 *
 * "No IPs" is ambiguous on its own: for a broadcast-capable type it could mean site-wide,
 * or it could mean nothing was chosen. Only an explicit broadcast selection or a non-blank IP
 * counts — otherwise a credential merely *listed* because it is assigned elsewhere would be
 * written out as a site-wide target nobody asked for.
 */
export function hasExplicitTarget(
	scope: 'broadcast' | 'per_host' | undefined,
	ips: string[]
): boolean {
	return scope === 'broadcast' || ips.some((ip) => (ip?.trim() ?? '') !== '');
}

/**
 * Build the per-daemon `IntegrationTarget` for a credential with the given target IPs,
 * or `null` when its type permits no scope for that selection.
 *
 * The wire scope cannot be derived from the IP list alone: "no IPs" means site-wide
 * for a broadcast-capable type (SNMP) but means "nothing chosen yet" for one that
 * excludes `Site` (UniFi, a Docker proxy). Emitting `Site` there produces a target
 * the server drops at dispatch, so the credential silently never runs. Returning `null`
 * keeps it off the wire; callers validate the selection up front so a real user choice
 * doesn't reach this branch.
 */
export function integrationTargetFor(
	credentialId: string,
	targets: CredentialTarget[] | undefined,
	ips: string[]
): IntegrationTarget | null {
	// Tolerates holes: the shared form can leave an index unset (a removed row), which
	// arrives here as undefined. Throwing would abort the whole save with no message.
	const cleaned = ips.map((ip) => ip?.trim() ?? '').filter(Boolean);

	// A daemon-host-only type (the local socket) is always reached over the loopback,
	// as is an explicit loopback-only selection on any type.
	if (isDaemonHostOnly(targets) || (cleaned.length > 0 && cleaned.every(isLoopback))) {
		return supportsTarget(targets, 'DaemonHost')
			? { credential_id: credentialId, scope: 'DaemonHost' }
			: null;
	}

	if (cleaned.length > 0) {
		return supportsTarget(targets, 'Hosts')
			? { credential_id: credentialId, ips: cleaned, scope: 'Hosts' }
			: null;
	}

	return supportsTarget(targets, 'Site') ? { credential_id: credentialId, scope: 'Site' } : null;
}
