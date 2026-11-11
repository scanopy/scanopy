import { getLocale } from '#lib/paraglide/runtime.js';
import { common_andNMore } from '#lib/paraglide/messages.js';

/** How many items a list names before eliding the rest. A line long enough to scroll is not read. */
export const MAX_LISTED = 10;

/**
 * Join as a localized list, capped, saying how many were left out.
 *
 * `conjunction` for things that are all true at once ("A, B, and C stopped responding"),
 * `disjunction` for alternatives ("LLDP neighbours or the ARP table"). `Intl.ListFormat` joins in
 * the reader's own language.
 */
export function joinList(values: string[], type: 'conjunction' | 'disjunction'): string {
	const unique = [...new Set(values)];
	const listed = unique.slice(0, MAX_LISTED);
	const elided = unique.length - listed.length;
	if (elided > 0) {
		// A capped list that simply stops reads as though that was all of them.
		listed.push(common_andNMore({ count: elided }));
	}
	return new Intl.ListFormat(getLocale(), { style: 'long', type }).format(listed);
}
