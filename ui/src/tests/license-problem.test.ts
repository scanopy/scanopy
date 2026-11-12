import { describe, it, expect } from 'vitest';
import {
	licenseProblemBanner,
	licenseProblemMessage
} from '#lib/features/settings/license-problem.ts';
import { isLicenseLocked, type PublicServerConfig } from '#lib/shared/stores/config-query.ts';

const config = (license_status: PublicServerConfig['license_status']) =>
	({ license_status }) as PublicServerConfig;

describe('licenseProblemBanner', () => {
	it('keeps the cause and links the troubleshooting guide', () => {
		const banner = licenseProblemBanner('untrusted_certificate');
		expect(banner.startsWith(licenseProblemMessage('untrusted_certificate'))).toBe(true);
		expect(banner).toContain('/self-hosted-server/troubleshooting/');
	});

	it('gives different failures different messages', () => {
		expect(licenseProblemMessage('dns')).not.toEqual(licenseProblemMessage('proxy'));
	});
});

// The config query polls exactly while this holds, so a check-in that succeeds
// clears the banner without a reload, and a healthy server never polls.
describe('isLicenseLocked', () => {
	it('holds for every status a check-in can lift', () => {
		for (const status of ['pending', 'invalid', 'expired'] as const) {
			expect(isLicenseLocked(config(status))).toBe(true);
		}
	});

	it('does not hold for a valid license or a server with no license', () => {
		expect(isLicenseLocked(config('valid'))).toBe(false);
		expect(isLicenseLocked(config(null))).toBe(false);
	});
});
