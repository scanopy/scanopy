import { describe, it, expect } from 'vitest';
import { resolvePlaceholder } from '$lib/features/credentials/utils/placeholders';

describe('resolvePlaceholder', () => {
	const script = {
		placeholder: '/usr/local/bin/inventory',
		placeholder_by: [
			{ depends_on: 'target_os', value: 'Windows', placeholder: 'C:\\inventory.ps1' }
		]
	};
	const socket = {
		placeholder: '/var/run/docker.sock',
		placeholder_by: [
			{ depends_on: 'daemon_os', value: 'Windows', placeholder: '\\\\.\\pipe\\docker_engine' }
		]
	};

	it('uses the dependent placeholder while the sibling field holds its value', () => {
		expect(resolvePlaceholder(script, { target_os: 'Windows' }, 'Unix')).toBe('C:\\inventory.ps1');
	});

	it('falls back to the field placeholder for any other sibling value', () => {
		expect(resolvePlaceholder(script, { target_os: 'Unix' }, 'Windows')).toBe(
			'/usr/local/bin/inventory'
		);
		expect(resolvePlaceholder(script, {}, 'Windows')).toBe('/usr/local/bin/inventory');
	});

	it('reads daemon_os from the credential, not from the type fields', () => {
		expect(resolvePlaceholder(socket, { daemon_os: 'Unix' }, 'Windows')).toBe(
			'\\\\.\\pipe\\docker_engine'
		);
		expect(resolvePlaceholder(socket, { daemon_os: 'Windows' }, 'Unix')).toBe(
			'/var/run/docker.sock'
		);
	});

	it('returns an empty string for a field with no placeholder at all', () => {
		expect(resolvePlaceholder({ placeholder: null }, {}, 'Unix')).toBe('');
	});
});
