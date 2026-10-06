<script lang="ts">
	import { lastSeenItems } from '$lib/shared/utils/freshness';
	import { cidrSourceItems, isProvisionalCidr } from '$lib/shared/utils/cidr-source';
	import { cidrContains, compareCidr } from '$lib/shared/utils/cidr';
	import SubnetUtilization from './SubnetUtilization.svelte';
	import { nestedRangeOf, subnetNesting, utilizationRatio } from '../nesting';
	import SubnetEditModal from './SubnetEditModal/SubnetEditModal.svelte';
	import ProvisionalRangeModal from './ProvisionalRangeModal.svelte';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import type { Subnet, SubnetResponse } from '../types/base';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import { defineFields, entityRef, type CardAction } from '$lib/shared/components/data/types';
	import { tagNames } from '$lib/features/tags/columns';
	import { siteItems } from '$lib/features/sites/columns';
	import { Plus, Trash2, Edit, CloudAlert } from 'lucide-svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import {
		isUserManagedSubnet,
		useSubnetsQuery,
		useCreateSubnetMutation,
		useMergeSubnetMutation,
		useUpdateSubnetMutation,
		useDeleteSubnetMutation,
		useBulkDeleteSubnetsMutation
	} from '../queries';
	import { useSitesQuery } from '$lib/features/sites/queries';
	import { useServicesByIds } from '$lib/features/services/queries';
	import type { Service } from '$lib/features/services/types/base';
	import { useDiscoveriesByIds } from '$lib/features/discovery/queries';
	import { discoveryRunIds, discoveryRunItems } from '$lib/features/discovery/columns';
	import type { TabProps } from '$lib/shared/types';
	import type { components } from '$lib/api/schema';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import {
		common_cidr,
		common_confirmDeleteName,
		common_create,
		common_confirmBulkDelete,
		common_created,
		common_description,
		common_lastSeen,
		common_name,
		common_site,
		common_noEntityYet,
		common_delete,
		common_source,
		common_edit,
		common_firstFoundBy,
		common_lastFoundBy,
		common_subnets,
		common_tags,
		common_unknownSite,
		common_updated,
		daemons_installPromptSubnets,
		subnets_managedBy,
		subnets_resolveRange,
		subnets_subnetType,
		common_utilization
	} from '$lib/paraglide/messages';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';
	import { concepts, entitySources, subnetTypes } from '$lib/shared/stores/metadata';
	import { entitySourceItems } from '$lib/shared/utils/entity-source';

	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];
	type SubnetOrderField = components['schemas']['SubnetOrderField'];

	let { isReadOnly = false }: TabProps = $props();

	// Organization query for onboarding state
	const organizationQuery = useOrganizationQuery();
	let onboarding = $derived((organizationQuery.data?.onboarding ?? []) as OnboardingOperation[]);

	// Queries
	const tagsQuery = useTagsQuery();
	// Staleness filter state. Server-side so it applies to the whole set, and
	// keyed separately from the shared subnets cache.
	let stale = $state<boolean | null>(null);
	const subnetsQuery = useSubnetsQuery(undefined, () => stale ?? undefined);

	function handleStaleFilterChange(next: boolean | null) {
		stale = next;
	}
	const sitesQuery = useSitesQuery();
	// The container runtimes that manage the bridge subnets, fetched by id.
	const runtimesQuery = useServicesByIds(() => [
		...new Set(
			(subnetsQuery.data ?? [])
				.map((s) => s.virtualization_service_id)
				.filter((id): id is string => id != null)
		)
	]);
	const discoveryRunsQuery = useDiscoveriesByIds(() => discoveryRunIds(subnetsQuery.data ?? []));

	// Mutations
	const createSubnetMutation = useCreateSubnetMutation();
	const updateSubnetMutation = useUpdateSubnetMutation();
	const mergeSubnetMutation = useMergeSubnetMutation();
	const deleteSubnetMutation = useDeleteSubnetMutation();
	const bulkDeleteSubnetsMutation = useBulkDeleteSubnetsMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let subnetsData = $derived((subnetsQuery.data ?? []).filter(isUserManagedSubnet));
	let sitesData = $derived(sitesQuery.data ?? []);
	let runtimesData = $derived(runtimesQuery.data ?? []);
	let discoveryRunsData = $derived(discoveryRunsQuery.data ?? []);

	let nesting = $derived(subnetNesting(subnetsData));

	function runtimeOf(subnet: Subnet): Service | undefined {
		return runtimesData.find((s) => s.id === subnet.virtualization_service_id);
	}
	let isLoading = $derived(subnetsQuery.isPending);

	let showSubnetEditor = $state(false);
	let editingSubnet = $state<Subnet | null>(null);
	let showProvisionalRange = $state(false);
	let resolvingSubnet = $state<Subnet | null>(null);

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

	/** Row actions for table mode, matching what the card offers. */
	function subnetActions(subnet: Subnet): CardAction[] {
		if (isReadOnly) return [];

		const actions: CardAction[] = [];

		// Only where there is something to resolve. A badge that says a range was guessed and
		// offers no way to settle it leaves the operator with the question and no answer.
		if (isProvisionalCidr(subnet)) {
			// Neutral, like Edit. The badge in the row already carries the colour, and the two
			// variants that would stand out are spoken for: amber means stale and red means broken,
			// which is exactly what an assumed range is not.
			actions.push({
				label: subnets_resolveRange(),
				icon: CloudAlert,
				onClick: () => handleResolveRange(subnet)
			});
		}

		actions.push(
			{ label: common_edit(), icon: Edit, onClick: () => handleEditSubnet(subnet) },
			{
				label: common_delete(),
				icon: Trash2,
				class: 'btn-icon-danger',
				onClick: () => handleDeleteSubnet(subnet)
			}
		);

		return actions;
	}

	function handleResolveRange(subnet: Subnet) {
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
	 * Searching `subnetsData` rather than the raw query is what keeps the `0.0.0.0/0` catch-alls
	 * out: they are synthetic, so `isUserManagedSubnet` has already dropped them, and either would
	 * otherwise "cover" every range on the site.
	 */
	let coveringSubnet = $derived.by(() => {
		if (!resolvingSubnet) return null;
		const target = resolvingSubnet;
		return (
			subnetsData.find(
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
		await downloadCsv('Subnet', {});
	}

	// Define field configuration for the DataTableControls
	// Uses defineFields to ensure all SubnetOrderField values are covered
	let subnetFields = $derived(
		defineFields<SubnetResponse, SubnetOrderField>(
			{
				// Identity fields: grouping by one would render a header per subnet.
				name: {
					label: common_name(),
					type: 'string',
					searchable: true,
					groupable: false,
					display: { order: 0, primary: true, width: 220 }
				},
				cidr: {
					label: common_cidr(),
					type: 'string',
					searchable: true,
					// Groups each range with every range nested inside it, as nested sections. A subnet
					// that nests with nothing falls in the ungrouped bucket rather than heading a
					// group of one.
					getGroupLabel: (subnet) => nestedRangeOf(subnet, nesting),
					// Address order, not text order, which is also the tree order: each range sorts
					// directly before the ranges inside it.
					compare: (a, b) => compareCidr(a.cidr, b.cidr),
					tree: {
						key: (subnet) => subnet.id,
						parentKey: (subnet) => subnet.parent_subnet_id ?? null,
						label: (subnet) => subnet.cidr
					},
					display: { order: 3, getItems: cidrSourceItems() }
				},
				subnet_type: {
					label: subnets_subnetType(),
					type: 'string',
					searchable: true,
					filterable: true,
					// Every type the backend defines, named the way the chip renders them.
					filterOptions: subnetTypes.getItems().map((type) => subnetTypes.getName(type.id)),
					getValue: (subnet) => subnetTypes.getName(subnet.subnet_type),
					display: {
						order: 4,
						getItems: (subnet) => [
							{
								id: subnet.subnet_type,
								...subnetTypes.getTag(subnet.subnet_type),
								icon: subnetTypes.getIconComponent(subnet.subnet_type)
							}
						]
					}
				},
				site_id: {
					label: common_site(),
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					getValue: (item) =>
						sitesData.find((n) => n.id == item.site_id)?.name || common_unknownSite(),
					display: { order: 2, getItems: (item) => siteItems(item.site_id, sitesData) }
				},
				created_at: { label: common_created(), type: 'date', display: { hiddenByDefault: true } },
				updated_at: { label: common_updated(), type: 'date', display: { hiddenByDefault: true } },
				last_seen_at: {
					label: common_lastSeen(),
					type: 'date',
					staleFilter: true,
					display: {
						recency: true,
						order: 1,
						getItems: lastSeenItems(() => sitesData, 'Subnet')
					}
				}
			},
			[
				{
					key: 'utilization',
					label: common_utilization(),
					type: 'string',
					sortable: true,
					getValue: (subnet) => `${Math.round(utilizationRatio(subnet) * 100)}%`,
					compare: (a, b) => utilizationRatio(a) - utilizationRatio(b),
					display: { order: 5, cell: utilizationCell }
				},
				{
					key: 'description',
					label: common_description(),
					type: 'string',
					searchable: true,
					display: { hiddenByDefault: true }
				},
				{
					key: 'source',
					label: common_source(),
					type: 'string',
					filterable: true,
					groupable: true,
					sortable: true,
					// Every source the backend can stamp, named the way the column renders them.
					filterOptions: entitySources.getItems().map((source) => entitySources.getName(source.id)),
					getValue: (subnet) => entitySources.getName(subnet.source.type),
					display: {
						hiddenByDefault: true,
						getItems: (subnet) => entitySourceItems(subnet.source)
					}
				},
				{
					// The container runtime that created this bridge network. Only bridge subnets have one.
					key: 'managed_by',
					label: subnets_managedBy(),
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					sortable: true,
					getValue: (subnet) => runtimeOf(subnet)?.name ?? null,
					display: {
						hiddenByDefault: true,
						getItems: (subnet) => {
							const runtime = runtimeOf(subnet);
							if (!runtime) return [];
							return [
								{
									id: runtime.id,
									label: runtime.name,
									color: concepts.getColorHelper('Containerization').color,
									entityRef: entityRef('Service', runtime.id, runtime)
								}
							];
						}
					}
				},
				{
					key: 'first_found_by',
					label: common_firstFoundBy(),
					type: 'string',
					display: {
						hiddenByDefault: true,
						getItems: (item) => discoveryRunItems(item.first_discovery_id, discoveryRunsData)
					}
				},
				{
					key: 'last_found_by',
					label: common_lastFoundBy(),
					type: 'string',
					display: {
						hiddenByDefault: true,
						getItems: (item) => discoveryRunItems(item.last_discovery_id, discoveryRunsData)
					}
				},
				{
					key: 'tags',
					label: common_tags(),
					type: 'array',
					searchable: true,
					filterable: true,
					getValue: (entity) => tagNames(entity.tags, tagsData)
				}
			]
		)
	);
</script>

{#snippet utilizationCell(subnet: SubnetResponse)}
	<SubnetUtilization {subnet} />
{/snippet}

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={common_subnets()}>
		<svelte:fragment slot="actions">
			{#if hasDaemon(onboarding) && !isReadOnly}
				<button class="btn-primary flex items-center" onclick={handleCreateSubnet}
					><Plus class="h-5 w-5" />{common_create()}</button
				>
			{/if}
		</svelte:fragment>
	</TabHeader>

	{#if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title={daemons_installPromptSubnets()} {isReadOnly} />
	{:else if isLoading}
		<!-- Loading state -->
		<Loading />
	{:else if subnetsData.length === 0 && stale === null}
		<!--
			"Nothing configured yet" only when nothing is narrowing the list. `stale` is a
			server-side filter, so an empty response under it means "no matches", not "no
			subnets" — and this branch replaces the controls that would let the user clear
			it. DataControls renders the filtered-empty state instead.
		-->
		<EmptyState
			title={common_noEntityYet({ entity: common_subnets() })}
			subtitle=""
			onClick={handleCreateSubnet}
			cta={common_create()}
		/>
	{:else}
		<DataControls
			items={subnetsData}
			fields={subnetFields}
			storageKey="scanopy-subnets-table-state"
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			entityType={isReadOnly ? undefined : 'Subnet'}
			getItemTags={getSubnetTags}
			getItemId={(item) => item.id}
			getIcon={(subnet) => ({
				icon: subnetTypes.getIconComponent(subnet.subnet_type),
				color: subnetTypes.getColorHelper(subnet.subnet_type).icon
			})}
			onStaleFilterChange={handleStaleFilterChange}
			onCsvExport={handleCsvExport}
			getActions={subnetActions}
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
