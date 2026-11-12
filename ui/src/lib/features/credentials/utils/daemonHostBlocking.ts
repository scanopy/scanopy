import { credentialTypes } from '#lib/shared/stores/metadata.ts';
import { daemons_credentialWizardDaemonHostUnavailable } from '#lib/paraglide/messages.js';
import { getCredentialTypeId, type Credential } from '../types/base';

/**
 * Socket↔proxy mutual exclusion on a daemon host.
 *
 * A daemon host is a single endpoint per `single_endpoint_per_host` integration (e.g. its local
 * Docker/Podman API), so only ONE transport — socket OR proxy — of a given integration may be
 * assigned to it. This mirrors the backend `single_endpoint_targets_conflict` rule and the
 * discovery-modal blocking, sharing one metadata-driven predicate across all assignment surfaces.
 */

/** The `associated_service`s claimed by the single-endpoint credentials in `assignedCredentials`. */
export function claimedIntegrations(
	assignedCredentials: Credential[],
	excludeCredentialId?: string
): string[] {
	const claimed: string[] = [];
	for (const c of assignedCredentials) {
		if (c.id === excludeCredentialId) continue;
		const meta = credentialTypes.getMetadata(getCredentialTypeId(c));
		if (
			meta?.single_endpoint_per_host &&
			meta.associated_service &&
			!claimed.includes(meta.associated_service)
		) {
			claimed.push(meta.associated_service);
		}
	}
	return claimed;
}

/** Claimed integrations for the credentials assigned (via the `host_credentials` junction) to
 *  `hostId`, computed from the full credentials list. */
export function claimedIntegrationsForHost(
	hostId: string,
	allCredentials: Credential[],
	excludeCredentialId?: string
): string[] {
	const assigned = allCredentials.filter((c) =>
		(c.host_assignments ?? []).some((a) => a.host_id === hostId)
	);
	return claimedIntegrations(assigned, excludeCredentialId);
}

/** A localized reason a credential type cannot be assigned to a host whose single-endpoint
 *  integration is already claimed, or `null` when assignable. */
export function daemonHostBlockReason(
	candidateTypeId: string,
	claimedIntegrationsList: string[]
): string | null {
	const meta = credentialTypes.getMetadata(candidateTypeId);
	if (!meta?.single_endpoint_per_host || !meta.associated_service) return null;
	return claimedIntegrationsList.includes(meta.associated_service)
		? daemons_credentialWizardDaemonHostUnavailable({ integration: meta.associated_service })
		: null;
}
