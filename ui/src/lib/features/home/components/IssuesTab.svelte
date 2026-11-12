<script lang="ts">
	import { untrack } from 'svelte';
	import Loading from '#lib/shared/components/feedback/Loading.svelte';
	import LoadFailedState from '#lib/shared/components/layout/LoadFailedState.svelte';
	import { loadFailure } from '#lib/shared/utils/load-failure.ts';
	import { useDashboardQuery } from '#lib/features/home/queries.ts';
	import { useOrganizationQuery } from '#lib/features/organizations/queries.ts';
	import type { TabProps } from '#lib/shared/types.ts';
	import IssuesTable from './IssuesTable.svelte';

	let { isActive = false }: TabProps = $props();

	const dashboardQuery = useDashboardQuery();
	const organizationQuery = useOrganizationQuery();

	let dashboard = $derived(dashboardQuery.data);
	let pageLoad = $derived(loadFailure(dashboardQuery, organizationQuery));

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

{#if pageLoad.failed}
	<LoadFailedState onRetry={pageLoad.retry} />
{:else if dashboardQuery.isPending || organizationQuery.isPending}
	<Loading />
{:else if dashboard}
	<IssuesTable
		issueSites={dashboard.issue_site_ids}
		{isActive}
		setup={organizationQuery.data?.setup ?? []}
	/>
{/if}
