import { describe, it, expect } from 'vitest';
import {
	groupByTier,
	isDismissable,
	setupIssueCount,
	type SetupItemType,
	type SetupListEntry
} from '$lib/features/home/setup-list';

const daemonIssue: SetupListEntry = {
	key: {
		type: 'Entity',
		item_type: 'DaemonHealth',
		entity_id: '00000000-0000-4000-8000-000000000001'
	},
	tier: 'Issue',
	item: {
		type: 'DaemonHealth',
		daemon_id: '00000000-0000-4000-8000-000000000001',
		name: 'edge-01',
		site_id: '00000000-0000-4000-8000-000000000002',
		health: 'outdated'
	}
};

function orgEntry(
	type: Exclude<SetupItemType, 'DaemonHealth' | 'RunFirstScan'>,
	tier: SetupListEntry['tier']
) {
	return { key: { type: 'Org', item_type: type }, tier, item: { type } } as SetupListEntry;
}

/** One Issue, one Next step and three Suggestions, in the order the server lists them. */
const entries: SetupListEntry[] = [
	daemonIssue,
	orgEntry('OpenTopology', 'NextStep'),
	orgEntry('Tags', 'Suggestion'),
	orgEntry('InviteTeam', 'Suggestion'),
	orgEntry('Share', 'Suggestion')
];

describe('Setup list', () => {
	it('groups what it is given by tier, in tier order, without capping', () => {
		const groups = groupByTier([...entries].reverse());
		expect(groups.map((g) => [g.tier, g.entries.length])).toEqual([
			['Issue', 1],
			['NextStep', 1],
			['Suggestion', 3]
		]);
	});

	it('leaves out a tier with no entries', () => {
		expect(groupByTier([orgEntry('Tags', 'Suggestion')]).map((g) => g.tier)).toEqual([
			'Suggestion'
		]);
	});

	it('offers Dismiss on Suggestions only', () => {
		const dismissable = groupByTier(entries).map((g) => [g.tier, g.entries.map(isDismissable)]);
		expect(dismissable).toEqual([
			['Issue', [false]],
			['NextStep', [false]],
			['Suggestion', [true, true, true]]
		]);
	});

	it('badges Home with the rows under Issues', () => {
		const issues = groupByTier(entries).find((g) => g.tier === 'Issue')?.entries ?? [];
		expect(setupIssueCount(entries)).toBe(issues.length);
		const other: SetupListEntry = {
			...daemonIssue,
			key: { ...daemonIssue.key, entity_id: '00000000-0000-4000-8000-000000000003' }
		} as SetupListEntry;
		expect(setupIssueCount([...entries, other])).toBe(2);
		expect(setupIssueCount(entries.slice(1))).toBe(0);
	});
});
