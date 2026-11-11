<script lang="ts">
	// The app's banner stack, in one place.
	//
	// These render in the main app when it is reachable, and inside the Settings
	// modal frame when the app is gated behind it. The caller decides *where*;
	// which banners show is decided only here, so the two surfaces cannot drift.
	// Exactly one instance should be mounted at a time: AppBanner keeps its
	// dismissed state locally, so a second copy would not follow a dismissal.
	//
	// No props. Every condition reads one of the global query hooks, which
	// work anywhere under the provider in +layout.svelte.
	import EmailVerificationBanner from './EmailVerificationBanner.svelte';
	import TrialEndingBanner from './TrialEndingBanner.svelte';
	import NoPaymentMethodBanner from './NoPaymentMethodBanner.svelte';
	import PostStripeWelcomeBanner from './PostStripeWelcomeBanner.svelte';
	import PlanLapsedBanner from './PlanLapsedBanner.svelte';
	import PlanEndedBanner from './PlanEndedBanner.svelte';
	import DemoBanner from './DemoBanner.svelte';
	import LicenseLockedBanner from './LicenseLockedBanner.svelte';
	import LicensePendingBanner from './LicensePendingBanner.svelte';
	import LicenseGraceBanner from './LicenseGraceBanner.svelte';
	import LicenseExpiringBanner from './LicenseExpiringBanner.svelte';
	import LicenseInstanceLimitBanner from './LicenseInstanceLimitBanner.svelte';
	import NonProductionBanner from './NonProductionBanner.svelte';
	import UpgradeAvailableBanner from './UpgradeAvailableBanner.svelte';
	import { useEmailVerificationPending } from '#lib/features/auth/queries.ts';
	import { useCan } from '#lib/features/auth/access.ts';
	import { useOrganizationQuery } from '#lib/features/organizations/queries.ts';
	import { useConfigQuery, isLicenseApproachingExpiry } from '#lib/shared/stores/config-query.ts';

	const emailVerification = useEmailVerificationPending();
	const organizationQuery = useOrganizationQuery();
	const configQuery = useConfigQuery();
	const can = useCan();

	let organization = $derived(organizationQuery.data);
	// Whoever runs the organization's users runs the upgrade; nobody else can act on the banner.
	let canUpgrade = $derived(can('Member', 'Read'));
	// The production or non-production count that is over its limit. A null
	// limit is a plan with no cap.
	let instanceOverrun = $derived.by(() => {
		const usage = configQuery.data?.license_instances;
		if (!usage) return null;
		if (usage.limit != null && usage.active > usage.limit) {
			return { active: usage.active, limit: usage.limit };
		}
		const nonProductionLimit = usage.non_production_limit;
		if (nonProductionLimit != null && usage.non_production_active > nonProductionLimit) {
			return { active: usage.non_production_active, limit: nonProductionLimit };
		}
		return null;
	});
</script>

{#if emailVerification.pending && emailVerification.email}
	<EmailVerificationBanner email={emailVerification.email} />
{/if}
<!-- These four gate themselves on org and trial state. The lapsed banner
     cannot overlap the first two: they need a trialing or active plan. -->
<TrialEndingBanner />
<NoPaymentMethodBanner />
<PostStripeWelcomeBanner />
<PlanLapsedBanner />
<PlanEndedBanner />
{#if organization?.plan?.type === 'Demo'}
	<DemoBanner />
{/if}
<!-- One chain, mutually exclusive by construction: a licence that is locked must
     not also announce a grace period or an approaching expiry. -->
{#if configQuery.data?.license_status === 'expired' || configQuery.data?.license_status === 'invalid'}
	<LicenseLockedBanner
		status={configQuery.data.license_status}
		refusedFor={instanceOverrun}
		problem={configQuery.data.license_problem}
	/>
{:else if configQuery.data?.license_status === 'pending'}
	<LicensePendingBanner problem={configQuery.data.license_problem} />
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
<!-- Outside the chain above: a server that holds a slot on a key running past
     its limit keeps working and warns. The refused server is read-only and
     says so in the locked banner instead. -->
{#if instanceOverrun && configQuery.data?.license_status !== 'invalid'}
	<LicenseInstanceLimitBanner active={instanceOverrun.active} limit={instanceOverrun.limit} />
{/if}
{#if configQuery.data?.non_production}
	<NonProductionBanner />
{/if}
<!-- Self-hosted only: the server leaves available_upgrade unset on the cloud. -->
{#if canUpgrade && configQuery.data?.available_upgrade}
	<UpgradeAvailableBanner upgrade={configQuery.data.available_upgrade} />
{/if}
