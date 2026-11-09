import { describe, it, expect } from 'vitest';
import {
	doneEntries,
	issueSiteIds,
	issuesOnSites,
	openEntries,
	setupIssueCount,
	setupProgress,
	type SetupItemState,
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
	state: 'Open',
	alerts_when_stuck: true,
	site_ids: ['00000000-0000-4000-8000-000000000002'],
	item: {
		type: 'DaemonHealth',
		daemon_id: '00000000-0000-4000-8000-000000000001',
		name: 'edge-01',
		site_id: '00000000-0000-4000-8000-000000000002',
		health: 'outdated'
	}
};

function setupEntry(
	type: Exclude<SetupItemType, 'DaemonHealth' | 'RunFirstScan'>,
	state: SetupItemState
) {
	return {
		key: { type: 'Org', item_type: type },
		tier: 'Setup',
		state,
		alerts_when_stuck: false,
		site_ids: [],
		item: { type }
	} as SetupListEntry;
}

/** One Issue, and Setup items in each state, in the order the server lists them. */
const entries: SetupListEntry[] = [
	daemonIssue,
	setupEntry('InstallDaemon', 'Done'),
	setupEntry('OpenTopology', 'Open'),
	setupEntry('ExploreViews', 'Locked'),
	setupEntry('Share', 'Locked'),
	setupEntry('Tags', 'Done')
];

describe('Setup list', () => {
	it('counts Locked items in the progress total and Done items in the numerator', () => {
		expect(setupProgress(entries)).toEqual({ done: 2, total: 5 });
	});

	it('lists only Open Setup items as open, never a Locked one or an Issue', () => {
		expect(openEntries(entries).map((e) => e.item.type)).toEqual(['OpenTopology']);
		expect(doneEntries(entries).map((e) => e.item.type)).toEqual(['InstallDaemon', 'Tags']);
	});

	it('badges Home with the Issue rows', () => {
		expect(setupIssueCount(entries)).toBe(1);
		const other: SetupListEntry = {
			...daemonIssue,
			key: { ...daemonIssue.key, entity_id: '00000000-0000-4000-8000-000000000003' }
		} as SetupListEntry;
		expect(setupIssueCount([...entries, other])).toBe(2);
		expect(setupIssueCount(entries.slice(1))).toBe(0);
	});

	describe('site chips', () => {
		const [home, branch] = [
			'00000000-0000-4000-8000-000000000002',
			'00000000-0000-4000-8000-000000000004'
		];
		const onBranch = {
			...daemonIssue,
			key: { ...daemonIssue.key, entity_id: '00000000-0000-4000-8000-000000000005' },
			site_ids: [branch]
		} as SetupListEntry;
		const orgWide = {
			...setupEntry('UnconfirmedHosts', 'Open'),
			tier: 'Issue'
		} as SetupListEntry;
		const issues = [daemonIssue, onBranch, orgWide];

		it('offers each site the Issues name, once', () => {
			expect(issueSiteIds(issues)).toEqual([home, branch]);
			expect(issueSiteIds([daemonIssue, daemonIssue])).toEqual([home]);
		});

		it("narrows to the picked sites' Issues and keeps org-wide ones", () => {
			expect(issuesOnSites(issues, new Set([branch]))).toEqual([onBranch, orgWide]);
			expect(issuesOnSites(issues, new Set())).toEqual([orgWide]);
			expect(issuesOnSites(issues, new Set([home, branch]))).toEqual(issues);
		});
	});
});
