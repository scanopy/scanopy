import type { components } from '#lib/api/schema';
import {
	license_problem_certificateDates,
	license_problem_connect,
	license_problem_dns,
	license_problem_instanceLimit,
	license_problem_keyMalformed,
	license_problem_keyRefused,
	license_problem_keyRegenerated,
	license_problem_organizationMissing,
	license_problem_planHasNoKey,
	license_problem_proxy,
	license_problem_rateLimited,
	license_problem_serverError,
	license_problem_timeout,
	license_problem_untrustedCertificate,
	common_troubleshooting
} from '#lib/paraglide/messages.js';
import { slot } from '#lib/shared/components/data/MessageWithSlots.svelte';
import type { MessageLink } from '#lib/shared/components/data/MessageWithLinks.svelte';
import { docsUrl } from '#lib/shared/utils/docs.ts';
import { LICENSE_PORTAL_URL } from '#lib/shared/utils/site-links.ts';

export type LicenseProblem = components['schemas']['LicenseProblem'];

const messages: Record<LicenseProblem, () => string> = {
	dns: () => license_problem_dns(),
	connect: () => license_problem_connect(),
	timeout: () => license_problem_timeout(),
	untrusted_certificate: () => license_problem_untrustedCertificate(),
	certificate_dates: () => license_problem_certificateDates(),
	proxy: () => license_problem_proxy(),
	server_error: () => license_problem_serverError(),
	rate_limited: () => license_problem_rateLimited(),
	key_malformed: () => license_problem_keyMalformed(),
	key_regenerated: () => license_problem_keyRegenerated(),
	organization_missing: () => license_problem_organizationMissing(),
	plan_has_no_key: () => license_problem_planHasNoKey(),
	instance_limit: () => license_problem_instanceLimit(),
	key_refused: () => license_problem_keyRefused()
};

/** The cause and next step, as plain text. */
export const licenseProblemMessage = (problem: LicenseProblem): string => messages[problem]();

/** The cause and next step, ending with a link to the troubleshooting guide. Render it with
 *  `licenseBannerLinks`. */
export const licenseProblemBanner = (problem: LicenseProblem): string =>
	`${licenseProblemMessage(problem)} ${slot('troubleshoot')}`;

/** The links the license banners place: `link` is where the license is managed, and
 *  `troubleshoot` is the troubleshooting guide a problem banner ends with. */
export const licenseBannerLinks = (): Record<string, MessageLink> => ({
	// Named by its host, which reads the same in every language.
	link: { href: LICENSE_PORTAL_URL, label: new URL(LICENSE_PORTAL_URL).host },
	troubleshoot: {
		href: docsUrl('/docs/troubleshooting/self-hosted-server/?edition=commercial'),
		label: common_troubleshooting()
	}
});
