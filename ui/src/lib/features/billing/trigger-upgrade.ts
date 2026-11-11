import { queryClient, queryKeys } from '#lib/api/query-client.ts';
import { isCloud, type PublicServerConfig } from '#lib/shared/stores/config-query.ts';
import { trackEvent } from '#lib/shared/utils/analytics.ts';
import { openModal } from '#lib/shared/stores/modal-registry.ts';
import {
	billingOpensOverModal,
	openBillingModal,
	reopenSettingsAfterBilling
} from '#lib/features/billing/stores.ts';
import type { UpgradeFeature } from '#lib/shared/stores/metadata.ts';
import type { PlanPickerHosting } from '#lib/features/billing/types.ts';

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
	| 'sites_tab'
	| 'users_tab'
	| 'hosts_tab'
	| 'api_keys_tab'
	| 'shares_modal'
	| 'topology_tab'
	| 'credential_form'
	| 'app_banner';

export type PaywallGateType = 'limit_hit' | 'plan_required';

/** What a control the plan lacks opens the plans for: its feature, where it is, and why. */
export interface UpgradeGate {
	feature: UpgradeFeature;
	surface: PaywallSurface;
	gate_type?: PaywallGateType;
}

export interface TriggerUpgradeOptions {
	/** Feature the upgrade CTA gates, for analytics. Null/undefined = generic upgrade. */
	feature?: UpgradeFeature | null;
	/** Source identifier for analytics (e.g., 'sidebar', 'export_modal'). */
	source: string;
	/** UI surface where the gated control was clicked. */
	surface: PaywallSurface;
	/** Whether this is a usage-limit hit or a feature-not-on-plan gate. Defaults to 'plan_required'. */
	gate_type?: PaywallGateType;
	/** If true, reopens settings modal after billing modal closes. */
	reopenSettings?: boolean;
	/**
	 * Callback to run before opening the billing modal (e.g., close another modal). Skipped when
	 * the plan picker opens over the open modal instead of replacing it.
	 */
	beforeModal?: () => void;
	/** Hosting tab the plan picker opens on, overriding the org's current plan's. */
	hosting?: PlanPickerHosting;
}

/**
 * Single entry point for all upgrade actions.
 * Cloud: opens the billing modal.
 * Self-hosted: opens pricing page in a new tab.
 */
export function triggerUpgrade(options: TriggerUpgradeOptions): void {
	const config = queryClient.getQueryData<PublicServerConfig>(queryKeys.config.all);
	const cloud = config ? isCloud(config) : false;

	trackEvent('paywall_gate_hit', {
		feature: options.feature ?? null,
		surface: options.surface,
		gate_type: options.gate_type ?? 'plan_required'
	});

	trackEvent('upgrade_button_clicked', {
		feature: options.feature ?? options.source,
		source: options.source,
		external: !cloud
	});

	if (!cloud) {
		options.beforeModal?.();
		window.open(pricingUrlFor(options.surface), '_blank', 'noopener,noreferrer');
		return;
	}

	const entityData = options.hosting ? { hosting: options.hosting } : undefined;

	// Over a form, the plan picker stacks on it and the form keeps what was entered.
	if (billingOpensOverModal()) {
		openBillingModal('billing-plan', entityData);
		return;
	}

	options.beforeModal?.();

	if (options.reopenSettings) {
		reopenSettingsAfterBilling.set(true);
	}

	openModal('billing-plan', entityData ? { entityData } : undefined);
}
