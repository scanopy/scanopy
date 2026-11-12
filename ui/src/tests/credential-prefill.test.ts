import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { get } from 'svelte/store';
import { closeModal, initModalFromUrl, modalState } from '#lib/shared/stores/modal-registry.ts';
import {
	openCredentialCreate,
	prefillCredentialName,
	prefillFor,
	typesForIntegration
} from '#lib/features/credentials/utils/prefill.ts';
import type { CredentialTypeMetadata } from '#lib/shared/stores/metadata.ts';
import { attachUrlState } from '#lib/shared/stores/url-state.ts';

const type = (
	id: string,
	integration: CredentialTypeMetadata['integration'],
	targets: CredentialTypeMetadata['targets']
) => ({ id, metadata: { fields: [], integration, targets } });

const types = [
	type('SnmpCommunity', 'Snmp', ['DaemonHost', 'Hosts', 'Site']),
	type('SnmpV3', 'Snmp', ['DaemonHost', 'Hosts', 'Site']),
	type('DockerProxy', 'Docker', ['DaemonHost', 'Hosts']),
	type('DockerSocket', 'Docker', ['DaemonHost']),
	type('UnifiApiKey', 'UnifiController', ['DaemonHost', 'Hosts']),
	type('UnifiLocalAdmin', 'UnifiController', ['DaemonHost', 'Hosts'])
];

describe('typesForIntegration', () => {
	it('offers only transports that can apply where the flow comes assigned', () => {
		expect(typesForIntegration(types, 'Docker', 'Hosts')).toEqual(['DockerProxy']);
		expect(typesForIntegration(types, 'Docker', null)).toEqual(['DockerProxy', 'DockerSocket']);
		expect(typesForIntegration(types, 'Docker', 'Site')).toEqual([]);
	});

	it('keeps every transport of an integration with several, in fixture order', () => {
		expect(typesForIntegration(types, 'UnifiController', 'Hosts')).toEqual([
			'UnifiApiKey',
			'UnifiLocalAdmin'
		]);
	});
});

describe('prefillFor', () => {
	const found = { hostIds: ['h1', 'h2'], siteIds: ['s1'] };

	// Nothing says which community or version a device accepts, so the credential goes where the
	// scan tries it on every host and pins it where it works.
	it('assigns a site-capable integration to the site, not the hosts', () => {
		expect(prefillFor(types, 'Snmp', found)).toEqual({ integration: 'Snmp', sites: 's1' });
	});

	it('assigns a host-only integration to the hosts it was found on', () => {
		expect(prefillFor(types, 'Docker', found)).toEqual({ integration: 'Docker', hosts: 'h1,h2' });
	});
});

describe('openCredentialCreate', () => {
	let href = 'http://localhost/#home';
	let detach: () => void;
	beforeEach(() => {
		vi.stubGlobal('window', {
			get location() {
				const url = new URL(href);
				return {
					href,
					search: url.search,
					get hash() {
						return url.hash;
					},
					set hash(value: string) {
						url.hash = value;
						href = url.toString();
					}
				};
			},
			history: {
				replaceState: (_: unknown, __: string, next: string) => {
					href = next;
				}
			},
			addEventListener: () => {},
			removeEventListener: () => {}
		});
		detach = attachUrlState();
	});
	afterEach(() => {
		closeModal();
		detach();
		vi.unstubAllGlobals();
		href = 'http://localhost/#home';
	});

	// The digest email links the same URL, so a reload or an emailed link must open the same form.
	it('survives the URL: a reload opens the same pre-filled create flow', async () => {
		openCredentialCreate('Proxmox', { hostIds: ['h1', 'h2'], siteIds: ['s1'] });
		await Promise.resolve();
		const link = href;

		// A fresh page load at that URL.
		closeModal();
		await Promise.resolve();
		detach();
		href = link;
		detach = attachUrlState();
		initModalFromUrl();

		expect(get(modalState)).toMatchObject({
			name: 'credential-editor',
			id: null,
			prefill: { integration: 'Proxmox', hosts: 'h1,h2' }
		});
		expect(new URL(link).hash).toBe('#credentials');
	});
});

describe('prefillCredentialName', () => {
	it('names a one-host credential after the host, snake_cased', () => {
		expect(prefillCredentialName([{ display_name: 'DC Proxmox-HV01.lan' }], 'Proxmox VE')).toBe(
			'dc_proxmox_hv01_lan'
		);
	});

	it('names it after the integration when one host would mislabel the rest', () => {
		expect(
			prefillCredentialName([{ display_name: 'switch-a' }, { display_name: 'switch-b' }], 'SNMP')
		).toBe('snmp');
	});

	it('names it after the integration when the host has no name of its own', () => {
		expect(prefillCredentialName([{ display_name: null }], 'Proxmox VE')).toBe('proxmox_ve');
	});
});
