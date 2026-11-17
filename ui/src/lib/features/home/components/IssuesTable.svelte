<script lang="ts">
	import { ArrowRight } from '@lucide/svelte';
	import DataControls from '#lib/shared/components/data/DataControls.svelte';
	import Loading from '#lib/shared/components/feedback/Loading.svelte';
	import DaemonTroubleshootingModal from '#lib/shared/components/layout/DaemonTroubleshootingModal.svelte';
	import { listPageState } from '#lib/shared/components/data/listPage.svelte.ts';
	import type { RowAction, TableDefaults } from '#lib/shared/components/data/types.ts';
	import type { components } from '#lib/api/schema';
	import { setupItemTiers } from '#lib/shared/stores/metadata.ts';
	import { trackEvent } from '#lib/shared/utils/analytics.ts';
	import { useSitesQuery } from '#lib/features/sites/queries.ts';
	import { useDaemonsQuery } from '#lib/features/daemons/queries.ts';
	import { useIssuesListQuery } from '../queries';
	import { entryId, type SetupItemType, type SetupListEntry } from '../setup-list';
	import { setupRowView } from '../setup-row-view';
	import { setupLookups } from '../setup-lookups.svelte';
	import { downloadCsvFrom } from '#lib/shared/utils/csvExport.ts';
	import SetupItemSummary from './SetupItemSummary.svelte';
	import { issueColumns } from './IssuesTable.columns';

	type IssueOrderField = components['schemas']['IssueOrderField'];

	let {
		issueSites,
		isActive,
		setup
	}: {
		/** The sites the org's Issues are on. */
		issueSites: string[];
		/** Whether Home is on screen, which holds the Issues page until it is. */
		isActive: boolean;
		/** The org's done Setup item types. */
		setup: SetupItemType[];
	} = $props();

	const page = listPageState(() => isActive);
	const issuesQuery = useIssuesListQuery(() => page.params());
	let issues = $derived(issuesQuery.data?.items ?? []);
	let pagination = $derived(issuesQuery.data?.pagination ?? null);

	const sitesQuery = useSitesQuery();
	let sites = $derived(sitesQuery.data ?? []);
	const lookups = setupLookups(() => issues);

	const daemonsQuery = useDaemonsQuery();
	/** The daemon whose connection help is open, if any. */
	let troubleshootDaemonId = $state<string | null>(null);
	let troubleshootDaemon = $derived(
		(daemonsQuery.data ?? []).find((d) => d.id === troubleshootDaemonId)
	);

	/** What a row says and does. Issues ask no questions, so `onAnswer` is never called. */
	function viewOf(entry: SetupListEntry) {
		return setupRowView(entry.item, {
			setup,
			lookups,
			onAnswer: () => {},
			onTroubleshoot: (daemonId) => (troubleshootDaemonId = daemonId)
		});
	}

	/** One action per Issue: its fix, named on the row. Only the button runs it. */
	function issueActions(entry: SetupListEntry): RowAction[] {
		const fix = viewOf(entry).fix;
		return [
			{
				label: fix.label,
				icon: ArrowRight,
				showLabel: true,
				onClick: () => {
					trackEvent('setup_item_action_clicked', { item_type: entry.item.type });
					fix.run();
				}
			}
		];
	}

	const tableDefaults: TableDefaults<IssueOrderField> = { group: 'category' };

	let fields = $derived(
		issueColumns({ sites: () => sites, issueSites: () => issueSites, issueCell })
	);

	/** The Issues the table's filters and order select, every page. */
	async function handleCsvExport() {
		await downloadCsvFrom('dashboard/issues', page.listQuery);
	}
</script>

{#snippet issueCell(entry: SetupListEntry)}
	<SetupItemSummary view={viewOf(entry)} />
{/snippet}

{#if issuesQuery.isPending && !issuesQuery.data}
	<Loading />
{:else}
	<DataControls
		title={setupItemTiers.getName('Issue')}
		items={issues}
		{fields}
		storageKey="scanopy-home-issues-table-state"
		defaults={tableDefaults}
		getItemId={entryId}
		serverPagination={pagination}
		onPageChange={page.onPageChange}
		onQueryChange={page.onQueryChange}
		serverQuery={{ stale: false, tags: false, search: false }}
		getActions={issueActions}
		onCsvExport={handleCsvExport}
	/>
{/if}

<DaemonTroubleshootingModal
	daemon={troubleshootDaemon}
	isOpen={troubleshootDaemonId !== null}
	onClose={() => (troubleshootDaemonId = null)}
/>
