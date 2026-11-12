/**
 * The selection toolbar's actions, built from the selected rows' own actions, and their keys.
 *
 * The delete action takes Delete, and an action naming a `shortcut` takes it when free. Every
 * other action takes the first letter of its label that no earlier action and no list-wide key
 * has claimed: Edit is E, Rescan R, Merge M, and an action starting with C its next letter, since C is Create. An action left with no
 * free letter shows without a key.
 */
import type { ActionTone, RowAction } from '../types';

/** Single keys the list already uses: C for Create, T for the tag island. */
export const RESERVED_ACTION_KEYS = ['C', 'T'];

/** The `key` value the delete action takes; Backspace triggers it too. */
export const DELETE_KEY = 'Delete';

export interface KeyedAction {
	action: RowAction;
	/** A capital letter, `DELETE_KEY`, or null when none was free. */
	key: string | null;
}

/** Why a selection of several rows can't run an action, when no row's own reason applies. */
export interface SelectionReasons {
	/** A selected row's type has no such action (an invite selected with users). */
	missing: string;
	/** The action has no bulk form, so it runs on one row at a time. */
	oneRowAtATime: string;
}

/** The actions a row draws: all but those `unavailable` in the `hidden` state. */
export function shownActions(actions: RowAction[]): RowAction[] {
	return actions.filter((action) => action.unavailable?.state !== 'hidden');
}

/**
 * The toolbar's actions for a selection, from each selected row's own actions (`perRow`).
 *
 * One row: the actions the row shows, as it shows them; one with `onBulk` runs through `runBulk`.
 * Several: every action any selected row shows, the same action across rows matched by label. An
 * action with `onBulk` runs on the whole selection through `runBulk`, and is available only when
 * every selected row has it available; otherwise it is disabled with the first blocking row's
 * `bulkReason ?? reason`, or `reasons.missing` when that row has no such action. An action without
 * `onBulk` is disabled with `reasons.oneRowAtATime`.
 */
export function selectionActions(
	perRow: RowAction[][],
	runBulk: (action: RowAction) => () => void,
	reasons: SelectionReasons
): RowAction[] {
	if (perRow.length === 1) {
		return shownActions(perRow[0]).map((action) =>
			action.onBulk && !action.unavailable ? { ...action, onClick: runBulk(action) } : action
		);
	}

	return mergeRowOrders(perRow.map(shownActions)).map((action): RowAction => {
		if (!action.onBulk) {
			return { ...action, unavailable: { state: 'disabled', reason: reasons.oneRowAtATime } };
		}
		const matches = perRow.map((actions) => actions.find((a) => a.label === action.label));
		const blocker = matches.findIndex((a) => !a || a.unavailable);
		if (blocker === -1) return { ...action, onClick: runBulk(action), unavailable: undefined };
		const blocked = matches[blocker]?.unavailable;
		const reason = blocked ? (blocked.bulkReason ?? blocked.reason) : reasons.missing;
		return { ...action, unavailable: { state: 'disabled', reason } };
	});
}

/**
 * Every distinct action across the rows, in the rows' own order: an action one row has and an
 * earlier row lacks goes right after its neighbour in that row, so Enable sits beside Disable. The
 * delete action goes last.
 */
function mergeRowOrders(rows: RowAction[][]): RowAction[] {
	const merged: RowAction[] = [];
	const indexOf = (label: string) => merged.findIndex((a) => a.label === label);
	for (const row of rows) {
		let after = -1;
		row.forEach((action, i) => {
			const at = indexOf(action.label);
			if (at !== -1) {
				after = at;
				return;
			}
			// After its predecessor in this row; with none yet placed, before its successor.
			const next = row
				.slice(i + 1)
				.map((a) => indexOf(a.label))
				.find((j) => j !== -1);
			const position = after !== -1 ? after + 1 : (next ?? merged.length);
			merged.splice(position, 0, action);
			after = position;
		});
	}
	return [...merged.filter((a) => !a.isDelete), ...merged.filter((a) => a.isDelete)];
}

/** An action's tooltip as it reads in its current state: why it's unavailable, or its hint. */
export function resolveTooltip(action: RowAction): string | undefined {
	return action.unavailable?.reason ?? action.tooltip;
}

// Whole class names, so Tailwind's content scan keeps them.
const ICON_BUTTON_CLASS: Record<ActionTone, string> = {
	danger: 'btn-icon-danger',
	warning: 'btn-icon-warning',
	success: 'btn-icon-success',
	info: 'btn-icon-info'
};
const TOOLBAR_BUTTON_CLASS: Record<ActionTone, string> = {
	danger: 'btn-danger',
	warning: 'btn-warning',
	success: 'btn-success',
	info: 'btn-info'
};

/** The class of an action's icon button in a table row. */
export function iconButtonClass(tone: ActionTone | undefined): string {
	return tone ? ICON_BUTTON_CLASS[tone] : 'btn-icon';
}

/** The class of an action's labelled button in the selection toolbar. */
export function toolbarButtonClass(tone: ActionTone | undefined): string {
	return tone ? TOOLBAR_BUTTON_CLASS[tone] : 'btn-secondary';
}

/** What key assignment reads from an action: a toolbar's `RowAction`, or a modal's. */
export type KeyableAction = Pick<RowAction, 'label' | 'isDelete' | 'shortcut'>;

export function assignActionKeys<A extends KeyableAction = RowAction>(
	actions: A[],
	reserved: readonly string[] = RESERVED_ACTION_KEYS
): { action: A; key: string | null }[] {
	const taken = new Set(reserved);
	// Preferred keys and Delete first, so a label earlier in the row can't take them. Actions
	// naming the same shortcut are one control in two states (Enable and Disable side by side in
	// a mixed selection), so each shows it.
	const shared = new Set<string>();
	const claimed = actions.map((action) => {
		const key = action.isDelete ? DELETE_KEY : action.shortcut?.toUpperCase();
		if (!key) return null;
		if (shared.has(key)) return key;
		if (taken.has(key)) return null;
		taken.add(key);
		if (action.shortcut) shared.add(key);
		return key;
	});
	return actions.map((action, i) => {
		if (claimed[i]) return { action, key: claimed[i] };
		const letter = [...action.label.toUpperCase()].find(
			(char) => char >= 'A' && char <= 'Z' && !taken.has(char)
		);
		if (letter) taken.add(letter);
		return { action, key: letter ?? null };
	});
}

/** Whether a keydown presses `key` (from `assignActionKeys`) with no modifier held. */
export function matchesActionKey(event: KeyboardEvent, key: string): boolean {
	if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey) return false;
	if (key === DELETE_KEY) return event.key === 'Delete' || event.key === 'Backspace';
	return event.key.toUpperCase() === key;
}
