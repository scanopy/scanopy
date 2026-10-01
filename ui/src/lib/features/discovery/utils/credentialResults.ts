import type { components } from '$lib/api/schema';
import { credentialIdsOf, warningCountsByCredential, type DiscoveryWarning } from './warnings';

export type CredentialRunResult = components['schemas']['CredentialRunResult'];
export type CredentialRunOutcome = components['schemas']['CredentialRunOutcome'];

/** One stored credential's line on a run's Credentials tab. */
export interface CredentialResultRow {
	credentialId: string;
	/** What it did, or `null` for a credential the run only warned about. */
	outcome: CredentialRunOutcome | null;
	/** How many of the run's warnings name it. */
	warningCount: number;
}

/**
 * Every credential a run reports on: one row per result, in the order the daemon sent them, then
 * one per credential that appears only in the warnings (skipped before it ran, so it has no
 * result), in first-warned order.
 */
export function credentialResultRows(
	results: CredentialRunResult[],
	warnings: DiscoveryWarning[]
): CredentialResultRow[] {
	const counts = warningCountsByCredential(warnings);
	const reported = new Set(results.map((r) => r.credential_id));
	return [
		...results.map((r) => ({
			credentialId: r.credential_id,
			outcome: r.outcome,
			warningCount: counts.get(r.credential_id) ?? 0
		})),
		...credentialIdsOf({ warnings })
			.filter((id) => !reported.has(id))
			.map((id) => ({ credentialId: id, outcome: null, warningCount: counts.get(id) ?? 0 }))
	];
}
