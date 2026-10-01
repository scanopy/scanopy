import type { components } from '$lib/api/schema';
import type { FieldDefinition } from '$lib/shared/stores/metadata';

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
