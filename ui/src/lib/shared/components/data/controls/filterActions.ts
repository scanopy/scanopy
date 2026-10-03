import { getFieldKey, type FieldConfig } from '../types';
import {
	blankFilter,
	blankFilterState,
	booleanFilterValues,
	toggleBoolean,
	toggleValue,
	type FilterState
} from './filtering';

/**
 * The filter state and parent callbacks the actions read and write.
 *
 * Accessors rather than values, so the component passes its live `$state`
 * through getters and setters and every action sees the current selection.
 */
export interface FilterActionsContext<T> {
	readonly fields: FieldConfig<T>[];
	filterState: FilterState;
	staleOnly: boolean;
	readonly onFilterChange: ((fieldKey: string, values: string[]) => void) | null;
	readonly onStaleFilterChange: ((stale: boolean | null) => void) | null;
	/** A narrowed filter invalidates the current offset. */
	resetToFirstPage: () => void;
}

export type FilterActions = ReturnType<typeof createFilterActions>;

/**
 * Every way a user changes a filter, shared by the card pane and the table
 * headers so both edit one selection and notify the parent the same way.
 *
 * Tag selections are only recorded here: the component's tag effect notifies
 * the parent, whichever control changed them.
 */
export function createFilterActions<T>(ctx: FilterActionsContext<T>) {
	/** Whether the parent, not the client pass, acts on this field's filter. */
	function isServerFiltered(fieldKey: string): boolean {
		if (!ctx.onFilterChange) return false;
		return ctx.fields.find((f) => getFieldKey(f) === fieldKey)?.serverFiltered === true;
	}

	/**
	 * Tell the parent a server-side filter was reset. A cleared boolean is both
	 * boxes checked, not an empty selection — an empty one would read as "show
	 * neither" to a parent that maps the values literally.
	 */
	function notifyCleared(field: FieldConfig<T>) {
		ctx.onFilterChange?.(
			getFieldKey(field),
			field.type === 'boolean' ? booleanFilterValues(true, true) : []
		);
	}

	function clearStale() {
		if (!ctx.staleOnly) return;
		ctx.staleOnly = false;
		ctx.onStaleFilterChange?.(null);
	}

	return {
		toggleString(fieldKey: string, value: string) {
			const next = toggleValue(ctx.filterState, fieldKey, value);
			if (!next) return;

			ctx.filterState = next;

			if (isServerFiltered(fieldKey)) {
				ctx.onFilterChange!(fieldKey, Array.from(next[fieldKey].values));
				ctx.resetToFirstPage();
			}
		},

		toggleBoolean(fieldKey: string, box: 'showTrue' | 'showFalse') {
			const serverSide = isServerFiltered(fieldKey);
			const next = toggleBoolean(ctx.filterState, fieldKey, box, serverSide);
			if (!next) return;

			ctx.filterState = next.state;

			if (serverSide) {
				ctx.onFilterChange!(fieldKey, booleanFilterValues(next.showTrue, next.showFalse));
				ctx.resetToFirstPage();
			}
		},

		// Uses tag ids, which is what the server filters by.
		toggleTag(tagId: string) {
			const next = toggleValue(ctx.filterState, 'tags', tagId);
			if (next) ctx.filterState = next;
		},

		toggleStale() {
			ctx.staleOnly = !ctx.staleOnly;
			// `null` rather than `false` — unchecking means "no staleness
			// constraint", not "show me only fresh entities".
			ctx.onStaleFilterChange?.(ctx.staleOnly ? true : null);
		},

		/** Clear every filter. No defaults: clearing clears, including a default the product chose. */
		clearAll() {
			ctx.filterState = blankFilterState(ctx.fields, false);
			clearStale();

			if (ctx.onFilterChange) {
				ctx.fields
					.filter((field) => field.filterable && field.serverFiltered)
					.forEach(notifyCleared);
			}
		},

		/** Clear one column's filter, including its stale toggle when it carries one. */
		clearField(fieldKey: string) {
			const field = ctx.fields.find((f) => getFieldKey(f) === fieldKey);
			if (!field) return;

			if (field.staleFilter) clearStale();
			if (!field.filterable) return;

			ctx.filterState = { ...ctx.filterState, [fieldKey]: blankFilter(field, false) };

			if (isServerFiltered(fieldKey)) {
				notifyCleared(field);
				ctx.resetToFirstPage();
			}
		},

		/** Tell the parent about server-side filters cleared outside these actions. */
		notifyServerCleared(fieldKeys: string[]) {
			const cleared = ctx.fields.filter(
				(f) => fieldKeys.includes(getFieldKey(f)) && isServerFiltered(getFieldKey(f))
			);
			cleared.forEach(notifyCleared);
			if (cleared.length > 0) ctx.resetToFirstPage();
		}
	};
}
