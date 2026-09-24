import { queryClient, queryKeys } from '$lib/api/query-client';
import type { PublicServerConfig } from '$lib/shared/stores/config-query';
import { trackEvent } from '$lib/shared/utils/analytics';
import { openModal } from '$lib/shared/stores/modal-registry';
import { upgradeContext, reopenSettingsAfterBilling } from '$lib/features/billing/stores';
import type { UpgradeFeature } from '$lib/shared/stores/metadata';
import type { Organization } from '$lib/features/organizations/types';
import { pushError } from '$lib/shared/stores/feedback';
import { formatDate } from '$lib/shared/utils/formatting';
import { errors_billing_air_gapped_plan_change_blocked } from '$lib/paraglide/messages';

const PRICING_URL_BASE = 'https://scanopy.net/pricing';

function pricingUrlFor(surface: PaywallSurface): string {
	const params = new URLSearchParams({
		utm_source: 'app',
		utm_medium: 'in_app',
		utm_campaign: 'plan_upgrade',
		utm_content: surface
	});
	return `${PRICING_URL_BASE}?${params.toString()}`;
}

export type PaywallSurface =
	| 'export_modal'
	| 'discovery_form'
	| 'share_panel'
	| 'sidebar'
	| 'billing_tab'
	| 'home_plan_usage'
	| 'networks_tab'
	| 'users_tab'
	| 'hosts_tab'
	| 'api_keys_tab'
	| 'shares_modal'
	| 'topology_tab';

export type PaywallGateType = 'limit_hit' | 'plan_required';

export interface TriggerUpgradeOptions {
	/** Feature context for recommended plan selection. Null/undefined = generic upgrade. */
	feature?: UpgradeFeature | null;
	/** Source identifier for analytics (e.g., 'sidebar', 'export_modal'). */
	source: string;
	/** UI surface where the gated control was clicked. */
	surface: PaywallSurface;
	/** Whether this is a usage-limit hit or a feature-not-on-plan gate. Defaults to 'plan_required'. */
	gate_type?: PaywallGateType;
	/** If true, reopens settings modal after billing modal closes. */
	reopenSettings?: boolean;
	/** Callback to run before opening the billing modal (e.g., close another modal). */
	beforeModal?: () => void;
}

/**
 * Single entry point for all upgrade actions.
 * Cloud: opens billing modal with feature context.
 * Self-hosted: opens pricing page in a new tab.
 */
export function triggerUpgrade(options: TriggerUpgradeOptions): void {
	const config = queryClient.getQueryData<PublicServerConfig>(queryKeys.config.all);
	const billingEnabled = config?.billing_enabled ?? false;

	trackEvent('paywall_gate_hit', {
		feature: options.feature ?? null,
		surface: options.surface,
		gate_type: options.gate_type ?? 'plan_required'
	});

	trackEvent('upgrade_button_clicked', {
		feature: options.feature ?? options.source,
		source: options.source,
		external: !billingEnabled
	});

	if (!billingEnabled) {
		options.beforeModal?.();
		window.open(pricingUrlFor(options.surface), '_blank', 'noopener,noreferrer');
		return;
	}

	// An air-gapped key carries the plan it was issued for and validates offline,
	// so the server refuses a plan change until the period paid for ends. Explain
	// it here rather than opening a picker in which every card 409s.
	//
	// This is the single entry point for every upgrade CTA in the app, so one
	// check covers all of them. `beforeModal` is deliberately not called: it
	// closes the caller's own modal, which would be wrong when nothing opens.
	const organization = queryClient.getQueryData<Organization>(queryKeys.organizations.current());
	if (organization?.air_gapped_key_current_until) {
		pushError(
			errors_billing_air_gapped_plan_change_blocked({
				date: formatDate(organization.air_gapped_key_current_until)
			})
		);
		return;
	}

	options.beforeModal?.();
	upgradeContext.set(options.feature ? { feature: options.feature } : null);

	if (options.reopenSettings) {
		reopenSettingsAfterBilling.set(true);
	}

	openModal('billing-plan');
}
