import { describe, it, expect, vi, afterEach } from 'vitest';
import { entityFreshness, neighborEvidenceFreshness } from '$lib/shared/utils/freshness';
import type { Site } from '$lib/features/sites/types';
import type { components } from '$lib/api/schema';

type EntitySource = components['schemas']['EntitySource'];

const HOUR_MS = 60 * 60 * 1000;
const NOW = new Date('2026-07-22T12:00:00Z').getTime();

/**
 * `effective_stale_after_hours` is what the server publishes after applying its
 * own default — the frontend holds no default of its own, precisely so it
 * cannot drift from the digest.
 */
function site(effective_stale_after_hours: number): Site {
	return { id: 'n1', effective_stale_after_hours } as Site;
}

function entity(hoursAgo: number, source: EntitySource = { type: 'Discovery' }) {
	return { last_seen_at: new Date(NOW - hoursAgo * HOUR_MS).toISOString(), source };
}

afterEach(() => vi.useRealTimers());

function freezeClock() {
	vi.useFakeTimers();
	vi.setSystemTime(NOW);
}

describe('entityFreshness', () => {
	it('uses each site’s own window rather than one global cutoff', () => {
		freezeClock();
		const strict = site(1);
		const lenient = site(24 * 30);
		const seenTwoHoursAgo = entity(2);

		expect(entityFreshness(seenTwoHoursAgo, strict)).toBe('stale');
		expect(entityFreshness(seenTwoHoursAgo, lenient)).toBe('current');
	});

	// Without a site we cannot know the window, so make no claim rather than
	// guessing — a wrong guess would badge assets stale that the digest calls
	// current.
	it('makes no staleness claim when the site is not loaded', () => {
		freezeClock();
		expect(entityFreshness(entity(24 * 365), undefined)).toBe('current');
	});

	// Discovery never refreshes last_seen_at on entities it didn't create, so
	// judging them would mark every hand-curated asset stale once it aged out.
	// Mirrors the backend's `is_discovery_managed` guard.
	it('never marks entities discovery does not manage as stale', () => {
		freezeClock();
		const net = site(1);
		const longAgo = 24 * 365;

		expect(entityFreshness(entity(longAgo, { type: 'Manual' }), net)).toBe('current');
		expect(entityFreshness(entity(longAgo, { type: 'System' }), net)).toBe('current');
		expect(entityFreshness(entity(longAgo, { type: 'Discovery' }), net)).toBe('stale');
		expect(
			entityFreshness(
				entity(longAgo, {
					type: 'DiscoveryWithMatch'
				} as EntitySource),
				net
			)
		).toBe('stale');
	});

	// IPAddress, Port, Interface and Binding carry no `source` column because
	// they cannot be created any other way. The backend trait defaults
	// `is_discovery_managed` to true for exactly these; treating an absent
	// source as unmanaged here would make them permanently Current in the UI
	// while the digest judged them normally.
	it('treats an entity with no source column as discovery-managed', () => {
		freezeClock();
		const net = site(1);
		const ipWithNoSourceField = { last_seen_at: new Date(NOW - 100 * HOUR_MS).toISOString() };
		expect(entityFreshness(ipWithNoSourceField, net)).toBe('stale');
	});

	it('treats a never-observed entity as current rather than guessing', () => {
		freezeClock();
		expect(entityFreshness({ source: { type: 'Discovery' } }, site(1))).toBe('current');
	});

	// The frontend cutoff must agree with the backend's
	// `Site::stale_cutoff` (reference - stale_after_hours), or a host badged
	// stale in the inventory would not be the host reported stale in the digest.
	it('places the boundary exactly at the window edge, matching the backend rule', () => {
		freezeClock();
		const net = site(24);

		expect(entityFreshness(entity(23.9), net)).toBe('current');
		expect(entityFreshness(entity(24.1), net)).toBe('stale');
	});
});

// The UI judges every entity on its own last_seen_at, with no inheritance from
// a parent host. The digest inherits (`ChildPolicy`) because it reports one
// scan's events; a persistent badge answers "when was this last seen?", where
// inheriting produced a node marked Stale whose tooltip read "last seen 2 hours
// ago" and made topology disagree with the inventory.
describe('no parent inheritance in the UI', () => {
	it('leaves a recently-seen child current even when its host is stale', () => {
		freezeClock();
		const net = site(24);
		const freshChild = entity(1);

		expect(entityFreshness(freshChild, net)).toBe('current');
		// The host being stale is irrelevant to the child's own verdict.
		expect(entityFreshness(entity(100), net)).toBe('stale');
	});
});

// A link's freshness subject is the adjacency, not the port. The port keeps appearing in the
// ifTable long after its neighbour record stops arriving, so `last_seen_at` cannot tell a live
// link from one whose evidence has completely disappeared.
describe('neighborEvidenceFreshness', () => {
	it('calls a link stale while both its ports are still being scanned', () => {
		freezeClock();
		const net = site(24);
		// The reproduction: the port is observed every scan, but nothing has said anything is
		// attached to it in a week.
		const port = {
			...entity(0.2),
			neighbor_seen_at: new Date(NOW - 24 * 7 * HOUR_MS).toISOString()
		};

		expect(entityFreshness(port, net)).toBe('current');
		expect(neighborEvidenceFreshness(port, net)).toBe('stale');
	});

	it('reads a port that has never carried evidence as unknown rather than stale', () => {
		freezeClock();
		const net = site(24);

		// Every row predating the column arrives this way, and none of them may be flagged.
		expect(neighborEvidenceFreshness({ neighbor_seen_at: null }, net)).toBe('current');
		expect(neighborEvidenceFreshness({}, net)).toBe('current');
	});

	it('judges the adjacency on the same window as everything else on the site', () => {
		freezeClock();
		const port = { neighbor_seen_at: new Date(NOW - 2 * HOUR_MS).toISOString() };

		expect(neighborEvidenceFreshness(port, site(1))).toBe('stale');
		expect(neighborEvidenceFreshness(port, site(24 * 30))).toBe('current');
	});
});
