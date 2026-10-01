import { describe, it, expect } from 'vitest';
import {
	credentialResultRows,
	type CredentialRunResult
} from '$lib/features/discovery/utils/credentialResults';
import {
	warningCountsByCredential,
	type DiscoveryWarning
} from '$lib/features/discovery/utils/warnings';

const SNMP = '00000000-0000-0000-0000-0000000000c1';
const SSH = '00000000-0000-0000-0000-0000000000c2';
const DOCKER = '00000000-0000-0000-0000-0000000000c3';

const rejected = (credential_id: string | null, address: string): DiscoveryWarning =>
	({
		code: 'CredentialRejected',
		integration: 'Snmp',
		address,
		detail: 'auth failed',
		credential_id
	}) as DiscoveryWarning;

const mismatch = (credential_id: string): DiscoveryWarning =>
	({
		code: 'CredentialDaemonOsMismatch',
		integration: 'DockerSocket',
		credential_id,
		declared: 'Windows',
		actual: 'Unix'
	}) as DiscoveryWarning;

const subnetTimeout = { code: 'IcmpSweepTimedOut', seconds: 30 } as DiscoveryWarning;

describe('warningCountsByCredential', () => {
	it('counts every occurrence naming a credential and ignores warnings without one', () => {
		const counts = warningCountsByCredential([
			rejected(SNMP, '10.0.0.1'),
			rejected(SNMP, '10.0.0.2'),
			rejected(null, '10.0.0.3'),
			mismatch(DOCKER),
			subnetTimeout
		]);

		expect(counts.get(SNMP)).toBe(2);
		expect(counts.get(DOCKER)).toBe(1);
		expect(counts.size).toBe(2);
	});
});

describe('credentialResultRows', () => {
	const results: CredentialRunResult[] = [
		{ credential_id: SSH, outcome: { type: 'SshScript', runs: [] } },
		{ credential_id: SNMP, outcome: { type: 'Collected', hosts: 4 } }
	];

	it('keeps result order and attaches each credential its warning count', () => {
		const rows = credentialResultRows(results, [rejected(SNMP, '10.0.0.1')]);

		expect(rows.map((r) => [r.credentialId, r.warningCount])).toEqual([
			[SSH, 0],
			[SNMP, 1]
		]);
	});

	it('adds a row for a credential that only appears in the warnings', () => {
		const rows = credentialResultRows(results, [mismatch(DOCKER), mismatch(DOCKER)]);

		expect(rows).toHaveLength(3);
		expect(rows[2]).toEqual({ credentialId: DOCKER, outcome: null, warningCount: 2 });
	});

	it('is empty when nothing ran and no warning names a credential', () => {
		expect(credentialResultRows([], [subnetTimeout, rejected(null, '10.0.0.1')])).toEqual([]);
	});
});
