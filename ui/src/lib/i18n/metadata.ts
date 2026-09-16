/**
 * Translation utilities for backend-metadata fixture strings.
 *
 * Fixture files in $lib/data/*.json ship English names/descriptions generated
 * from the backend. `scripts/generate-meta-messages.js` syncs those strings
 * into messages/en.json as `meta_<fixtureKey>_<id>_*` keys; this module
 * resolves them dynamically (mirroring $lib/i18n/errors.ts), falling back to
 * the raw fixture string when no translation exists.
 */

import * as m from '$lib/paraglide/messages';

/**
 * The translatable subset of a field definition. Optionals admit `null` because the generated
 * `FieldDefinition` schema renders Rust's `Option<T>` that way, and this must stay a supertype of
 * it — every caller passes one.
 */
interface TranslatableField {
	id: string;
	label: string;
	placeholder?: string | null;
	help_text?: string | null;
	options?: { value: string; label: string }[] | null;
}

/**
 * Fill `{named}` slots in a fixture string, for the fallback path only.
 *
 * Paraglide does the interpolation on the translated path; this exists for when no message is
 * compiled for a key, which would otherwise show the operator a raw `{addresses}`.
 */
function interpolate(template: string, params: Record<string, unknown>): string {
	return template.replace(/\{(\w+)\}/g, (whole, name: string) =>
		name in params ? String(params[name]) : whole
	);
}

/**
 * Resolve a fixture item's translated description and fill its slots.
 *
 * Separate from `metaDescription` because a message with slots compiles to a function that
 * *requires* an inputs object — calling it with none, as `resolveMeta` does, throws and silently
 * degrades to the untranslated fixture string. Modelled on `translateError` in ./errors.ts, which
 * has the same shape for the same reason.
 */
export function metaDescriptionWith(
	fixtureKey: string,
	id: string | null,
	params: Record<string, unknown>,
	fallback: string
): string {
	if (!id) return interpolate(fallback, params);
	return resolveMetaWith(`meta_${fixtureKey}_${id}_description`, params, fallback);
}

/**
 * Resolve a fixture item's translated name and fill its slots.
 *
 * The name-side counterpart of `metaDescriptionWith`, for fixtures whose names are templates:
 * `attribute-sources.json` names `Probe` and `Authored` as `{probe}` sentences.
 */
export function metaNameWith(
	fixtureKey: string,
	id: string | null,
	params: Record<string, unknown>,
	fallback: string
): string {
	if (!id) return interpolate(fallback, params);
	return resolveMetaWith(`meta_${fixtureKey}_${id}_name`, params, fallback);
}

function resolveMetaWith(key: string, params: Record<string, unknown>, fallback: string): string {
	const messageFn = m[key as keyof typeof m] as
		| ((inputs: Record<string, unknown>) => string)
		| undefined;

	if (typeof messageFn === 'function') {
		try {
			return messageFn(params);
		} catch {
			// Fall through to the fixture string, interpolated the same way.
		}
	}

	return interpolate(fallback, params);
}

function resolveMeta(key: string, fallback: string): string {
	const messageFn = m[key as keyof typeof m] as (() => string) | undefined;

	if (typeof messageFn === 'function') {
		try {
			return messageFn();
		} catch {
			// If resolution fails, fall through to fallback
		}
	}

	return fallback;
}

/** Resolve a fixture item's translated name, falling back to the fixture string. */
export function metaName(fixtureKey: string, id: string | null, fallback: string): string {
	if (!id) return fallback;
	return resolveMeta(`meta_${fixtureKey}_${id}_name`, fallback);
}

/** Resolve a fixture item's translated description, falling back to the fixture string. */
export function metaDescription(fixtureKey: string, id: string | null, fallback: string): string {
	if (!id) return fallback;
	return resolveMeta(`meta_${fixtureKey}_${id}_description`, fallback);
}

/**
 * Resolve translated label/placeholder/help_text/option labels for field
 * definitions. `ownerId` namespaces nested fields (credential type id);
 * pass null for flat field-definition fixtures like scan-settings.
 *
 * Key shape mirrors scripts/generate-meta-messages.js:
 *   meta_<fixtureKey>[_<ownerId>]_<fieldId>_label / _placeholder / _helpText
 *   meta_<fixtureKey>[_<ownerId>]_<fieldId>_option_<value>
 */
export function translateFieldDefinitions<F extends TranslatableField>(
	fixtureKey: string,
	ownerId: string | null,
	fields: F[]
): F[] {
	const namespace = ownerId ? `meta_${fixtureKey}_${ownerId}` : `meta_${fixtureKey}`;
	return fields.map((field) => {
		const prefix = `${namespace}_${field.id}`;
		return {
			...field,
			label: resolveMeta(`${prefix}_label`, field.label),
			placeholder: field.placeholder
				? resolveMeta(`${prefix}_placeholder`, field.placeholder)
				: field.placeholder,
			help_text: field.help_text
				? resolveMeta(`${prefix}_helpText`, field.help_text)
				: field.help_text,
			options: field.options?.map((option) => ({
				...option,
				label: resolveMeta(`${prefix}_option_${option.value}`, option.label)
			}))
		};
	});
}
