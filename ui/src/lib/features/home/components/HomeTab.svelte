<script lang="ts">
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import { useDashboardQuery } from '$lib/features/home/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import GettingStartedChecklist from './GettingStartedChecklist.svelte';
	import NetworkMetrics from './NetworkMetrics.svelte';
	import DaemonHealthPanel from './DaemonHealthPanel.svelte';
	import RecentDiscoveries from './RecentDiscoveries.svelte';
	import FeatureNudges from './FeatureNudges.svelte';
	import PlanUsage from './PlanUsage.svelte';
	import type { TabProps } from '$lib/shared/types';
	import type { components } from '$lib/api/schema';
	import { onMount } from 'svelte';

	type TelemetryOperation = components['schemas']['TelemetryOperation'];

	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	let { isReadOnly = false }: TabProps = $props();

	const dashboardQuery = useDashboardQuery();
	const organizationQuery = useOrganizationQuery();
	const currentUserQuery = useCurrentUserQuery();

	let dashboard = $derived(dashboardQuery.data);
	let organization = $derived(organizationQuery.data);
	let currentUser = $derived(currentUserQuery.data);

	let onboarding = $derived((organization?.onboarding ?? []) as TelemetryOperation[]);
	let isOwner = $derived(currentUser?.permissions === 'Owner');

	// Checklist dismiss state
	let checklistDismissed = $state(false);
	onMount(() => {
		checklistDismissed = localStorage.getItem('home-checklist-dismissed') === 'true';
	});

	// Journey stage derivation
	const has = (op: TelemetryOperation) => onboarding.includes(op);
	let hasDaemon = $derived(has('FirstDaemonRegistered'));
	let hasDiscovery = $derived(has('FirstDiscoveryCompleted'));
	let hasTopology = $derived(has('FirstTopologyRebuild'));
	let checklistComplete = $derived(hasDaemon && hasDiscovery && hasTopology);
	let showNudges = $derived(
		(checklistComplete || checklistDismissed) && dashboard != null && organization != null
	);

	// Navigation handler — sets the active tab via the URL hash
	function navigateTo(tab: string) {
		if (typeof window !== 'undefined') {
			window.location.hash = tab;
		}
	}
</script>

<div class="mx-auto max-w-5xl space-y-6">
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
		{#if !checklistComplete || !checklistDismissed}
			<GettingStartedChecklist {onboarding} onNavigate={navigateTo} />
		{/if}

		<!-- Network Metrics — shown once daemon is registered -->
		{#if hasDaemon && dashboard.networks.length > 0}
			<NetworkMetrics networks={dashboard.networks} planUsage={dashboard.plan_usage} />
		{/if}

		<!-- Daemon Health — shown once daemon is registered -->
		{#if hasDaemon && dashboard.daemons.length > 0}
			<DaemonHealthPanel daemons={dashboard.daemons} />
		{/if}

		<!-- Recent Discoveries — shown once first discovery is completed -->
		{#if hasDiscovery}
			<RecentDiscoveries discoveries={dashboard.recent_discoveries} />
		{/if}

		<!-- Feature Nudges — shown after checklist is complete -->
		{#if showNudges}
			<FeatureNudges {organization} {dashboard} onNavigate={navigateTo} />
		{/if}

		<!-- Plan Usage — always visible if limits are approaching -->
		<PlanUsage
			planUsage={dashboard.plan_usage}
			planType={organization.plan?.type ?? null}
			{isOwner}
		/>
	{/if}
</div>
