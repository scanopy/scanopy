import type { components } from '$lib/api/schema';
import osFamilies from '$lib/data/os-families.json';
import { metaName } from '$lib/i18n/metadata';

export type OsFamily = components['schemas']['OsFamily'];

/** The field id a `placeholder_by` entry uses for the credential's own `daemon_os`. */
export const DAEMON_OS_FIELD = 'daemon_os';

interface PlaceholderField {
	placeholder?: string | null;
	placeholder_by?: { depends_on: string; value: string; placeholder: string }[] | null;
}

/**
 * The placeholder a field shows for the form's current values.
 *
 * The first `placeholder_by` entry whose field currently holds its value wins; otherwise the
 * field's own placeholder. `daemon_os` names the credential's daemon OS, which lives beside the
 * credential type's fields rather than among them.
 */
export function resolvePlaceholder(
	field: PlaceholderField,
	values: Record<string, string | undefined>,
	daemonOs: OsFamily
): string {
	const match = field.placeholder_by?.find(
		(dependent) =>
			(dependent.depends_on === DAEMON_OS_FIELD ? daemonOs : values[dependent.depends_on]) ===
			dependent.value
	);
	return match?.placeholder ?? field.placeholder ?? '';
}

/**
 * The placeholder for a field's "File on daemon host" path: the daemon OS's example directory
 * joined to the field's own `file_name`, e.g. `/etc/scanopy/snmp-community`.
 */
export function filePathPlaceholder(field: { file_name?: string | null }, os: OsFamily): string {
	const dir = osFamilies.find((family) => family.id === os)?.example_dir ?? '';
	return field.file_name ? `${dir}${field.file_name}` : dir;
}

/** Every OS family as radio options, with translated names. */
export function osFamilyOptions(): { value: OsFamily; label: string }[] {
	return osFamilies.map((family) => ({
		value: family.id as OsFamily,
		label: metaName('os_families', family.id, family.name)
	}));
}
