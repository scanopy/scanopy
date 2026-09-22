import { describe, expect, it } from 'vitest';
import { formatDiagnostics } from '$lib/features/discovery/utils/diagnostics';
import type { DiscoveryUpdatePayload } from '$lib/features/discovery/types/api';

function parse(block: string): Map<string, string> {
	return new Map(
		block.split('\n').map((line) => {
			const at = line.indexOf(': ');
			return [line.slice(0, at), line.slice(at + 2)];
		})
	);
}

const stalled = {
	session_id: '11111111-1111-4111-8111-111111111111',
	discovery_id: '22222222-2222-4222-8222-222222222222',
	daemon_id: '33333333-3333-4333-8333-333333333333',
	network_id: '44444444-4444-4444-8444-444444444444',
	daemon_version: '0.17.14',
	discovery_type: { type: 'SelfReport', host_id: '55555555-5555-4555-8555-555555555555' },
	phase: 'Failed',
	reason: 'StalledNoUpdates',
	error: 'No updates for 5 minutes',
	progress: 42,
	started_at: '2026-09-20T10:00:00Z',
	last_update_at: '2026-09-20T10:12:00Z'
} as DiscoveryUpdatePayload;

describe('formatDiagnostics', () => {
	it('carries every id, the reason and the versions, one field per line', () => {
		const fields = parse(formatDiagnostics(stalled));
		expect(fields.get('session_id')).toBe(stalled.session_id);
		expect(fields.get('discovery_id')).toBe(stalled.discovery_id);
		expect(fields.get('daemon_id')).toBe(stalled.daemon_id);
		expect(fields.get('network_id')).toBe(stalled.network_id);
		expect(fields.get('reason')).toBe(stalled.reason);
		expect(fields.get('daemon_version')).toBe(stalled.daemon_version);
		expect(fields.get('last_update_at')).toBe(stalled.last_update_at);
	});

	it('keeps a field an older run never recorded, so the block always has the same shape', () => {
		const legacy = { ...stalled, reason: undefined, daemon_version: undefined };
		const fields = parse(formatDiagnostics(legacy));
		expect(fields.has('reason')).toBe(true);
		expect(fields.has('daemon_version')).toBe(true);
		expect([...fields.keys()]).toEqual([...parse(formatDiagnostics(stalled)).keys()]);
	});
});
