import { describe, expect, it } from 'vitest';
import { runDiagnostics } from '#lib/features/discovery/utils/diagnostics.ts';
import type { DiscoveryUpdatePayload } from '#lib/features/discovery/types/api.ts';

const stalled = {
	session_id: '11111111-1111-4111-8111-111111111111',
	discovery_id: '22222222-2222-4222-8222-222222222222',
	daemon_id: '33333333-3333-4333-8333-333333333333',
	site_id: '44444444-4444-4444-8444-444444444444',
	daemon_version: '0.17.14',
	discovery_type: { type: 'SelfReport', host_id: '55555555-5555-4555-8555-555555555555' },
	phase: 'Failed',
	reason: 'StalledNoUpdates',
	error: 'No updates for 5 minutes',
	progress: 42,
	started_at: '2026-09-20T10:00:00Z',
	last_update_at: '2026-09-20T10:12:00Z'
} as DiscoveryUpdatePayload;

describe('runDiagnostics', () => {
	it('carries every id, the reason and the versions', () => {
		const run = runDiagnostics(stalled);
		expect(run.session_id).toBe(stalled.session_id);
		expect(run.discovery_id).toBe(stalled.discovery_id);
		expect(run.daemon_id).toBe(stalled.daemon_id);
		expect(run.site_id).toBe(stalled.site_id);
		expect(run.reason).toBe(stalled.reason);
		expect(run.daemon_version).toBe(stalled.daemon_version);
		expect(run.last_update_at).toBe(stalled.last_update_at);
	});

	it('keeps a field an older run never recorded, so the block always has the same shape', () => {
		const legacy = { ...stalled, reason: undefined, daemon_version: undefined };
		const run = runDiagnostics(legacy);
		expect(run.reason).toBeNull();
		expect(run.daemon_version).toBeNull();
		expect(Object.keys(run)).toEqual(Object.keys(runDiagnostics(stalled)));
	});
});
