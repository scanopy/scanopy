<script lang="ts">
	import { Server } from '@lucide/svelte';
	import EmptyState from '#lib/shared/components/layout/EmptyState.svelte';
	import { useOrganizationQuery } from '#lib/features/organizations/queries.ts';
	import { triggerUpgrade } from '#lib/features/billing/trigger-upgrade.ts';
	import { openModal } from '#lib/shared/stores/modal-registry.ts';
	import { formatRelativeTime } from '#lib/shared/utils/formatting.ts';
	import type { IconComponent } from '#lib/shared/utils/types.ts';
	import {
		settings_billing_license_pageOnServer,
		settings_billing_license_pageOnServerCheckedIn,
		settings_billing_license_pageOnServerSetUp,
		settings_billing_license_setUpServer,
		settings_billing_license_useCloudInstead
	} from '#lib/paraglide/messages.js';

	/**
	 * What a main-app page shows to an org on a licensed self-hosted plan. The app runs on the
	 * org's own server, so the page points there: to the license key and install steps until the
	 * server has checked in, then to the server itself. Moving to a cloud plan is the secondary
	 * link; the plan picker says what that does to the key.
	 */
	let { page }: { page: string } = $props();

	const organizationQuery = useOrganizationQuery();
	let checkedInAt = $derived(organizationQuery.data?.license_checkin_at ?? null);

	function openLicenseTab() {
		openModal('settings', { tab: 'license' });
	}

	function useCloud() {
		triggerUpgrade({
			source: 'switch_to_cloud_empty_state',
			surface: 'sidebar',
			hosting: 'cloud'
		});
	}
</script>

<EmptyState
	IconComponent={Server as IconComponent}
	title={settings_billing_license_pageOnServer({ page })}
	subtitle={checkedInAt
		? settings_billing_license_pageOnServerCheckedIn({
				time: formatRelativeTime(checkedInAt),
				page
			})
		: settings_billing_license_pageOnServerSetUp()}
>
	<div class="flex flex-col items-center gap-3">
		{#if !checkedInAt}
			<button type="button" class="btn-primary" onclick={openLicenseTab}>
				{settings_billing_license_setUpServer()}
			</button>
		{/if}
		<button type="button" class="text-sm text-link hover:underline" onclick={useCloud}>
			{settings_billing_license_useCloudInstead()}
		</button>
	</div>
</EmptyState>
