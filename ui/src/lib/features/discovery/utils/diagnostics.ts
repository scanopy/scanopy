import { VERSION } from '$lib/version';
import type { DiscoveryUpdatePayload } from '../types/api';

/**
 * A run's identifiers, versions and timings as `field: value` lines, for pasting into a support
 * thread. Field names are the payload's own, so the block reads the same in every locale and maps
 * straight onto the server's logs and metrics.
 */
export function formatDiagnostics(results: DiscoveryUpdatePayload): string {
	const warningCodes = [...new Set((results.warnings ?? []).map((w) => w.code))];
	const lines: [string, string | number | null | undefined][] = [
		['session_id', results.session_id],
		['discovery_id', results.discovery_id],
		['daemon_id', results.daemon_id],
		['network_id', results.network_id],
		['daemon_version', results.daemon_version],
		['ui_version', VERSION],
		['discovery_type', results.discovery_type.type],
		['phase', results.phase],
		['reason', results.reason],
		['error', results.error],
		['progress', results.progress],
		['hosts_discovered', results.hosts_discovered],
		['started_at', results.started_at],
		['finished_at', results.finished_at],
		['last_update_at', results.last_update_at],
		['warning_codes', warningCodes.length > 0 ? warningCodes.join(', ') : null]
	];
	return lines.map(([field, value]) => `${field}: ${value ?? 'unknown'}`).join('\n');
}
