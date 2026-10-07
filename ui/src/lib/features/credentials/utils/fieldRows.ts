import type { FieldDefinition } from '$lib/shared/stores/metadata';

/**
 * Lay a group's fields out in rows: two adjacent half-width fields share a row, every other field
 * takes one of its own. A half-width field with no half-width neighbour stays full width, so a
 * field hidden by its dependency never leaves its partner stranded at half width.
 */
export function fieldRows<F extends Pick<FieldDefinition, 'half_width'>>(fields: F[]): F[][] {
	const rows: F[][] = [];
	for (let i = 0; i < fields.length; i++) {
		const field = fields[i];
		const next = fields[i + 1];
		if (field.half_width && next?.half_width) {
			rows.push([field, next]);
			i++;
		} else {
			rows.push([field]);
		}
	}
	return rows;
}
