<script lang="ts">
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import LoadFailedState from '$lib/shared/components/layout/LoadFailedState.svelte';
	import { loadFailure } from '$lib/shared/utils/load-failure';
	import PageTitle from '$lib/shared/components/layout/PageTitle.svelte';
	import { common_dismiss, common_home } from '$lib/paraglide/messages';
	import { useDashboardQuery } from '$lib/features/home/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import SetupList from './SetupList.svelte';
	import { isPlanLapsed } from '$lib/features/organizations/types';
	import SiteMetrics from './SiteMetrics.svelte';
	import type { TabProps } from '$lib/shared/types';
	import { onMount, untrack } from 'svelte';
	import { useConfigQuery, isCloud } from '$lib/shared/stores/config-query';
	import {
		home_demoEmbedTitle,
		home_demoEmbedSubtitle,
		home_demoNetworkTopology
	} from '$lib/paraglide/messages';

	let { isActive = false }: TabProps = $props();

	const dashboardQuery = useDashboardQuery();
	const organizationQuery = useOrganizationQuery();
	const configQuery = useConfigQuery();

	let dashboard = $derived(dashboardQuery.data);
	let organization = $derived(organizationQuery.data);
	let pageLoad = $derived(loadFailure(dashboardQuery, organizationQuery));

	let setup = $derived(organization?.setup ?? []);
	// A lapsed org keeps its plan's feature flags but can act on none of them, so it gets only its
	// Issues.
	let setupItems = $derived(
		organization && isPlanLapsed(organization) ? [] : (dashboard?.setup_items ?? [])
	);
	let configData = $derived(configQuery.data);

	let demoTopologyDismissed = $state(false);
	onMount(() => {
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
		{#if setupItems.length > 0 || dashboard.issue_count > 0}
			<SetupList
				entries={setupItems}
				issueCount={dashboard.issue_count}
				issueSites={dashboard.issue_site_ids}
				{isActive}
				{setup}
			/>
		{/if}

		<!-- Demo Topology Embed — shown until the first scan finishes -->
		{#if configData && isCloud(configData) && !setup.includes('RunFirstScan') && !demoTopologyDismissed}
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
								{common_dismiss()}
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

		<!-- Site Metrics — hidden pre-daemon since no meaningful data yet -->
		{#if setup.includes('InstallDaemon') && dashboard.sites.length > 0}
			<SiteMetrics sites={dashboard.sites} />
		{/if}
	{/if}
</div>
