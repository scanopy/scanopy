<script lang="ts">
	import { Lock } from '@lucide/svelte';
	import AppBanner from './AppBanner.svelte';
	import { useCan } from '#lib/features/auth/access.ts';
	import { useOrganizationQuery } from '#lib/features/organizations/queries.ts';
	import { hasLicensedPlan, isPlanLapsed } from '#lib/features/organizations/types.ts';
	import { useWrittenOffInvoice } from '#lib/features/billing/queries.ts';
	import { billingPlans } from '#lib/shared/stores/metadata.ts';
	import { isCloud, useConfigQuery } from '#lib/shared/stores/config-query.ts';
	import { triggerUpgrade } from '#lib/features/billing/trigger-upgrade.ts';
	import {
		billing_lapsedBannerAskOwner,
		billing_lapsedBannerBody,
		billing_lapsedBannerCta,
		settings_billing_license_lapsed,
		settings_billing_license_lapsedUnpaid,
		settings_billing_payInvoice
	} from '#lib/paraglide/messages.js';

	// A lapsed org (subscription ended, no paid plan chosen since) keeps its
	// plan and browses read-only. The backend refuses its writes; this banner
	// is the standing explanation and, for owners, the way back. It never
	// dismisses: the state it describes only ends with a plan choice.
	//
	// It is the one place the lapse is explained. A licensed org reaches only
	// Settings, where this renders in the frame above the License tab, so the
	// licensed wording is about the key.
	const can = useCan();
	const organizationQuery = useOrganizationQuery();
	const configQuery = useConfigQuery();
	const writtenOffInvoice = useWrittenOffInvoice();

	let org = $derived(organizationQuery.data);
	let isCloudDeployment = $derived(configQuery.data ? isCloud(configQuery.data) : false);
	let isOwner = $derived(can('Billing', 'Write'));
	let shouldShow = $derived(isCloudDeployment && org != null && isPlanLapsed(org));
	let plan = $derived(billingPlans.getName(org?.plan?.type ?? null));
	// A write-off takes the paid-through date back to the start of the period
	// it covered, so a date already past means the invoice went unpaid rather
	// than that the plan simply ended.
	let unpaid = $derived(
		org?.license_paid_through != null && Date.parse(org.license_paid_through) <= Date.now()
	);
	let message = $derived.by(() => {
		if (org == null || !hasLicensedPlan(org)) return billing_lapsedBannerBody({ plan });
		return unpaid
			? settings_billing_license_lapsedUnpaid({ plan })
			: settings_billing_license_lapsed({ plan });
	});
	let body = $derived(isOwner ? message : `${message} ${billing_lapsedBannerAskOwner()}`);
	// Settling the written-off invoice brings the plan back, so it outranks choosing a new one.
	let invoiceUrl = $derived(writtenOffInvoice.current?.hosted_invoice_url ?? null);
</script>

{#if shouldShow}
	<AppBanner variant="danger" icon={Lock} {body}>
		{#snippet actions()}
			{#if isOwner && invoiceUrl}
				<!-- eslint-disable svelte/no-navigation-without-resolve -->
				<a
					href={invoiceUrl}
					target="_blank"
					rel="external noopener noreferrer"
					class="ml-2 rounded-sm px-2 py-0.5 text-xs font-medium underline hover:no-underline"
				>
					{settings_billing_payInvoice()}
				</a>
				<!-- eslint-enable svelte/no-navigation-without-resolve -->
			{:else if isOwner}
				<button
					onclick={() => triggerUpgrade({ source: 'plan_lapsed_banner', surface: 'app_banner' })}
					class="ml-2 rounded-sm px-2 py-0.5 text-xs font-medium underline hover:no-underline"
				>
					{billing_lapsedBannerCta()}
				</button>
			{/if}
		{/snippet}
	</AppBanner>
{/if}
