import type { components } from '$lib/api/schema';
import { isCredentialWarning, type DiscoveryWarning } from './warnings';

export type CredentialRunResult = components['schemas']['CredentialRunResult'];
export type CredentialRunOutcome = components['schemas']['CredentialRunOutcome'];
type DiscoveryIntegration = components['schemas']['CredentialQueryPayloadDiscriminants'];

/** One line on a run's Credentials tab. */
export interface CredentialResultRow {
	/** Unique within the run: the credential id, or the integration for unattributed warnings. */
	key: string;
	/** The stored credential, or `null` for warnings about a credential with no stored row (a
	 *  network default such as SNMP's built-in "public"), grouped by `integration`. */
	credentialId: string | null;
	integration: DiscoveryIntegration | null;
	/** What it did, or `null` for a credential the run only warned about. */
	outcome: CredentialRunOutcome | null;
	/** The run's credential warnings about it. */
	warnings: DiscoveryWarning[];
	/** Every address it reported on, from its results and its warnings, in first-seen order. */
	addresses: string[];
}

function outcomeAddresses(outcome: CredentialRunOutcome | null): string[] {
	switch (outcome?.type) {
		case 'SshScript':
			return outcome.runs.map((r) => r.ip);
		case 'WakeOnLan':
			return outcome.hosts.map((h) => h.ip);
		default:
			return [];
	}
}

function warningAddresses(warnings: DiscoveryWarning[]): string[] {
	return warnings.flatMap((w) => ('address' in w && w.address ? [w.address] : []));
}

/**
 * Every credential a run reports on: one row per result, in the order the daemon sent them; then
 * one per stored credential that appears only in the warnings (skipped before it ran); then one per
 * integration whose warnings name no stored credential. Only credential warnings, as the backend
 * files their codes, are read.
 */
export function credentialResultRows(
	results: CredentialRunResult[],
	allWarnings: DiscoveryWarning[]
): CredentialResultRow[] {
	const warnings = allWarnings.filter(isCredentialWarning);
	const byCredential = new Map<string, DiscoveryWarning[]>();
	const byIntegration = new Map<DiscoveryIntegration, DiscoveryWarning[]>();
	for (const w of warnings) {
		const id = 'credential_id' in w ? w.credential_id : null;
		if (id) {
			byCredential.set(id, [...(byCredential.get(id) ?? []), w]);
		} else if ('integration' in w) {
			byIntegration.set(w.integration, [...(byIntegration.get(w.integration) ?? []), w]);
		}
	}

	const row = (
		credentialId: string | null,
		integration: DiscoveryIntegration | null,
		outcome: CredentialRunOutcome | null,
		rowWarnings: DiscoveryWarning[]
	): CredentialResultRow => ({
		key: credentialId ?? `integration:${integration}`,
		credentialId,
		integration,
		outcome,
		warnings: rowWarnings,
		addresses: [...new Set([...outcomeAddresses(outcome), ...warningAddresses(rowWarnings)])]
	});

	const reported = new Set(results.map((r) => r.credential_id));
	return [
		...results.map((r) =>
			row(r.credential_id, null, r.outcome, byCredential.get(r.credential_id) ?? [])
		),
		...[...byCredential]
			.filter(([id]) => !reported.has(id))
			.map(([id, ws]) => row(id, null, null, ws)),
		...[...byIntegration].map(([integration, ws]) => row(null, integration, null, ws))
	];
}
