<script lang="ts">
	import { rowStatusBadges } from '$lib/shared/utils/row-badges';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import LoadFailedState from '$lib/shared/components/layout/LoadFailedState.svelte';
	import { loadFailure } from '$lib/shared/utils/load-failure';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { TableDefaults } from '$lib/shared/components/data/types';
	import { vlanColumns } from './VlanTab.columns';
	import { collectionLookup } from '$lib/shared/entity-fields';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useSitesQuery } from '$lib/features/sites/queries';
	import { isUserManagedSubnet, useSubnetsQuery } from '$lib/features/subnets/queries';
	import {
		useBulkDeleteVlansMutation,
		useDeleteVlanMutation,
		useUpdateVlanMutation,
		useVlansListQuery
	} from '../queries';
	import { listPageState } from '$lib/shared/components/data/listPage.svelte';
	import { useFieldValuesQuery } from '$lib/shared/api/field-values';
	import VlanEditModal from './VlanEditModal.svelte';
	import { Edit, Trash2 } from '@lucide/svelte';
	import type { RowAction } from '$lib/shared/components/data/types';
	import type { TabProps } from '$lib/shared/types';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import { useDiscoveriesByIds } from '$lib/features/discovery/queries';
	import { useInterfacesByIds } from '$lib/features/interfaces/queries';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import { discoveryRunIds } from '$lib/features/discovery/columns';
	import type { Vlan, VlanOrderField } from '../types/base';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import {
		common_confirmBulkDelete,
		common_confirmDeleteName,
		common_delete,
		common_edit,
		common_noEntityYet,
		common_vlans,
		daemons_installPromptVlans,
		vlans_emptySubtitle,
		vlans_vlanTitle
	} from '$lib/paraglide/messages';

	import { refusedUnless, useCan } from '$lib/features/auth/access';

	let { isActive = false }: TabProps = $props();
	const can = useCan();

	const VLAN_FIELD_VALUES = '/api/v1/vlans/field-values';

	let showVlanEditor = $state(false);
	let editingVlan: Vlan | null = $state(null);

	// Deep-link: open the VLAN editor from the URL (fresh open and entity switch)
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'vlan-editor',
			vlansData,
			showVlanEditor,
			editingVlan?.id
		);
		if (result) {
			editingVlan = result;
			showVlanEditor = true;
		}
	});

	const organizationQuery = useOrganizationQuery();
	let daemonConnected = $derived(organizationQuery.data?.daemon_connected ?? false);

	// Queries
	// One page of VLANs, paged, filtered and ordered on the server.
	const page = listPageState(() => isActive);
	const vlansQuery = useVlansListQuery(() => page.params());
	// Filter options: the values the caller's VLANs hold, counted by the server.
	const fieldValuesQuery = useFieldValuesQuery(
		VLAN_FIELD_VALUES,
		['site_id', 'source'],
		undefined,
		() => isActive
	);
	const sitesQuery = useSitesQuery();
	// Shared full-list subnets cache — used to resolve the hydrated `subnet_ids`
	// on each VLAN into names.
	const subnetsQuery = useSubnetsQuery();
	const tagsQuery = useTagsQuery();
	const discoveryRunsQuery = useDiscoveriesByIds(() =>
		discoveryRunIds(vlansQuery.data?.items ?? [])
	);
	// The interfaces serving each VLAN, named by the hydrated `interface_ids`.
	const interfacesQuery = useInterfacesByIds(() => [
		...new Set((vlansQuery.data?.items ?? []).flatMap((vlan) => vlan.interface_ids ?? []))
	]);

	// Derived data
	let vlansData = $derived(vlansQuery.data?.items ?? []);
	let vlansPagination = $derived(vlansQuery.data?.pagination ?? null);
	let sitesData = $derived(sitesQuery.data ?? []);
	let discoveryRunsData = $derived(discoveryRunsQuery.data ?? []);
	let interfacesData = $derived(interfacesQuery.data ?? []);
	let tagsData = $derived(tagsQuery.data ?? []);
	let subnetsById = $derived(
		new Map((subnetsQuery.data ?? []).filter(isUserManagedSubnet).map((s) => [s.id, s]))
	);
	let isLoading = $derived(vlansQuery.isPending && !vlansQuery.data);
	let pageLoad = $derived(loadFailure(organizationQuery, vlansQuery));

	const updateVlanMutation = useUpdateVlanMutation();
	const deleteVlanMutation = useDeleteVlanMutation();
	const bulkDeleteVlansMutation = useBulkDeleteVlansMutation();

	/** Row actions. */
	function vlanActions(vlan: Vlan): RowAction[] {
		if (!can('Vlan', 'Write')) return [];
		return [
			{
				label: common_edit(),
				icon: Edit,
				isOpen: true,
				onClick: () => handleEditVlan(vlan),
				unavailable: refusedUnless(can('Vlan', 'Write', vlan.site_id))
			},
			{
				label: common_delete(),
				icon: Trash2,
				tone: 'danger',
				isDelete: true,
				onBulk: handleBulkDelete,
				onClick: () => handleDeleteVlan(vlan),
				unavailable: refusedUnless(can('Vlan', 'Delete', vlan.site_id))
			}
		];
	}

	function handleDeleteVlan(vlan: Vlan) {
		const name = vlans_vlanTitle({ number: vlan.vlan_number, name: vlan.name });
		if (confirm(common_confirmDeleteName({ name }))) {
			deleteVlanMutation.mutate(vlan.id);
		}
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(common_confirmBulkDelete({ count: ids.length, entity: common_vlans() }))) {
			await bulkDeleteVlansMutation.mutateAsync(ids);
		}
	}

	function handleEditVlan(vlan: Vlan) {
		editingVlan = vlan;
		showVlanEditor = true;
	}

	async function handleVlanUpdate(data: Vlan) {
		try {
			await updateVlanMutation.mutateAsync(data);
			handleCloseVlanEditor();
		} catch {
			// Error handled by mutation
		}
	}

	function handleCloseVlanEditor() {
		showVlanEditor = false;
		editingVlan = null;
	}

	function getVlanTags(vlan: Vlan): string[] {
		return vlan.tags;
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('Vlan', page.listQuery);
	}

	const tableDefaults: TableDefaults<VlanOrderField> = {
		sort: { field: 'vlan_number', direction: 'asc' }
	};

	// Related entities the registry's VLAN fields read: the subnets and interfaces a VLAN
	// carries, the site.
	let lookup = $derived(
		collectionLookup({
			Subnet: [...subnetsById.values()],
			Interface: interfacesData,
			Site: sitesData
		})
	);

	let vlanFields = $derived(
		vlanColumns({
			lookup: () => lookup,
			sites: () => sitesData,
			subnets: () => [...subnetsById.values()],
			tags: () => tagsData,
			discoveryRuns: () => discoveryRunsData,
			siteValues: () => fieldValuesQuery.data?.site_id,
			sourceValues: () => fieldValuesQuery.data?.source
		})
	);
</script>

<div class="space-y-6">
	<!-- Header: no actions — VLANs are discovery-populated, so there is nothing to create -->

	{#if pageLoad.failed}
		<LoadFailedState onRetry={pageLoad.retry} />
	{:else if !daemonConnected}
		<PreDaemonEmptyState title={daemons_installPromptVlans()} />
	{:else if isLoading}
		<!-- Loading state -->
		<Loading />
	{:else if page.isEmpty(vlansQuery)}
		<!-- Empty state: no CTA, there is nothing to create -->
		<EmptyState
			title={common_noEntityYet({ entity: common_vlans() })}
			subtitle={vlans_emptySubtitle()}
		/>
	{:else}
		<DataControls
			title={common_vlans()}
			items={vlansData}
			fields={vlanFields}
			storageKey="scanopy-vlans-table-state"
			defaults={tableDefaults}
			entityType="Vlan"
			canEditTags={can('Tag', 'Write')}
			getItemTags={getVlanTags}
			getItemId={(item) => item.id}
			serverPagination={vlansPagination}
			onPageChange={page.onPageChange}
			onQueryChange={page.onQueryChange}
			serverQuery={{ stale: false }}
			onCsvExport={handleCsvExport}
			getActions={vlanActions}
			getRowBadges={rowStatusBadges(() => sitesData, 'Vlan')}
			entityLabel={common_vlans()}
		></DataControls>
	{/if}
</div>

<VlanEditModal
	name="vlan-editor"
	isOpen={showVlanEditor}
	vlan={editingVlan}
	onUpdate={handleVlanUpdate}
	onClose={handleCloseVlanEditor}
/>
