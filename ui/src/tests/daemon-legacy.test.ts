import { describe, expect, it } from 'vitest';
import { isPreUnifiedDaemon } from '$lib/features/daemons/utils';
import type { Daemon } from '$lib/features/daemons/types/base';

function daemon(last_seen: string | null, supports_unified_discovery: boolean): Daemon {
	return {
		last_seen,
		version_status: { supports_unified_discovery }
	} as unknown as Daemon;
}

describe('isPreUnifiedDaemon', () => {
	it('is true for a daemon that checked in on a version below the floor', () => {
		expect(isPreUnifiedDaemon(daemon('2026-09-01T00:00:00Z', false))).toBe(true);
	});

	it('is false for a daemon that checked in on a current version', () => {
		expect(isPreUnifiedDaemon(daemon('2026-09-01T00:00:00Z', true))).toBe(false);
	});

	// A daemon awaiting its first connection has no version, which the server reports the same way
	// as an old one. Reading that as legacy put a consolidation banner in front of every new
	// daemon.
	it('is false for a daemon awaiting its first connection', () => {
		expect(isPreUnifiedDaemon(daemon(null, false))).toBe(false);
	});
});
