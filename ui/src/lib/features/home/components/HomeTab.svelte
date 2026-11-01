<script lang="ts">
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import LoadFailedState from '$lib/shared/components/layout/LoadFailedState.svelte';
	import { loadFailure } from '$lib/shared/utils/load-failure';
	import PageTitle from '$lib/shared/components/layout/PageTitle.svelte';
	import { common_home } from '$lib/paraglide/messages';
	import { useDashboardQuery } from '$lib/features/home/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { isPlanLapsed } from '$lib/features/organizations/types';
	import { useCan } from '$lib/features/auth/access';
	import { useActiveSessionsQuery } from '$lib/features/discovery/queries';
	import GettingStartedChecklist from './GettingStartedChecklist.svelte';
	import ActiveDiscoveries from './ActiveDiscoveries.svelte';
	import SiteMetrics from './SiteMetrics.svelte';
	import DaemonHealthPanel from './DaemonHealthPanel.svelte';
	import RecentDiscoveries from './RecentDiscoveries.svelte';
	import FeatureNudges from './FeatureNudges.svelte';
	import PlanUsage from './PlanUsage.svelte';
	import ProfilePrompt from './ProfilePrompt.svelte';
	import ReferralSourcePrompt from './ReferralSourcePrompt.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import { homeDaemonPrompt } from '$lib/shared/onboarding/checklist';
	import type { TabProps } from '$lib/shared/types';
	import type { components } from '$lib/api/schema';
	import { onMount, untrack } from 'svelte';
	import { openModal } from '$lib/shared/stores/modal-registry';
	import { useConfigQuery, isCloud } from '$lib/shared/stores/config-query';
	import {
		home_demoEmbedTitle,
		home_demoEmbedSubtitle,
		home_demoNetworkTopology,
		gettingStarted_stepDaemonDescription
	} from '$lib/paraglide/messages';
	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];

	let { isActive = false }: TabProps = $props();

	const can = useCan();
	const dashboardQuery = useDashboardQuery();
	const organizationQuery = useOrganizationQuery();
	const configQuery = useConfigQuery();
	const sessionsQuery = useActiveSessionsQuery();

	let dashboard = $derived(dashboardQuery.data);
	let organization = $derived(organizationQuery.data);
	let activeSessions = $derived(sessionsQuery.data ?? []);
	let pageLoad = $derived(loadFailure(dashboardQuery, organizationQuery));

	let onboarding = $derived((organization?.onboarding ?? []) as OnboardingOperation[]);
	let isOwner = $derived(can('Billing', 'Write'));
	let configData = $derived(configQuery.data);

	// Checklist dismiss state
	let checklistDismissed = $state(false);
	let demoTopologyDismissed = $state(false);
	onMount(() => {
		checklistDismissed = localStorage.getItem('home-checklist-dismissed') === 'true';
		demoTopologyDismissed = localStorage.getItem('home-demo-topology-dismissed') === 'true';
	});

	function dismissDemoTopology() {
		demoTopologyDismissed = true;
		localStorage.setItem('home-demo-topology-dismissed', 'true');
	}

	// Refetch when the tab opens. Untracked: reading `refetch` off a query subscribes to every
	// change of its result, and each refetch changes it, so the effect would refetch in a loop.
	$effect(() => {
		if (isActive) {
			untrack(() => {
				organizationQuery.refetch();
				dashboardQuery.refetch();
			});
		}
	});

	// Journey stage derivation
	const has = (op: OnboardingOperation) => onboarding.includes(op);
	let showNudges = $derived(dashboard != null && organization != null);

	// One prompt at a time: referral source first, then profile
	let showReferralSource = $derived(
		configData &&
			configData.deployment_type === 'cloud' &&
			has('FirstDaemonRegistered') &&
			!has('ReferralSourceCompleted')
	);
	let showProfile = $derived(!showReferralSource);

	// Fills the checklist's slot once it is dismissed before the first daemon
	let showDaemonPrompt = $derived(homeDaemonPrompt(onboarding, checklistDismissed));

	// Navigation handler — sets the active tab via the URL hash
	function navigateTo(tab: string) {
		if (typeof window !== 'undefined') {
			window.location.hash = tab;
		}
	}
</script>

<div class="space-y-6">
	<!-- Same title row as every list page; the organization's name follows the title, muted,
	     rather than sitting on a line of its own. -->
	<PageTitle title={common_home()} aside={organization?.name ?? null} />

	{#if pageLoad.failed}
		<LoadFailedState onRetry={pageLoad.retry} />
	{:else if dashboardQuery.isPending || organizationQuery.isPending}
		<Loading />
	{:else if dashboard && organization}
		<!-- Getting Started Checklist -->
		{#if !checklistDismissed}
			<GettingStartedChecklist
				{onboarding}
				{organization}
				onNavigate={navigateTo}
				onDismiss={() => (checklistDismissed = true)}
				{isActive}
			/>
		{:else if showDaemonPrompt}
			<section class="card-static card">
				<PreDaemonEmptyState title={gettingStarted_stepDaemonDescription()} />
			</section>
		{/if}

		<!-- Onboarding prompts — one at a time -->
		{#if showReferralSource}
			<ReferralSourcePrompt {organization} {configData} />
		{:else if showProfile}
			<ProfilePrompt {organization} {configData} />
		{/if}

		<!-- Demo Topology Embed — shown before first topology rebuild -->
		{#if configData && isCloud(configData) && !has('FirstDiscoveryCompleted') && !demoTopologyDismissed}
			<section>
				<div class="card-static card overflow-hidden p-0!">
					<div class="flex items-center justify-between px-4 pt-3">
						<h3 class="text-base font-semibold text-primary">{home_demoEmbedTitle()}</h3>
						<div class="flex items-center gap-3">
							<a
								href="https://demo.scanopy.net/share/a1b2c3d4-e5f6-7890-abcd-ef1234567890?utm_source=app&utm_medium=in_app&utm_campaign=demo_embed&utm_content=home_link"
								target="_blank"
								rel="noopener noreferrer"
								class="text-sm text-link hover:underline"
							>
								View full screen
							</a>
							<button
								onclick={dismissDemoTopology}
								class="text-sm text-tertiary hover:text-secondary"
							>
								Dismiss
							</button>
						</div>
					</div>
					<p class="px-4 pb-2 text-sm text-secondary">
						{home_demoEmbedSubtitle()}
					</p>
					<div class="h-[400px] w-full">
						<iframe
							src="https://demo.scanopy.net/share/a1b2c3d4-e5f6-7890-abcd-ef1234567890/embed?utm_source=app&utm_medium=in_app&utm_campaign=demo_embed&utm_content=home_embed"
							width="100%"
							height="100%"
							frameborder="0"
							style="border: none;"
							title={home_demoNetworkTopology()}
						></iframe>
					</div>
				</div>
			</section>
		{/if}

		<!-- Active Discoveries — shown when sessions exist, after first discovery completed -->
		{#if has('FirstDiscoveryCompleted') && activeSessions.length > 0}
			<ActiveDiscoveries
				sessions={activeSessions}
				onNavigate={() => navigateTo('discovery-scans')}
			/>
		{/if}

		<!-- Feature Nudges — shown after checklist is complete/dismissed. A lapsed
		     org keeps its plan's feature flags but cannot act on any of them. -->
		{#if showNudges && !isPlanLapsed(organization)}
			<FeatureNudges {organization} {dashboard} onNavigate={navigateTo} />
		{/if}

		<!-- Daemon Health — shown when daemons exist -->
		{#if dashboard.daemons.length > 0}
			<DaemonHealthPanel daemons={dashboard.daemons} onNavigate={navigateTo} />
		{/if}

		<!-- Recent scans — shown once a daemon exists, so before that the checklist is the only ask -->
		{#if dashboard.recent_discoveries.length > 0 || has('FirstDaemonRegistered')}
			<RecentDiscoveries
				recent={dashboard.recent_discoveries}
				daemons={dashboard.daemons}
				sites={dashboard.sites}
				onOpenRun={(discovery) => {
					// The run itself, so the modal opens even when the history tab has not loaded it.
					openModal('discovery-history-detail', { id: discovery.id, entityData: discovery });
					navigateTo('discovery-history');
				}}
			/>
		{/if}

		<!-- Plan Usage — hidden pre-daemon since limits can't be hit -->
		{#if has('FirstDaemonRegistered')}
			<PlanUsage planUsage={dashboard.plan_usage} plan={organization.plan} {isOwner} />
		{/if}

		<!-- Site Metrics — hidden pre-daemon since no meaningful data yet -->
		{#if has('FirstDaemonRegistered') && dashboard.sites.length > 0}
			<SiteMetrics sites={dashboard.sites} />
		{/if}
	{/if}
</div>
