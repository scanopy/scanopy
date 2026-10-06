import { describe, expect, it } from 'vitest';
import {
	credentialFieldValues,
	parseScriptSource
} from '$lib/features/credentials/utils/fieldValues';
import type { CredentialType } from '$lib/features/credentials/types/base';
import type { FieldDefinition } from '$lib/shared/stores/metadata';

const field = (id: string, field_type: string) => ({ id, field_type }) as FieldDefinition;

// The credential wizard rebuilds its shared form from each row's field values whenever a card is
// added or removed. An existing credential's row must therefore carry every saved value, in the
// raw form the inputs and `buildCredentialType` read, or its required fields come back empty.
describe('credentialFieldValues', () => {
	it('carries every saved value of an existing credential', () => {
		const secret = { mode: 'Inline', value: 'abc' };
		const values = credentialFieldValues(
			{
				type: 'ProxmoxApiToken',
				port: 8006,
				token_id: 'scanopy@pve!discovery',
				token_secret: secret
			} as unknown as CredentialType,
			[
				field('port', 'port'),
				field('token_id', 'string'),
				field('token_secret', 'secretpathorinline')
			]
		);

		expect(values.token_id).toBe('scanopy@pve!discovery');
		expect(values.port).toBe('8006');
		expect(JSON.parse(values.token_secret)).toEqual(secret);
		expect(values).not.toHaveProperty('type');
	});

	it('keeps a script source readable by the form', () => {
		const source = { mode: 'DaemonFile', path: '/etc/scanopy/probe.sh' };
		const values = credentialFieldValues(
			{ type: 'Script', script: source } as unknown as CredentialType,
			[field('script', 'scriptsource')]
		);
		expect(parseScriptSource(values.script)).toEqual(source);
	});
});
