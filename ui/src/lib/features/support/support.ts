import type { BillingPlanFeatures } from '#lib/shared/stores/metadata.ts';

/** Booking page for demos and plan onboarding calls. */
export const BOOK_CALL_URL = 'https://cal.com/mferrandiz/scanopy-demo';

/** The bug report form on GitHub. */
export const BUG_REPORT_URL =
	'https://github.com/scanopy/scanopy/issues/new?template=bug_report.md';

/** A mailto link to billing@ with the subject and body filled in. */
export function billingMailto(subject: string, body: string): string {
	return `mailto:billing@scanopy.net?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
}

/**
 * The support address a plan's email card sends to, or `null` when the plan
 * has no email support. Priority-support plans get their own alias.
 */
export function supportEmailAddress(
	features: Pick<BillingPlanFeatures, 'email_support' | 'priority_support'> | undefined
): string | null {
	if (!features?.email_support) return null;
	return features.priority_support ? 'priority@scanopy.net' : 'support@scanopy.net';
}

/** Where in the app a visitor opened the beta program sign-up. The website records it with the consent. */
export type BetaProgramSource = 'app_settings' | 'app_plans';

/**
 * The beta program sign-up on the website: the MCP Server page, which explains the feature the
 * program opens first and holds the form. The query names only where the link was opened and the
 * deployment, so nothing that identifies the user leaves a self-hosted server until they type it
 * into the form.
 */
export function betaProgramUrl(source: BetaProgramSource, cloud: boolean): string {
	const params = new URLSearchParams({ source, deployment: cloud ? 'cloud' : 'self_hosted' });
	return `https://scanopy.net/product/mcp-server?${params}`;
}
