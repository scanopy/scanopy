import type { components } from '$lib/api/schema';
import type { FieldDefinition } from '$lib/shared/stores/metadata';
import type { CredentialType } from '../types/base';

export type ScriptSource = components['schemas']['ScriptSource'];
export type ScriptSourceMode = ScriptSource['mode'];

/**
 * A new credential's starting value for one field, as the form's raw string.
 *
 * Structured fields hold their wire object as JSON, which `CredentialForm.buildCredentialType`
 * parses back. Shared by the form and the credential wizard, which seeds the same values.
 */
export function defaultFieldValue(field: FieldDefinition): string {
	if (field.field_type === 'pathorinline') {
		return JSON.stringify({ mode: 'Inline', value: '' });
	}
	if (field.field_type === 'scriptsource') {
		return JSON.stringify({ mode: 'HostFile', path: '' } satisfies ScriptSource);
	}
	return field.default_value ?? '';
}

/**
 * A saved credential's field values, as the form's raw strings. Structured fields (secrets, files,
 * script sources) hold their wire object as JSON, matching `defaultFieldValue`.
 *
 * Shared by the form, which renders them, and the credential wizard, which carries them on each
 * row so that rebuilding the shared form's defaults keeps an existing credential's values.
 */
export function credentialFieldValues(
	credentialType: CredentialType,
	fields: FieldDefinition[]
): Record<string, string> {
	const fieldMap = new Map(fields.map((f) => [f.id, f]));
	const values: Record<string, string> = {};
	for (const [key, val] of Object.entries(credentialType as unknown as Record<string, unknown>)) {
		if (key === 'type') continue;
		const fieldType = fieldMap.get(key)?.field_type;
		const structured =
			fieldType === 'secretpathorinline' ||
			fieldType === 'pathorinline' ||
			fieldType === 'scriptsource';
		if (structured && val != null && typeof val === 'object') {
			values[key] = JSON.stringify(val);
		} else {
			values[key] = val != null ? String(val) : '';
		}
	}
	return values;
}

/** A `scriptsource` field's raw form value as its wire object. Unparseable text is a host path. */
export function parseScriptSource(raw: string | undefined): ScriptSource {
	if (raw) {
		try {
			const parsed = JSON.parse(raw) as Partial<{ mode: string; path: string; value: string }>;
			if (parsed.mode === 'Inline') return { mode: 'Inline', value: parsed.value ?? '' };
			if (parsed.mode === 'DaemonFile') return { mode: 'DaemonFile', path: parsed.path ?? '' };
			if (parsed.mode === 'HostFile') return { mode: 'HostFile', path: parsed.path ?? '' };
		} catch {
			// Plain text: fall through and treat it as a host path.
		}
	}
	return { mode: 'HostFile', path: raw ?? '' };
}

/** The text the user typed, whichever mode holds it. */
export function scriptSourceText(source: ScriptSource): string {
	return source.mode === 'Inline' ? source.value : source.path;
}

/** The same text under another mode, so switching modes never discards what was typed. */
export function withScriptSourceMode(source: ScriptSource, mode: ScriptSourceMode): ScriptSource {
	const text = scriptSourceText(source);
	return mode === 'Inline' ? { mode, value: text } : { mode, path: text };
}

/** `source` with its text replaced, keeping the mode. */
export function withScriptSourceText(source: ScriptSource, text: string): ScriptSource {
	return source.mode === 'Inline' ? { mode: 'Inline', value: text } : { ...source, path: text };
}
