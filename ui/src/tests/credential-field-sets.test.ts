import { describe, it, expect } from 'vitest';
import credentialTypes from '#lib/data/credential-types.json';
import { shownFields } from '#lib/features/credentials/utils/fieldSets.ts';
import type { FieldDefinition } from '#lib/shared/stores/metadata.ts';

// The SNMPv3 form as the backend defines it, so this fails if the field set and the form drift.
const snmpV3Fields = credentialTypes.find((t) => t.id === 'SnmpV3')?.metadata
	?.fields as unknown as FieldDefinition[];

const shownAt = (level?: string) =>
	shownFields(snmpV3Fields, level ? { security_level: level } : {}).map((f) => f.id);

describe('SNMPv3 security level field set', () => {
	it('shows only the user name at NoAuthNoPriv', () => {
		expect(shownAt('NoAuthNoPriv')).not.toContain('auth_password');
		expect(shownAt('NoAuthNoPriv')).not.toContain('priv_password');
		expect(shownAt('NoAuthNoPriv')).toContain('security_name');
	});

	it('adds authentication, not privacy, at AuthNoPriv', () => {
		expect(shownAt('AuthNoPriv')).toEqual(
			expect.arrayContaining(['auth_protocol', 'auth_password'])
		);
		expect(shownAt('AuthNoPriv')).not.toContain('priv_password');
		expect(shownAt('AuthNoPriv')).not.toContain('priv_protocol');
	});

	it('shows every field at AuthPriv, and opens on it', () => {
		expect(shownAt('AuthPriv')).toEqual(snmpV3Fields.map((f) => f.id));
		// No level picked yet: the dropdown's default decides, which keeps existing credentials
		// (all AuthPriv) looking as they did.
		expect(shownAt()).toEqual(shownAt('AuthPriv'));
	});
});

describe('shownFields', () => {
	it('leaves a type without a field set alone', () => {
		const fields = [{ id: 'community', default_value: null, field_set: null }];
		expect(shownFields(fields, {})).toEqual(fields);
	});
});
