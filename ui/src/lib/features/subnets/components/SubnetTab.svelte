<script lang="ts">
	import { rowStatusBadges } from '$lib/shared/utils/row-badges';
	import CreateButton from '$lib/shared/components/data/controls/CreateButton.svelte';
	import { isProvisionalCidr } from '$lib/shared/utils/cidr-source';
	import { cidrContains } from '$lib/shared/utils/cidr';
	import SubnetUtilization from './SubnetUtilization.svelte';
	import SubnetEditModal from './SubnetEditModal/SubnetEditModal.svelte';
	import ProvisionalRangeModal from './ProvisionalRangeModal.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import LoadFailedState from '$lib/shared/components/layout/LoadFailedState.svelte';
	import { loadFailure } from '$lib/shared/utils/load-failure';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import type { Subnet, SubnetResponse } from '../types/base';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import { listPageState } from '$lib/shared/components/data/listPage.svelte';
	import { fieldValueOptions, useFieldValuesQuery } from '$lib/shared/api/field-values';
	import type { TableDefaults } from '$lib/shared/components/data/types';
	import { disabledWhen, hiddenWhen, type RowAction } from '$lib/shared/components/data/types';
	import { subnetColumns } from './SubnetTab.columns';
	import { collectionLookup } from '$lib/shared/entity-fields';
	import { Trash2, Edit, CircleDashedCheck } from '@lucide/svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import {
		isUserManagedSubnet,
		LISTED_SUBNETS,
		useSubnetsQuery,
		useSubnetsByIds,
		useSubnetsListQuery,
		useCreateSubnetMutation,
		useMergeSubnetMutation,
		useUpdateSubnetMutation,
		useDeleteSubnetMutation,
		useBulkDeleteSubnetsMutation
	} from '../queries';
	import { useSitesQuery } from '$lib/features/sites/queries';
	import { useVlansQuery } from '$lib/features/vlans/queries';
	import { useServicesByIds } from '$lib/features/services/queries';
	import type { Service } from '$lib/features/services/types/base';
	import { useDiscoveriesByIds } from '$lib/features/discovery/queries';
	import { discoveryRunIds } from '$lib/features/discovery/columns';
	import type { TabProps } from '$lib/shared/types';
	import type { components } from '$lib/api/schema';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import {
		common_confirmDeleteName,
		common_create,
		common_confirmBulkDelete,
		common_noEntityYet,
		common_delete,
		common_edit,
		common_subnets,
		daemons_installPromptSubnets,
		subnets_rangeSettled,
		subnets_resolveFirst,
		subnets_resolveFirstBulk,
		subnets_resolveRange,
		subnets_sharedRangeReadOnly,
		subnets_sharedRangeReadOnlyBulk
	} from '$lib/paraglide/messages';

	type SubnetOrderField = components['schemas']['SubnetOrderField'];

	import { refusedUnless, useCan } from '$lib/features/auth/access';

	let { isActive = false }: TabProps = $props();
	const can = useCan();

	const organizationQuery = useOrganizationQuery();
	let daemonConnected = $derived(organizationQuery.data?.daemon_connected ?? false);

	// Queries
	const tagsQuery = useTagsQuery();
	const SUBNET_FIELD_VALUES = '/api/v1/subnets/field-values';
	// The filter options count only the ranges the list shows.
	const listedOnly = () => LISTED_SUBNETS;

	// One page of the subnets the user curates, paged, filtered, ordered and nested on the server.
	const page = listPageState(() => isActive);
	const subnetsQuery = useSubnetsListQuery(() => page.params());
	// Declared before the by-id queries below: TanStack reads their options as each query is
	// created, so a later declaration throws "Cannot access 'subnetsData' before initialization".
	let subnetsData = $derived(subnetsQuery.data?.items ?? []);
	// Filter options: the values the listed subnets hold, counted by the server.
	const fieldValuesQuery = useFieldValuesQuery(
		SUBNET_FIELD_VALUES,
		['site_id', 'subnet_type', 'source', 'managed_by'],
		listedOnly,
		() => isActive
	);
	const sitesQuery = useSitesQuery();
	// Shared full-list VLANs cache, for the VLANs each subnet's hydrated `vlan_ids` names.
	const vlansQuery = useVlansQuery();
	// The container runtimes that manage some subnet, fetched by id: the page's, for its cells, and
	// every one the filter offers, for their labels.
	const runtimesQuery = useServicesByIds(() => [
		...new Set([
			...subnetsData
				.map((s) => s.virtualization_service_id)
				.filter((id): id is string => id != null),
			...fieldValueOptions(fieldValuesQuery.data?.managed_by)
		])
	]);
	const discoveryRunsQuery = useDiscoveriesByIds(() => discoveryRunIds(subnetsData));
	// The ranges that head a nesting group on this page but sit on another, so the group header
	// can name them.
	const nestingRootsQuery = useSubnetsByIds(() => {
		const present = new Set(subnetsData.map((s) => s.id));
		return [
			...new Set(
				subnetsData
					.map((s) => s.nesting_root_id)
					.filter((id): id is string => id != null && !present.has(id))
			)
		];
	});

	// Mutations
	const createSubnetMutation = useCreateSubnetMutation();
	const updateSubnetMutation = useUpdateSubnetMutation();
	const mergeSubnetMutation = useMergeSubnetMutation();
	const deleteSubnetMutation = useDeleteSubnetMutation();
	const bulkDeleteSubnetsMutation = useBulkDeleteSubnetsMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let subnetsPagination = $derived(subnetsQuery.data?.pagination ?? null);
	let sitesData = $derived(sitesQuery.data ?? []);
	let vlansData = $derived(vlansQuery.data ?? []);
	let runtimesData = $derived(runtimesQuery.data ?? []);
	let discoveryRunsData = $derived(discoveryRunsQuery.data ?? []);

	let nestingRoots = $derived(
		new Map([...subnetsData, ...(nestingRootsQuery.data ?? [])].map((s) => [s.id, s]))
	);

	function runtimeOf(subnet: Subnet): Service | undefined {
		return runtimesData.find((s) => s.id === subnet.virtualization_service_id);
	}
	let isLoading = $derived(subnetsQuery.isPending && !subnetsQuery.data);
	let pageLoad = $derived(loadFailure(organizationQuery, subnetsQuery));

	let showSubnetEditor = $state(false);
	let editingSubnet = $state<Subnet | null>(null);
	let showProvisionalRange = $state(false);
	let resolvingSubnet = $state<SubnetResponse | null>(null);

	// Deep-link: open subnet editor from URL (handles both fresh open and entity switch)
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'subnet-editor',
			subnetsData,
			showSubnetEditor,
			editingSubnet?.id
		);
		if (result !== undefined) {
			editingSubnet = result;
			showSubnetEditor = true;
		}
	});

	/**
	 * Deep-link: resolve an assumed range from anywhere that names one.
	 *
	 * A scan warning is the other place an operator meets a guessed range, and it is where they
	 * meet it first — this is what lets that row hand off to the same four-way choice instead of
	 * growing a second copy of it. Same shape as the editor's deep link above.
	 */
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'provisional-range',
			subnetsData,
			showProvisionalRange,
			resolvingSubnet?.id,
			isProvisionalCidr
		);
		// Create mode is meaningless here — there is no range to resolve without a subnet — so a
		// link that names none is ignored rather than opening an empty modal.
		if (result) {
			resolvingSubnet = result;
			showProvisionalRange = true;
		}
	});

	function handleCreateSubnet() {
		editingSubnet = null;
		showSubnetEditor = true;
	}

	/** Row actions. */
	function subnetActions(subnet: SubnetResponse): RowAction[] {
		if (!can('Subnet', 'Write')) return [];

		const writeRefusal = refusedUnless(can('Subnet', 'Write', subnet.site_id));
		// The Internet and Remote rows stand for a copy on every site, and editing one would change
		// only one site's copy.
		const shared = disabledWhen(
			!isUserManagedSubnet(subnet),
			subnets_sharedRangeReadOnly(),
			subnets_sharedRangeReadOnlyBulk()
		);
		// An assumed range is settled first, through Resolve range.
		const unresolved = disabledWhen(
			isProvisionalCidr(subnet),
			subnets_resolveFirst(),
			subnets_resolveFirstBulk()
		);

		return [
			// Only where there is something to resolve. A badge that says a range was guessed and
			// offers no way to settle it leaves the operator with the question and no answer.
			// Neutral, like Edit. The badge in the row already carries the colour, and the two
			// variants that would stand out are spoken for: amber means stale and red means broken,
			// which is exactly what an assumed range is not.
			{
				label: subnets_resolveRange(),
				icon: CircleDashedCheck,
				onClick: () => handleResolveRange(subnet),
				unavailable: hiddenWhen(!isProvisionalCidr(subnet), subnets_rangeSettled()) ?? writeRefusal
			},
			{
				label: common_edit(),
				icon: Edit,
				isOpen: true,
				onClick: () => handleEditSubnet(subnet),
				unavailable: writeRefusal ?? shared ?? unresolved
			},
			{
				label: common_delete(),
				icon: Trash2,
				tone: 'danger',
				isDelete: true,
				onBulk: handleBulkDelete,
				onClick: () => handleDeleteSubnet(subnet),
				unavailable: refusedUnless(can('Subnet', 'Delete', subnet.site_id)) ?? shared ?? unresolved
			}
		];
	}

	function handleResolveRange(subnet: SubnetResponse) {
		resolvingSubnet = subnet;
		showProvisionalRange = true;
	}

	/**
	 * A settled range that already covers the one being resolved, if the site holds one.
	 *
	 * This is the state discovery deliberately leaves alone: where a reading covers *several*
	 * assumed ranges it corrects none of them, because folding them into one means deleting rows.
	 * Offering the merge here is how that gets resolved, one deliberate click at a time.
	 *
	 * The covering range can sit on another page, so this reads the full list, only while a range is
	 * being resolved. Filtering it through `isUserManagedSubnet` is what keeps the `0.0.0.0/0`
	 * catch-alls out: they are synthetic, and either would otherwise "cover" every range on the site.
	 */
	const allSubnetsQuery = useSubnetsQuery(undefined, () => resolvingSubnet !== null);
	let coveringSubnet = $derived.by(() => {
		if (!resolvingSubnet) return null;
		const target = resolvingSubnet;
		return (
			(allSubnetsQuery.data ?? [])
				.filter(isUserManagedSubnet)
				.find(
					(candidate) =>
						candidate.id !== target.id &&
						candidate.site_id === target.site_id &&
						!isProvisionalCidr(candidate) &&
						cidrContains(candidate.cidr, target.cidr)
				) ?? null
		);
	});

	async function handleMergeRange(subnet: Subnet, into: Subnet) {
		try {
			await mergeSubnetMutation.mutateAsync({ id: subnet.id, into: into.id });
			handleCloseProvisionalRange();
		} catch {
			// Error handled by mutation
		}
	}

	function handleCloseProvisionalRange() {
		showProvisionalRange = false;
		resolvingSubnet = null;
	}

	/**
	 * Keep the range as it stands. The server takes the higher rung of the ladder on update, so a
	 * range a person confirmed sticks and the next scan cannot put the row back to an inference.
	 *
	 * `Manual` is that rung on the shared ladder — the same one a value typed into any other field
	 * carries, which is what makes "nothing discovery reads displaces it" one rule rather than a
	 * per-field convention.
	 */
	async function handleConfirmRange(subnet: Subnet) {
		try {
			await updateSubnetMutation.mutateAsync({ ...subnet, cidr_source: 'Manual' });
			handleCloseProvisionalRange();
		} catch {
			// Error handled by mutation
		}
	}

	/** Straight into the editor, which is where a corrected CIDR is typed. */
	function handleCorrectRange(subnet: Subnet) {
		handleCloseProvisionalRange();
		handleEditSubnet(subnet);
	}

	function handleEditSubnet(subnet: Subnet) {
		editingSubnet = subnet;
		showSubnetEditor = true;
	}

	function handleDeleteSubnet(subnet: Subnet) {
		if (confirm(common_confirmDeleteName({ name: subnet.name }))) {
			deleteSubnetMutation.mutate(subnet.id);
		}
	}

	async function handleSubnetCreate(data: Subnet) {
		try {
			await createSubnetMutation.mutateAsync(data);
			showSubnetEditor = false;
			editingSubnet = null;
		} catch {
			// Error handled by mutation
		}
	}

	async function handleSubnetUpdate(_id: string, data: Subnet) {
		try {
			await updateSubnetMutation.mutateAsync(data);
			showSubnetEditor = false;
			editingSubnet = null;
		} catch {
			// Error handled by mutation
		}
	}

	function handleCloseSubnetEditor() {
		showSubnetEditor = false;
		editingSubnet = null;
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(common_confirmBulkDelete({ count: ids.length, entity: common_subnets() }))) {
			await bulkDeleteSubnetsMutation.mutateAsync(ids);
		}
	}

	function getSubnetTags(subnet: Subnet): string[] {
		return subnet.tags;
	}

	// CSV export handler
	async function handleCsvExport() {
		// A one-off copy for the request, never read reactively.
		// eslint-disable-next-line svelte/prefer-svelte-reactivity
		const params = new URLSearchParams(page.listQuery);
		for (const [key, value] of Object.entries(LISTED_SUBNETS)) params.set(key, String(value));
		await downloadCsv('Subnet', params);
	}

	const tableDefaults: TableDefaults<SubnetOrderField> = {
		group: 'cidr',
		sort: { field: 'cidr', direction: 'asc' }
	};

	// Related entities the registry's subnet fields read: the managing runtime, the site, the VLANs.
	let lookup = $derived(
		collectionLookup({ Service: runtimesData, Site: sitesData, Vlan: vlansData })
	);

	let subnetFields = $derived(
		subnetColumns({
			lookup: () => lookup,
			sites: () => sitesData,
			tags: () => tagsData,
			discoveryRuns: () => discoveryRunsData,
			nestingRoots: () => nestingRoots,
			runtimeOf,
			runtimes: () => runtimesData,
			utilizationCell,
			siteValues: () => fieldValuesQuery.data?.site_id,
			typeValues: () => fieldValuesQuery.data?.subnet_type,
			sourceValues: () => fieldValuesQuery.data?.source,
			managedByValues: () => fieldValuesQuery.data?.managed_by
		})
	);
</script>

{#snippet utilizationCell(subnet: SubnetResponse)}
	<SubnetUtilization {subnet} />
{/snippet}

<!-- The page's own actions, last in the table toolbar beside the filter and columns. -->
{#snippet toolbarActions()}
	{#if daemonConnected && can('Subnet', 'Write')}
		<CreateButton onclick={handleCreateSubnet} />
	{/if}
{/snippet}

<div class="space-y-6">
	{#if pageLoad.failed}
		<LoadFailedState onRetry={pageLoad.retry} />
	{:else if !daemonConnected}
		<PreDaemonEmptyState title={daemons_installPromptSubnets()} />
	{:else if isLoading}
		<!-- Loading state -->
		<Loading />
	{:else if page.isEmpty(subnetsQuery)}
		<EmptyState
			title={common_noEntityYet({ entity: common_subnets() })}
			subtitle=""
			onClick={can('Subnet', 'Write') ? handleCreateSubnet : undefined}
			cta={can('Subnet', 'Write') ? common_create() : ''}
		/>
	{:else}
		<DataControls
			title={common_subnets()}
			{toolbarActions}
			items={subnetsData}
			fields={subnetFields}
			storageKey="scanopy-subnets-table-state"
			defaults={tableDefaults}
			entityType="Subnet"
			canEditTags={can('Tag', 'Write')}
			getItemTags={getSubnetTags}
			getItemId={(item) => item.id}
			serverPagination={subnetsPagination}
			onPageChange={page.onPageChange}
			onQueryChange={page.onQueryChange}
			onCsvExport={handleCsvExport}
			getActions={subnetActions}
			getRowBadges={rowStatusBadges(() => sitesData, 'Subnet')}
			entityLabel={common_subnets()}
		></DataControls>
	{/if}
</div>

<SubnetEditModal
	name="subnet-editor"
	isOpen={showSubnetEditor}
	subnet={editingSubnet}
	onCreate={handleSubnetCreate}
	onUpdate={handleSubnetUpdate}
	onClose={handleCloseSubnetEditor}
	onDelete={editingSubnet
		? () => {
				handleDeleteSubnet(editingSubnet!);
				handleCloseSubnetEditor();
			}
		: null}
/>

<ProvisionalRangeModal
	isOpen={showProvisionalRange}
	subnet={resolvingSubnet}
	coveredBy={coveringSubnet}
	onConfirm={handleConfirmRange}
	onCorrect={handleCorrectRange}
	onMerge={handleMergeRange}
	onClose={handleCloseProvisionalRange}
/>
