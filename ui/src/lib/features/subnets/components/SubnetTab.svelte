<script lang="ts">
	import SubnetCard from './SubnetCard.svelte';
	import SubnetEditModal from './SubnetEditModal/SubnetEditModal.svelte';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import type { Subnet } from '../types/base';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import { defineFields } from '$lib/shared/components/data/types';
	import { Plus } from 'lucide-svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import {
		useSubnetsQuery,
		useCreateSubnetMutation,
		useUpdateSubnetMutation,
		useDeleteSubnetMutation,
		useBulkDeleteSubnetsMutation
	} from '../queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import type { TabProps } from '$lib/shared/types';
	import type { components } from '$lib/api/schema';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import {
		common_cidr,
		common_confirmDeleteName,
		common_create,
		common_created,
		common_description,
		common_name,
		common_network,
		common_subnets,
		common_tags,
		common_unknownNetwork,
		common_updated,
		subnets_confirmBulkDelete,
		subnets_noSubnetsYet,
		subnets_subnetType
	} from '$lib/paraglide/messages';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';
	import { subnetTypes } from '$lib/shared/stores/metadata';

	type OnboardingOperation = components['schemas']['OnboardingOperation'];
	type SubnetOrderField = components['schemas']['SubnetOrderField'];

	let { isReadOnly = false }: TabProps = $props();

	// Organization query for onboarding state
	const organizationQuery = useOrganizationQuery();
	let onboarding = $derived((organizationQuery.data?.onboarding ?? []) as OnboardingOperation[]);

	// Queries
	const tagsQuery = useTagsQuery();
	const subnetsQuery = useSubnetsQuery();
	const networksQuery = useNetworksQuery();

	// Mutations
	const createSubnetMutation = useCreateSubnetMutation();
	const updateSubnetMutation = useUpdateSubnetMutation();
	const deleteSubnetMutation = useDeleteSubnetMutation();
	const bulkDeleteSubnetsMutation = useBulkDeleteSubnetsMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let subnetsData = $derived(
		(subnetsQuery.data ?? []).filter(
			(s) => !subnetTypes.getMetadata(s.subnet_type).hide_from_subnet_list
		)
	);
	let networksData = $derived(networksQuery.data ?? []);
	let isLoading = $derived(subnetsQuery.isPending);

	let showSubnetEditor = $state(false);
	let editingSubnet = $state<Subnet | null>(null);

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

	function handleCreateSubnet() {
		editingSubnet = null;
		showSubnetEditor = true;
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
		if (confirm(subnets_confirmBulkDelete({ count: ids.length }))) {
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
		defineFields<Subnet, SubnetOrderField>(
			{
				name: { label: common_name(), type: 'string', searchable: true },
				cidr: { label: common_cidr(), type: 'string', searchable: true },
				subnet_type: {
					label: subnets_subnetType(),
					type: 'string',
					searchable: true,
					filterable: true
				},
				network_id: {
					label: common_network(),
					type: 'string',
					filterable: true,
					groupable: true,
					getValue: (item) =>
						networksData.find((n) => n.id == item.network_id)?.name || common_unknownNetwork()
				},
				created_at: { label: common_created(), type: 'date' },
				updated_at: { label: common_updated(), type: 'date' }
			},
			[
				{ key: 'description', label: common_description(), type: 'string', searchable: true },
				{
					key: 'tags',
					label: common_tags(),
					type: 'array',
					searchable: true,
					filterable: true,
					getValue: (entity) =>
						entity.tags
							.map((id) => tagsData.find((t) => t.id === id)?.name)
							.filter((name): name is string => !!name)
				}
			]
		)
	);
</script>

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
		<PreDaemonEmptyState title="Install a daemon to start discovering subnets on your network." />
	{:else if isLoading}
		<!-- Loading state -->
		<Loading />
	{:else if subnetsData.length === 0}
		<!-- Empty state -->
		<EmptyState
			title={subnets_noSubnetsYet()}
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
			onCsvExport={handleCsvExport}
		>
			{#snippet children(
				item: Subnet,
				viewMode: 'card' | 'list',
				isSelected: boolean,
				onSelectionChange: (selected: boolean) => void
			)}
				<SubnetCard
					subnet={item}
					selected={isSelected}
					{onSelectionChange}
					{viewMode}
					onEdit={isReadOnly ? undefined : handleEditSubnet}
					onDelete={isReadOnly ? undefined : handleDeleteSubnet}
				/>
			{/snippet}
		</DataControls>
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
