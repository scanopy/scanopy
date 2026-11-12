import type { FieldDefinition } from '#lib/shared/stores/metadata.ts';

type FieldSetField = Pick<FieldDefinition, 'id' | 'field_set' | 'default_value'>;

/**
 * The fields a form shows for its current values.
 *
 * A type with a field set has one dropdown marked `Sets` (e.g. SNMPv3's security level). A field
 * marked `{ Requires: [...] }` shows only while that dropdown holds one of the listed values;
 * every other field always shows. The dropdown reads its default until the user picks.
 */
export function shownFields<F extends FieldSetField>(
	fields: F[],
	values: Record<string, string | undefined>
): F[] {
	const selector = fields.find((field) => field.field_set === 'Sets');
	if (!selector) return fields;
	const selected = values[selector.id] || selector.default_value || '';
	return fields.filter((field) => {
		const role = field.field_set;
		if (!role || role === 'Sets') return true;
		return role.Requires.includes(selected);
	});
}
