<script lang="ts">
	import { collectionLookup } from '$lib/shared/entity-fields';
	import CreateButton from '$lib/shared/components/data/controls/CreateButton.svelte';
	import { siteColumns } from './SitesTab.columns';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import LoadFailedState from '$lib/shared/components/layout/LoadFailedState.svelte';
	import { loadFailure } from '$lib/shared/utils/load-failure';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import type { Site } from '../types';
	import SiteEditModal from './SiteEditModal.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { TableDefaults } from '$lib/shared/components/data/types';
	import type { RowAction } from '$lib/shared/components/data/types';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { Trash2, Edit } from '@lucide/svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import type { TabProps } from '$lib/shared/types';
	import {
		common_confirmBulkDelete,
		common_create,
		common_delete,
		common_edit,
		common_sites,
		common_noEntityYet,
		sites_confirmDelete
	} from '$lib/paraglide/messages';

	import { refusedUnless, useCan } from '$lib/features/auth/access';

	let { isActive = false }: TabProps = $props();
	const can = useCan();
	import {
		useSitesQuery,
		useSitesListQuery,
		useCreateSiteMutation,
		useUpdateSiteMutation,
		useDeleteSiteMutation,
		useBulkDeleteSitesMutation
	} from '../queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { isUserManagedSubnet, useSubnetsQuery } from '$lib/features/subnets/queries';
	import { useVlansQuery } from '$lib/features/vlans/queries';
	import { useDependenciesQuery } from '$lib/features/dependencies/queries';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { listPageState } from '$lib/shared/components/data/listPage.svelte';
	import { useFieldValuesQuery } from '$lib/shared/api/field-values';
	import type { components } from '$lib/api/schema';

	type SiteOrderField = components['schemas']['SiteOrderField'];
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';

	// Queries

	const organizationQuery = useOrganizationQuery();
	let org = $derived(organizationQuery.data);
	let siteLimit = $derived(org?.plan?.included_sites ?? null);
	let canBuyMore = $derived(org?.plan?.site_cents !== undefined && org?.plan?.site_cents !== null);

	const tagsQuery = useTagsQuery();
	// Every site, for the plan's site count and deep links; the table reads its own page.
	const sitesQuery = useSitesQuery();
	const page = listPageState(() => isActive);
	const sitesPageQuery = useSitesListQuery(() => page.params());
	const fieldValuesQuery = useFieldValuesQuery(
		'/api/v1/sites/field-values',
		['effective_stale_after_hours'],
		undefined,
		() => isActive
	);
	// What each site contains, resolved here for the table's columns.
	const daemonsQuery = useDaemonsQuery();
	const subnetsQuery = useSubnetsQuery();
	const vlansQuery = useVlansQuery();
	const credentialsQuery = useCredentialsQuery();
	useDependenciesQuery();

	// Mutations
	const createSiteMutation = useCreateSiteMutation();
	const updateSiteMutation = useUpdateSiteMutation();
	const deleteSiteMutation = useDeleteSiteMutation();
	const bulkDeleteSitesMutation = useBulkDeleteSitesMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let sitesData = $derived(sitesQuery.data ?? []);
	let pageSites = $derived(sitesPageQuery.data?.items ?? []);
	let sitesPagination = $derived(sitesPageQuery.data?.pagination ?? null);
	let daemonsData = $derived(daemonsQuery.data ?? []);
	// Narrowed once, here: both the per-site subnet column and the daemon chips
	// below must show the same list the Subnets and Daemons tabs do.
	let subnetsData = $derived((subnetsQuery.data ?? []).filter(isUserManagedSubnet));
	let vlansData = $derived(vlansQuery.data ?? []);
	let credentialsData = $derived(credentialsQuery.data ?? []);
	let isLoading = $derived(
		sitesQuery.isPending || (sitesPageQuery.isPending && !sitesPageQuery.data)
	);
	let pageLoad = $derived(loadFailure(organizationQuery, sitesQuery, sitesPageQuery));
	let isAtSiteLimit = $derived(siteLimit !== null && sitesData.length >= siteLimit && !canBuyMore);
	let isNearSiteLimit = $derived(
		siteLimit !== null && sitesData.length >= siteLimit - 2 && !isAtSiteLimit && !canBuyMore
	);

	let showCreateSiteModal = $state(false);
	let editingSite = $state<Site | null>(null);

	// Deep-link: open site editor from URL (handles both fresh open and entity switch)
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'site-editor',
			sitesData,
			showCreateSiteModal,
			editingSite?.id
		);
		if (result !== undefined) {
			editingSite = result;
			showCreateSiteModal = true;
		}
	});

	/** Row actions. */
	function siteActions(site: Site): RowAction[] {
		if (!can('Site', 'Write')) return [];

		return [
			{
				label: common_edit(),
				icon: Edit,
				isOpen: true,
				onClick: () => handleEditSite(site),
				unavailable: refusedUnless(can('Site', 'Write', site.id))
			},
			{
				label: common_delete(),
				icon: Trash2,
				tone: 'danger',
				isDelete: true,
				onBulk: handleBulkDelete,
				onClick: () => handleDeleteSite(site),
				unavailable: refusedUnless(can('Site', 'Delete', site.id))
			}
		];
	}

	function handleDeleteSite(site: Site) {
		if (confirm(sites_confirmDelete({ name: site.name }))) {
			deleteSiteMutation.mutate(site.id);
		}
	}

	function handleCreateSite() {
		editingSite = null;
		showCreateSiteModal = true;
	}

	function handleEditSite(site: Site) {
		editingSite = site;
		showCreateSiteModal = true;
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(common_confirmBulkDelete({ count: ids.length, entity: common_sites() }))) {
			await bulkDeleteSitesMutation.mutateAsync(ids);
		}
	}

	function getSiteTags(site: Site): string[] {
		return site.tags;
	}

	async function handleSiteCreate(data: Site) {
		try {
			await createSiteMutation.mutateAsync(data);
			showCreateSiteModal = false;
			editingSite = null;
		} catch {
			// Error handled by mutation
		}
	}

	async function handleSiteUpdate(id: string, data: Site) {
		try {
			await updateSiteMutation.mutateAsync(data);
			showCreateSiteModal = false;
			editingSite = null;
		} catch {
			// Error handled by mutation
		}
	}

	function handleCloseSiteEditor() {
		showCreateSiteModal = false;
		editingSite = null;
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('Site', page.listQuery);
	}

	const tableDefaults: TableDefaults<SiteOrderField> = {
		sort: { field: 'name', direction: 'asc' }
	};

	// The registry's site fields reach no related entity.
	const lookup = collectionLookup({});

	let siteFields = $derived(
		siteColumns({
			lookup: () => lookup,
			tags: () => tagsData,
			daemons: () => daemonsData,
			subnets: () => subnetsData,
			vlans: () => vlansData,
			credentials: () => credentialsData,
			staleWindowValues: () => fieldValuesQuery.data?.effective_stale_after_hours
		})
	);
</script>

<!-- The page's own actions, last in the table toolbar beside the filter and columns. -->
{#snippet toolbarActions()}
	<div class="flex items-center gap-3">
		{#if siteLimit !== null && !canBuyMore}
			<span
				class="text-sm {isAtSiteLimit
					? 'text-amber-700 dark:text-amber-400'
					: isNearSiteLimit
						? 'text-yellow-700 dark:text-yellow-400'
						: 'text-tertiary'}"
			>
				{sitesData.length} / {siteLimit}
			</span>
		{/if}
		{#if can('NewSite', 'Write')}
			<CreateButton
				onclick={handleCreateSite}
				upgrade={isAtSiteLimit
					? { feature: 'sites', surface: 'sites_tab', gate_type: 'limit_hit' }
					: null}
			/>
		{/if}
	</div>
{/snippet}

<div class="space-y-6">
	{#if pageLoad.failed}
		<LoadFailedState onRetry={pageLoad.retry} />
	{:else if isLoading}
		<Loading />
	{:else if sitesData.length === 0}
		<!-- Empty state -->
		<EmptyState
			title={common_noEntityYet({ entity: common_sites() })}
			subtitle=""
			onClick={can('NewSite', 'Write') ? handleCreateSite : undefined}
			cta={can('NewSite', 'Write') ? common_create() : ''}
		/>
	{:else}
		<DataControls
			title={common_sites()}
			{toolbarActions}
			items={pageSites}
			fields={siteFields}
			entityType="Site"
			canEditTags={can('Tag', 'Write')}
			getItemTags={getSiteTags}
			storageKey="scanopy-sites-table-state"
			defaults={tableDefaults}
			getItemId={(item) => item.id}
			serverPagination={sitesPagination}
			onPageChange={page.onPageChange}
			onQueryChange={page.onQueryChange}
			serverQuery={{ stale: false }}
			onCsvExport={handleCsvExport}
			getActions={siteActions}
			entityLabel={common_sites()}
		></DataControls>
	{/if}
</div>

<SiteEditModal
	name="site-editor"
	isOpen={showCreateSiteModal}
	site={editingSite}
	onCreate={handleSiteCreate}
	onUpdate={handleSiteUpdate}
	onClose={handleCloseSiteEditor}
	onDelete={editingSite
		? () => {
				handleDeleteSite(editingSite!);
				handleCloseSiteEditor();
			}
		: null}
/>
