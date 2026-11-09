import { describe, it, expect } from 'vitest';
import type { components } from '$lib/api/schema';
import {
	groupByTier,
	isDismissable,
	setupIssueCount,
	setupItemKey
} from '$lib/features/home/setup-list';

type SetupItem = components['schemas']['SetupItem'];

const daemonIssue: SetupItem = {
	type: 'DaemonHealth',
	daemon_id: '00000000-0000-4000-8000-000000000001',
	name: 'edge-01',
	site_id: '00000000-0000-4000-8000-000000000002',
	health: 'outdated'
};

/** One Issue, one Next step and three Suggestions, in the order the server lists them. */
const items: SetupItem[] = [
	daemonIssue,
	{ type: 'OpenTopology' },
	{ type: 'Tags' },
	{ type: 'InviteTeam' },
	{ type: 'Share' }
];

describe('Setup list', () => {
	it('groups what it is given by tier, in tier order, without capping', () => {
		const groups = groupByTier([...items].reverse());
		expect(groups.map((g) => [g.tier, g.items.length])).toEqual([
			['Issue', 1],
			['NextStep', 1],
			['Suggestion', 3]
		]);
	});

	it('leaves out a tier with no items', () => {
		expect(groupByTier([{ type: 'Tags' }]).map((g) => g.tier)).toEqual(['Suggestion']);
	});

	it('offers Dismiss on Suggestions only', () => {
		const dismissable = groupByTier(items).map((g) => [g.tier, g.items.map(isDismissable)]);
		expect(dismissable).toEqual([
			['Issue', [false]],
			['NextStep', [false]],
			['Suggestion', [true, true, true]]
		]);
	});

	it('badges Home with the rows under Issues', () => {
		const issues = groupByTier(items).find((g) => g.tier === 'Issue')?.items ?? [];
		expect(setupIssueCount(items)).toBe(issues.length);
		expect(setupIssueCount([...items, { ...daemonIssue, daemon_id: 'other' }])).toBe(2);
		expect(setupIssueCount(items.slice(1))).toBe(0);
	});

	it('keys two daemons of one issue type apart', () => {
		const other = { ...daemonIssue, daemon_id: '00000000-0000-4000-8000-000000000003' };
		expect(setupItemKey(daemonIssue)).not.toBe(setupItemKey(other));
	});
});
