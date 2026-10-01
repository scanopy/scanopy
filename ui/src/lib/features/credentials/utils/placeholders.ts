import type { components } from '$lib/api/schema';
import osFamilies from '$lib/data/os-families.json';
import { metaName } from '$lib/i18n/metadata';

export type OsFamily = components['schemas']['OsFamily'];

/** The field id a `placeholder_by` entry uses for the credential's own `daemon_os`. */
export const DAEMON_OS_FIELD = 'daemon_os';

interface PlaceholderField {
	placeholder?: string | null;
	placeholder_by?: { field: string; value: string; placeholder: string }[] | null;
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
			(dependent.field === DAEMON_OS_FIELD ? daemonOs : values[dependent.field]) === dependent.value
	);
	return match?.placeholder ?? field.placeholder ?? '';
}

/** An example absolute path on a daemon running `os`, for a file-path input's placeholder. */
export function exampleFilePath(os: OsFamily): string {
	return osFamilies.find((family) => family.id === os)?.example_file_path ?? '';
}

/** Every OS family as radio options, with translated names. */
export function osFamilyOptions(): { value: OsFamily; label: string }[] {
	return osFamilies.map((family) => ({
		value: family.id as OsFamily,
		label: metaName('os_families', family.id, family.name)
	}));
}
