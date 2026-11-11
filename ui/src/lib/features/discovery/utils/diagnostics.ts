import { VERSION } from '#lib/version.ts';
import type { DiscoveryUpdatePayload } from '../types/api';

/**
 * A run's identifiers, versions, outcome and timings, for the `run` object of the copied run
 * data. Field names are the payload's own, so the block reads the same in every locale and maps
 * straight onto the server's logs and metrics. A field an older run never recorded is `null`
 * rather than absent, so every run's block has the same shape.
 */
export function runDiagnostics(
	results: DiscoveryUpdatePayload
): Record<string, string | number | null> {
	return {
		session_id: results.session_id ?? null,
		discovery_id: results.discovery_id ?? null,
		daemon_id: results.daemon_id ?? null,
		site_id: results.site_id ?? null,
		daemon_version: results.daemon_version ?? null,
		ui_version: VERSION,
		discovery_type: results.discovery_type.type,
		phase: results.phase ?? null,
		reason: results.reason ?? null,
		error: results.error ?? null,
		progress: results.progress ?? null,
		hosts_discovered: results.hosts_discovered ?? null,
		started_at: results.started_at ?? null,
		finished_at: results.finished_at ?? null,
		last_update_at: results.last_update_at ?? null
	};
}
