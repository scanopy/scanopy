import type { FieldConfig } from '../types';
import { serverFilterViolations, type ServerFilterMode } from './filtering';
import { serverOrderViolations } from './sorting';
import { treeDepthViolations } from './grouping';

/**
 * Fail loudly on field config a server-paginated list cannot honour.
 *
 * Call during component init. Dev/test only, so it never reaches a user.
 */
export function guardServerPaginatedConfig<T>(
	get: () => { fields: FieldConfig<T>[]; serverPaginated: boolean; server: ServerFilterMode }
) {
	// A filterable field nobody handles server-side is always a bug on a
	// server-paginated list: the client holds one page, so filtering here
	// narrows that page while `total_count` keeps describing the whole match
	// set — the list reads "62 of 1550" and pages through the wrong rows. The
	// rule is documented on `FieldConfig.serverFiltered`; this makes it fail
	// loudly instead of silently, at the one place that sees every field's
	// resolved config.
	$effect(() => {
		if (!import.meta.env.DEV) return;

		const { fields, serverPaginated, server } = get();
		const offenders = serverFilterViolations(fields, serverPaginated, server);
		if (offenders.length === 0) return;

		throw new Error(
			`DataControls: ${offenders.join(', ')} ${offenders.length === 1 ? 'is' : 'are'} ` +
				`filterable on a server-paginated list but filtered client-side, which narrows ` +
				`only the loaded page while the count describes every match. Mark the field ` +
				`serverFiltered and handle it in onFilterChange, or drop its filterable flag.`
		);
	});

	// The same failure for ordering: a display field opted into client-side
	// sorting or grouping would reorder or bucket only the loaded page while the
	// pager walks the server's order. Such a field is never offered under server
	// pagination, so the flag is dead config that reads as a promise; this makes
	// it fail loudly.
	$effect(() => {
		if (!import.meta.env.DEV) return;

		const { fields, serverPaginated } = get();
		const offenders = serverOrderViolations(fields, serverPaginated);
		if (offenders.length === 0) return;

		throw new Error(
			`DataControls: ${offenders.join(', ')} ${offenders.length === 1 ? 'is' : 'are'} ` +
				`sortable or groupable client-side on a server-paginated list, which would order ` +
				`only the loaded page. Add the field to the backend's OrderField, or drop its ` +
				`sortable and groupable flags.`
		);
	});

	// A tree field on a server-paginated list must carry the server's depth: a parent can sit on
	// another page, so deriving depth from the loaded rows would flatten every row whose parent
	// is not on this one.
	$effect(() => {
		if (!import.meta.env.DEV) return;

		const { fields, serverPaginated } = get();
		const offenders = treeDepthViolations(fields, serverPaginated);
		if (offenders.length === 0) return;

		throw new Error(
			`DataControls: ${offenders.join(', ')} ${offenders.length === 1 ? 'draws' : 'draw'} a ` +
				`tree on a server-paginated list without tree.depth. Return each row's depth from ` +
				`the server and pass it as tree.depth.`
		);
	});
}
