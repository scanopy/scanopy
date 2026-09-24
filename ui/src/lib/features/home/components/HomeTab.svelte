<script lang="ts">
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import { useDashboardQuery } from '$lib/features/home/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { isPlanLapsed } from '$lib/features/organizations/types';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useActiveSessionsQuery } from '$lib/features/discovery/queries';
	import GettingStartedChecklist from './GettingStartedChecklist.svelte';
	import ActiveDiscoveries from './ActiveDiscoveries.svelte';
	import NetworkMetrics from './NetworkMetrics.svelte';
	import DaemonHealthPanel from './DaemonHealthPanel.svelte';
	import RecentDiscoveries from './RecentDiscoveries.svelte';
	import FeatureNudges from './FeatureNudges.svelte';
	import PlanUsage from './PlanUsage.svelte';
	import ProfilePrompt from './ProfilePrompt.svelte';
	import ReferralSourcePrompt from './ReferralSourcePrompt.svelte';
	import type { TabProps } from '$lib/shared/types';
	import type { components } from '$lib/api/schema';
	import { onMount } from 'svelte';
	import { openModal } from '$lib/shared/stores/modal-registry';
	import { useConfigQuery, isCloud } from '$lib/shared/stores/config-query';
	import {
		home_demoEmbedTitle,
		home_demoEmbedSubtitle,
		home_demoNetworkTopology
	} from '$lib/paraglide/messages';
	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];

	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	let { isReadOnly = false, isActive = false }: TabProps = $props();

	const dashboardQuery = useDashboardQuery();
	const organizationQuery = useOrganizationQuery();
	const currentUserQuery = useCurrentUserQuery();
	const configQuery = useConfigQuery();
	const sessionsQuery = useActiveSessionsQuery();

	let dashboard = $derived(dashboardQuery.data);
	let organization = $derived(organizationQuery.data);
	let currentUser = $derived(currentUserQuery.data);
	let activeSessions = $derived(sessionsQuery.data ?? []);

	let onboarding = $derived((organization?.onboarding ?? []) as OnboardingOperation[]);
	let isOwner = $derived(currentUser?.permissions === 'Owner');
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

	$effect(() => {
		if (isActive) {
			organizationQuery.refetch();
			dashboardQuery.refetch();
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

	// Navigation handler — sets the active tab via the URL hash
	function navigateTo(tab: string) {
		if (typeof window !== 'undefined') {
			window.location.hash = tab;
		}
	}
</script>

<div class="space-y-6">
	<div>
		<h1 class="text-primary text-2xl font-bold">Home</h1>
		<p class="text-tertiary mt-1 text-sm">
			{#if organization}
				{organization.name}
			{/if}
		</p>
	</div>

	{#if dashboardQuery.isPending || organizationQuery.isPending}
		<Loading />
	{:else if dashboard && organization}
		<!-- Getting Started Checklist -->
		{#if !checklistDismissed}
			<GettingStartedChecklist {onboarding} {organization} onNavigate={navigateTo} {isActive} />
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
				<div class="card card-static overflow-hidden !p-0">
					<div class="flex items-center justify-between px-4 pt-3">
						<h3 class="text-primary text-base font-semibold">{home_demoEmbedTitle()}</h3>
						<div class="flex items-center gap-3">
							<a
								href="https://demo.scanopy.net/share/a1b2c3d4-e5f6-7890-abcd-ef1234567890?utm_source=app&utm_medium=in_app&utm_campaign=demo_embed&utm_content=home_link"
								target="_blank"
								rel="noopener noreferrer"
								class="text-link text-sm hover:underline"
							>
								View full screen
							</a>
							<button
								onclick={dismissDemoTopology}
								class="text-tertiary hover:text-secondary text-sm"
							>
								Dismiss
							</button>
						</div>
					</div>
					<p class="text-secondary px-4 pb-2 text-sm">
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

		<!-- Recent Discoveries — shown when discoveries exist -->
		{#if dashboard.recent_discoveries.length > 0}
			<RecentDiscoveries
				discoveries={dashboard.recent_discoveries}
				daemons={dashboard.daemons}
				networks={dashboard.networks}
				onNavigate={(discovery) => {
					openModal('discovery-history-detail', { id: discovery.id });
					navigateTo('discovery-history');
				}}
			/>
		{/if}

		<!-- Plan Usage — hidden pre-daemon since limits can't be hit -->
		{#if has('FirstDaemonRegistered')}
			<PlanUsage planUsage={dashboard.plan_usage} plan={organization.plan} {isOwner} />
		{/if}

		<!-- Network Metrics — hidden pre-daemon since no meaningful data yet -->
		{#if has('FirstDaemonRegistered') && dashboard.networks.length > 0}
			<NetworkMetrics networks={dashboard.networks} />
		{/if}
	{/if}
</div>
