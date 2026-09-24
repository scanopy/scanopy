<script lang="ts">
	// The app's banner stack, in one place.
	//
	// These render in the main app when it is reachable, and inside the Settings
	// modal frame when the app is gated behind it. The caller decides *where*;
	// which banners show is decided only here, so the two surfaces cannot drift.
	// Exactly one instance should be mounted at a time: AppBanner keeps its
	// dismissed state locally, so a second copy would not follow a dismissal.
	//
	// No props. Every condition reads one of the three global query hooks, which
	// work anywhere under the provider in +layout.svelte.
	import EmailVerificationBanner from './EmailVerificationBanner.svelte';
	import TrialEndingBanner from './TrialEndingBanner.svelte';
	import NoPaymentMethodBanner from './NoPaymentMethodBanner.svelte';
	import PostStripeWelcomeBanner from './PostStripeWelcomeBanner.svelte';
	import PlanLapsedBanner from './PlanLapsedBanner.svelte';
	import DemoBanner from './DemoBanner.svelte';
	import LicenseLockedBanner from './LicenseLockedBanner.svelte';
	import LicensePendingBanner from './LicensePendingBanner.svelte';
	import LicenseGraceBanner from './LicenseGraceBanner.svelte';
	import LicenseExpiringBanner from './LicenseExpiringBanner.svelte';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useConfigQuery, isLicenseApproachingExpiry } from '$lib/shared/stores/config-query';

	const currentUserQuery = useCurrentUserQuery();
	const organizationQuery = useOrganizationQuery();
	const configQuery = useConfigQuery();

	let organization = $derived(organizationQuery.data);
</script>

{#if currentUserQuery.data && !currentUserQuery.data.email_verified}
	<EmailVerificationBanner email={currentUserQuery.data.email} />
{/if}
<!-- These four gate themselves on org and trial state. The lapsed banner
     cannot overlap the first two: they need a trialing or active plan. -->
<TrialEndingBanner />
<NoPaymentMethodBanner />
<PostStripeWelcomeBanner />
<PlanLapsedBanner />
{#if organization?.plan?.type === 'Demo'}
	<DemoBanner />
{/if}
<!-- One chain, mutually exclusive by construction: a licence that is locked must
     not also announce a grace period or an approaching expiry. -->
{#if configQuery.data?.license_status === 'expired' || configQuery.data?.license_status === 'invalid'}
	<LicenseLockedBanner status={configQuery.data.license_status} />
{:else if configQuery.data?.license_status === 'pending'}
	<LicensePendingBanner />
{:else if configQuery.data?.license_in_grace_period && configQuery.data?.license_valid_through && configQuery.data?.license_expiry}
	<LicenseGraceBanner
		validThrough={configQuery.data.license_valid_through}
		hardExpiry={configQuery.data.license_expiry}
	/>
{:else if configQuery.data && isLicenseApproachingExpiry(configQuery.data) && configQuery.data.license_valid_through && configQuery.data.license_expiry}
	<LicenseExpiringBanner
		validThrough={configQuery.data.license_valid_through}
		hardExpiry={configQuery.data.license_expiry}
	/>
{/if}
