<script lang="ts">
	import { credentialItems } from '$lib/features/credentials/columns';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import type { Site } from '../types';
	import SiteEditModal from './SiteEditModal.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { TableDefaults } from '$lib/shared/components/data/types';
	import type { FieldConfig } from '$lib/shared/components/data/types';
	import { tagNames } from '$lib/features/tags/columns';
	import { entityRef, type CardAction } from '$lib/shared/components/data/types';
	import { entities } from '$lib/shared/stores/metadata';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import type { Credential } from '$lib/features/credentials/types/base';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import type { Subnet } from '$lib/features/subnets/types/base';
	import { Plus, Trash2, Edit } from 'lucide-svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { permissions } from '$lib/shared/stores/metadata';
	import UpgradeButton from '$lib/shared/components/UpgradeButton.svelte';
	import type { TabProps } from '$lib/shared/types';
	import {
		common_confirmBulkDelete,
		common_create,
		common_created,
		common_credentials,
		common_daemons,
		common_delete,
		common_edit,
		common_subnets,
		common_vlans,
		common_name,
		common_sites,
		common_noEntityYet,
		common_tags,
		common_hoursCount,
		common_updated,
		sites_confirmDelete,
		sites_staleAfter,
		sites_staleAfterDefault
	} from '$lib/paraglide/messages';

	let { isReadOnly = false }: TabProps = $props();
	import {
		useSitesQuery,
		useCreateSiteMutation,
		useUpdateSiteMutation,
		useDeleteSiteMutation,
		useBulkDeleteSitesMutation
	} from '../queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import { isUserManagedSubnet, useSubnetsQuery } from '$lib/features/subnets/queries';
	import { useVlansQuery } from '$lib/features/vlans/queries';
	import type { Vlan } from '$lib/features/vlans/types/base';
	import { useDependenciesQuery } from '$lib/features/dependencies/queries';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';

	// Queries
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	const organizationQuery = useOrganizationQuery();
	let org = $derived(organizationQuery.data);
	let siteLimit = $derived(org?.plan?.included_sites ?? null);
	let canBuyMore = $derived(org?.plan?.site_cents !== undefined && org?.plan?.site_cents !== null);

	const tagsQuery = useTagsQuery();
	const sitesQuery = useSitesQuery();
	// What each site contains, resolved here for the table's columns.
	const daemonsQuery = useDaemonsQuery();
	const subnetsQuery = useSubnetsQuery();
	const vlansQuery = useVlansQuery();
	const credentialsQuery = useCredentialsQuery();
	useDependenciesQuery();

	// Only the hosts the daemons run on: each daemon chip needs one host name,
	// and an unpaginated org-wide hosts query is ~1.9MB, shared by key with
	// every other consumer.
	let daemonHostIds = $derived([
		...new Set((daemonsQuery.data ?? []).map((d) => d.host_id).filter((id): id is string => !!id))
	]);
	const daemonHostsQuery = useHostsByIds(() => daemonHostIds);
	let daemonHosts = $derived(daemonHostsQuery.data ?? []);

	// Mutations
	const createSiteMutation = useCreateSiteMutation();
	const updateSiteMutation = useUpdateSiteMutation();
	const deleteSiteMutation = useDeleteSiteMutation();
	const bulkDeleteSitesMutation = useBulkDeleteSitesMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let sitesData = $derived(sitesQuery.data ?? []);
	let daemonsData = $derived(daemonsQuery.data ?? []);
	// Narrowed once, here: both the per-site subnet column and the daemon chips
	// below must show the same list the Subnets and Daemons tabs do.
	let subnetsData = $derived((subnetsQuery.data ?? []).filter(isUserManagedSubnet));
	let vlansData = $derived(vlansQuery.data ?? []);
	let credentialsData = $derived(credentialsQuery.data ?? []);
	let isLoading = $derived(sitesQuery.isPending);
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

	let allowBulkDelete = $derived(
		!isReadOnly && currentUser
			? permissions.getMetadata(currentUser.permissions).manage_org_entities
			: false
	);

	let canManageSites = $derived(
		!isReadOnly &&
			currentUser &&
			permissions.getMetadata(currentUser.permissions).manage_org_entities
	);

	/** Row actions. */
	function siteActions(site: Site): CardAction[] {
		if (!allowBulkDelete) return [];

		return [
			{ label: common_edit(), icon: Edit, onClick: () => handleEditSite(site) },
			{
				label: common_delete(),
				icon: Trash2,
				class: 'btn-icon-danger',
				onClick: () => handleDeleteSite(site)
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
		await downloadCsv('Site', {});
	}

	// What a site contains, one resolver per column.
	function siteDaemons(site: Site): Daemon[] {
		return daemonsData.filter((daemon) => daemon.site_id === site.id);
	}

	function siteSubnets(site: Site): Subnet[] {
		return subnetsData.filter((subnet) => subnet.site_id === site.id);
	}

	function siteVlans(site: Site): Vlan[] {
		return vlansData.filter((vlan) => vlan.site_id === site.id);
	}

	function siteCredentials(site: Site): Credential[] {
		return (site.credential_ids ?? [])
			.map((id) => credentialsData.find((c) => c.id === id))
			.filter((c): c is Credential => Boolean(c));
	}

	const tableDefaults: TableDefaults<string> = { sort: { field: 'name', direction: 'asc' } };

	// Derived, not a plain const: it closes over `tagsData` and references the
	// `tagsCell` snippet, neither of which exists yet when the script body runs.
	let siteFields = $derived<FieldConfig<Site>[]>([
		{
			key: 'name',
			label: common_name(),
			type: 'string',
			searchable: true,
			sortable: true,
			display: { primary: true, width: 220, order: 0 }
		},
		{
			key: 'vlans',
			label: common_vlans(),
			type: 'array',
			searchable: true,
			getValue: (site) => siteVlans(site).map((v) => v.name),
			display: {
				order: 1,
				getItems: (site) =>
					siteVlans(site).map((vlan) => ({
						id: vlan.id,
						label: vlan.name,
						color: entities.getColorHelper('Vlan').color,
						entityRef: entityRef('Vlan', vlan.id, vlan)
					}))
			}
		},
		{
			key: 'tags',
			label: common_tags(),
			type: 'array',
			searchable: true,
			filterable: true,
			getValue: (entity) => tagNames(entity.tags, tagsData)
		},
		{
			key: 'daemons',
			label: common_daemons(),
			type: 'array',
			searchable: true,
			getValue: (site) => siteDaemons(site).map((d) => d.name),
			display: {
				order: 3,
				getItems: (site) =>
					siteDaemons(site).map((daemon) => ({
						id: daemon.id,
						label: daemon.name,
						color: entities.getColorHelper('Daemon').color,
						entityRef: entityRef('Daemon', daemon.id, daemon, {
							hosts: daemonHosts,
							subnets: subnetsData
						})
					}))
			}
		},
		{
			key: 'credentials',
			label: common_credentials(),
			type: 'array',
			searchable: true,
			// Credentials are shared across sites, so this filters. The other arrays here hold
			// members of one site each, which search already finds.
			filterable: true,
			getValue: (site) => siteCredentials(site).map((c) => c.name),
			display: {
				order: 4,
				getItems: (site) => credentialItems(siteCredentials(site))
			}
		},
		{
			key: 'subnets',
			label: common_subnets(),
			type: 'array',
			searchable: true,
			getValue: (site) => siteSubnets(site).map((s) => s.name),
			display: {
				order: 2,
				getItems: (site) =>
					siteSubnets(site).map((subnet) => ({
						id: subnet.id,
						label: subnet.name,
						color: entities.getColorHelper('Subnet').color,
						entityRef: entityRef('Subnet', subnet.id, subnet)
					}))
			}
		},
		{
			key: 'created_at',
			label: common_created(),
			type: 'date',
			sortable: true,
			display: { hiddenByDefault: true }
		},
		{
			key: 'updated_at',
			label: common_updated(),
			type: 'date',
			sortable: true,
			display: { hiddenByDefault: true }
		},
		{
			// The effective window, with the server default applied, so a site that
			// never set its own still shows the number staleness is judged by.
			key: 'effective_stale_after_hours',
			label: sites_staleAfter(),
			type: 'string',
			// A handful of values across sites; numeric collation orders 24 before 168.
			sortable: true,
			groupable: true,
			filterable: true,
			// Marked when no override is set, so an override equal to the default still reads as one.
			getValue: (site) => {
				if (site.effective_stale_after_hours == null) return null;
				const hours = common_hoursCount({ hours: site.effective_stale_after_hours });
				return site.stale_after_hours == null ? sites_staleAfterDefault({ hours }) : hours;
			},
			display: { hiddenByDefault: true }
		}
	]);
</script>

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={common_sites()}>
		<svelte:fragment slot="actions">
			<div class="flex items-center gap-3">
				{#if siteLimit !== null && !canBuyMore}
					<span
						class="text-sm {isAtSiteLimit
							? 'text-amber-400'
							: isNearSiteLimit
								? 'text-yellow-400'
								: 'text-tertiary'}"
					>
						{sitesData.length} / {siteLimit}
					</span>
				{/if}
				{#if canManageSites}
					{#if isAtSiteLimit}
						<UpgradeButton feature="sites" surface="sites_tab" gate_type="limit_hit" />
					{:else}
						{#if isNearSiteLimit}
							<UpgradeButton feature="sites" surface="sites_tab" gate_type="limit_hit" />
						{/if}
						<button class="btn-primary flex items-center" onclick={handleCreateSite}
							><Plus class="h-5 w-5" />{common_create()}</button
						>
					{/if}
				{/if}
			</div>
		</svelte:fragment>
	</TabHeader>

	<!-- Loading state -->
	{#if isLoading}
		<Loading />
	{:else if sitesData.length === 0}
		<!-- Empty state -->
		<EmptyState
			title={common_noEntityYet({ entity: common_sites() })}
			subtitle=""
			onClick={handleCreateSite}
			cta={common_create()}
		/>
	{:else}
		<DataControls
			items={sitesData}
			fields={siteFields}
			onBulkDelete={handleBulkDelete}
			entityType={allowBulkDelete ? 'Site' : undefined}
			getItemTags={getSiteTags}
			{allowBulkDelete}
			storageKey="scanopy-sites-table-state"
			defaults={tableDefaults}
			getItemId={(item) => item.id}
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
