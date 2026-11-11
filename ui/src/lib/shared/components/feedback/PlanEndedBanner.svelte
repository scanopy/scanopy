<script lang="ts">
	import { Info } from '@lucide/svelte';
	import AppBanner from './AppBanner.svelte';
	import { useCan } from '#lib/features/auth/access.ts';
	import { useOrganizationQuery } from '#lib/features/organizations/queries.ts';
	import { billingPlans } from '#lib/shared/stores/metadata.ts';
	import { isCloud, useConfigQuery } from '#lib/shared/stores/config-query.ts';
	import { triggerUpgrade } from '#lib/features/billing/trigger-upgrade.ts';
	import {
		billing_lapsedBannerAskOwner,
		billing_lapsedBannerCta,
		billing_planEndedBannerBody
	} from '#lib/paraglide/messages.js';

	// A cloud plan that ends lands the org on Free. Says which plan ended, once
	// per downgrade: dismissing it hides it until the next one, because the key
	// carries the downgrade's time.
	const can = useCan();
	const organizationQuery = useOrganizationQuery();
	const configQuery = useConfigQuery();

	let org = $derived(organizationQuery.data);
	let isCloudDeployment = $derived(configQuery.data ? isCloud(configQuery.data) : false);
	let isOwner = $derived(can('Billing', 'Write'));
	let endedPlan = $derived(org?.last_downgrade_from_plan?.type ?? null);
	let shouldShow = $derived(
		isCloudDeployment &&
			org != null &&
			billingPlans.getMetadata(org.plan?.type ?? null).is_free === true &&
			endedPlan != null &&
			org.last_downgrade_at != null
	);
	let body = $derived(
		`${billing_planEndedBannerBody({ plan: billingPlans.getName(endedPlan) })}${
			isOwner ? '' : ` ${billing_lapsedBannerAskOwner()}`
		}`
	);
</script>

{#if shouldShow}
	<AppBanner
		variant="info"
		icon={Info}
		{body}
		dismissableKey={`plan-ended:${org?.id}:${org?.last_downgrade_at}`}
	>
		{#snippet actions()}
			{#if isOwner}
				<button
					onclick={() => triggerUpgrade({ source: 'plan_ended_banner', surface: 'app_banner' })}
					class="ml-2 rounded-sm px-2 py-0.5 text-xs font-medium underline hover:no-underline"
				>
					{billing_lapsedBannerCta()}
				</button>
			{/if}
		{/snippet}
	</AppBanner>
{/if}
