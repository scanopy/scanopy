<script lang="ts">
	import { Lock } from 'lucide-svelte';
	import AppBanner from './AppBanner.svelte';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { isPlanLapsed } from '$lib/features/organizations/types';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import { openModal } from '$lib/shared/stores/modal-registry';
	import {
		billing_lapsedBannerAskOwner,
		billing_lapsedBannerBody,
		billing_lapsedBannerCta
	} from '$lib/paraglide/messages';

	// A lapsed org (subscription ended, no paid plan chosen since) keeps its
	// plan and browses read-only. The backend refuses its writes; this banner
	// is the standing explanation and, for owners, the way back. It never
	// dismisses: the state it describes only ends with a plan choice.
	const currentUserQuery = useCurrentUserQuery();
	const organizationQuery = useOrganizationQuery();
	const configQuery = useConfigQuery();

	let org = $derived(organizationQuery.data);
	let billingEnabled = $derived(configQuery.data?.billing_enabled ?? false);
	let isOwner = $derived(currentUserQuery.data?.permissions === 'Owner');
	let shouldShow = $derived(billingEnabled && org != null && isPlanLapsed(org));
	let body = $derived(
		`${billing_lapsedBannerBody({ plan: billingPlans.getName(org?.plan?.type ?? null) })}${
			isOwner ? '' : ` ${billing_lapsedBannerAskOwner()}`
		}`
	);
</script>

{#if shouldShow}
	<AppBanner variant="warning" icon={Lock} {body}>
		{#snippet actions()}
			{#if isOwner}
				<button
					onclick={() => openModal('billing-plan')}
					class="ml-2 rounded px-2 py-0.5 text-xs font-medium underline hover:no-underline"
				>
					{billing_lapsedBannerCta()}
				</button>
			{/if}
		{/snippet}
	</AppBanner>
{/if}
