import { describe, it, expect } from 'vitest';
import {
	assignActionKeys,
	selectionActions,
	shownActions,
	DELETE_KEY,
	RESERVED_ACTION_KEYS
} from '$lib/shared/components/data/controls/actionShortcuts';
import type { RowAction } from '$lib/shared/components/data/types';
import type { IconComponent } from '$lib/shared/utils/types';

const action = (label: string, extra: Partial<RowAction> = {}): RowAction => ({
	label,
	icon: null as unknown as IconComponent,
	onClick: () => {},
	...extra
});

const keys = (actions: RowAction[]) => assignActionKeys(actions).map((a) => a.key);

describe('assignActionKeys', () => {
	it('takes each label’s first free letter, skipping the list’s own keys', () => {
		expect(keys([action('Edit'), action('Rescan'), action('Consolidate')])).toEqual([
			'E',
			'R',
			'O'
		]);
		expect(RESERVED_ACTION_KEYS).toContain('C');
	});

	it('gives the delete action Delete, whatever its label', () => {
		expect(keys([action('Edit'), action('Remove', { isDelete: true })])).toEqual(['E', DELETE_KEY]);
	});

	it('gives a danger action that is not the delete a letter', () => {
		expect(
			keys([action('Upgrade', { tone: 'danger' }), action('Delete', { isDelete: true })])
		).toEqual(['U', DELETE_KEY]);
	});

	it('moves to a later letter when an earlier action took the first', () => {
		expect(keys([action('Edit'), action('Enable')])).toEqual(['E', 'N']);
	});

	it('leaves an action keyless when every letter is taken', () => {
		expect(keys([action('Tc')])).toEqual([null]);
	});
});

describe('assignActionKeys with a preferred shortcut', () => {
	it('gives an action its preferred key ahead of an earlier label that would take it', () => {
		expect(
			keys([
				action('Consolidate'),
				action('Disable', { shortcut: 'O' }),
				action('Delete', { isDelete: true })
			])
		).toEqual(['N', 'O', DELETE_KEY]);
	});

	it('gives both states of one control their shared key', () => {
		expect(
			keys([
				action('Disable', { shortcut: 'O' }),
				action('Enable', { shortcut: 'O' }),
				action('Edit'),
				action('Run')
			])
		).toEqual(['O', 'O', 'E', 'R']);
	});
});

describe('selectionActions', () => {
	const bulk = () => {};
	const del = (extra: Partial<RowAction> = {}) =>
		action('Delete', { tone: 'danger', isDelete: true, onBulk: bulk, ...extra });
	const rescan = (extra: Partial<RowAction> = {}) => action('Rescan', { onBulk: bulk, ...extra });
	const busy: RowAction['unavailable'] = {
		state: 'disabled',
		reason: 'This one is busy',
		bulkReason: 'A selected one is busy'
	};
	const settled: RowAction['unavailable'] = { state: 'hidden', reason: 'Already settled' };
	const ran: string[] = [];
	const runBulk = (a: RowAction) => () => void ran.push(a.label);
	const reasons = { missing: 'missing', oneRowAtATime: 'one at a time' };
	const run = (perRow: RowAction[][]) => selectionActions(perRow, runBulk, reasons);
	const labels = (actions: RowAction[]) => actions.map((a) => a.label);

	it('shows one row’s actions as the row does, bulk ones running on the selection', () => {
		const [edit, remove] = run([
			[action('Resolve', { unavailable: settled }), action('Edit'), del()]
		]);
		expect(edit.label).toBe('Edit');
		expect(remove).toMatchObject({ label: 'Delete', tone: 'danger' });
		expect(remove.unavailable).toBeUndefined();
		remove.onClick();
		expect(ran).toContain('Delete');
	});

	it('gives one unavailable row its own reason', () => {
		expect(run([[del({ unavailable: busy })]])[0].unavailable?.reason).toBe('This one is busy');
	});

	it('shows every action any selected row shows, disabling those with no bulk form', () => {
		const actions = run([
			[action('Resolve'), action('Edit'), rescan(), del()],
			[action('Resolve', { unavailable: settled }), action('Edit'), rescan(), del()]
		]);
		expect(labels(actions)).toEqual(['Resolve', 'Edit', 'Rescan', 'Delete']);
		expect(actions[0].unavailable?.reason).toBe('one at a time');
		expect(actions[1].unavailable?.reason).toBe('one at a time');
		expect(actions[2].unavailable).toBeUndefined();
	});

	it('leaves out an action no selected row shows', () => {
		const hiddenEverywhere = action('Resolve', { unavailable: settled });
		expect(
			labels(
				run([
					[hiddenEverywhere, del()],
					[hiddenEverywhere, del()]
				])
			)
		).toEqual(['Delete']);
	});

	it('blocks a bulk action with the first blocking row’s selection wording', () => {
		expect(run([[del()], [del({ unavailable: busy })]])[0].unavailable?.reason).toBe(
			'A selected one is busy'
		);
		const cancel = action('Cancel', { onBulk: bulk });
		const hiddenCancel = action('Cancel', {
			onBulk: bulk,
			unavailable: {
				state: 'hidden',
				reason: 'Not running',
				bulkReason: 'A selected one isn’t running'
			}
		});
		expect(run([[cancel], [hiddenCancel]])[0].unavailable?.reason).toBe(
			'A selected one isn’t running'
		);
		const rescanOnOne = run([[rescan()], [action('Edit')]]).find((a) => a.label === 'Rescan');
		expect(rescanOnOne?.unavailable?.reason).toBe('missing');
	});

	it('keeps each row’s order, with Delete last', () => {
		const actions = run([
			[action('Disable', { onBulk: bulk }), action('Run', { onBulk: bulk }), del()],
			[action('Enable', { onBulk: bulk }), action('Run', { onBulk: bulk }), del()]
		]);
		expect(labels(actions)).toEqual(['Disable', 'Enable', 'Run', 'Delete']);
	});
});

describe('shownActions', () => {
	it('leaves out hidden actions and keeps disabled ones', () => {
		const shown = shownActions([
			action('Resolve', { unavailable: { state: 'hidden', reason: 'x' } }),
			action('Delete', { unavailable: { state: 'disabled', reason: 'y' } })
		]);
		expect(shown.map((a) => a.label)).toEqual(['Delete']);
	});
});
