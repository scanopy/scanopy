<script lang="ts">
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import type { Network } from '../types';
	import NetworkCard from './NetworkCard.svelte';
	import NetworkEditModal from './NetworkEditModal.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { FieldConfig } from '$lib/shared/components/data/types';
	import { Plus } from 'lucide-svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { permissions } from '$lib/shared/stores/metadata';
	import UpgradeButton from '$lib/shared/components/UpgradeButton.svelte';
	import type { TabProps } from '$lib/shared/types';
	import {
		common_create,
		common_created,
		common_name,
		common_networks,
		common_tags,
		networks_confirmBulkDelete,
		networks_confirmDelete,
		networks_noNetworksYet
	} from '$lib/paraglide/messages';

	let { isReadOnly = false }: TabProps = $props();
	import {
		useNetworksQuery,
		useCreateNetworkMutation,
		useUpdateNetworkMutation,
		useDeleteNetworkMutation,
		useBulkDeleteNetworksMutation
	} from '../queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { useGroupsQuery } from '$lib/features/groups/queries';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';

	// Queries
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	const organizationQuery = useOrganizationQuery();
	let org = $derived(organizationQuery.data);
	let networkLimit = $derived(org?.plan?.included_networks ?? null);
	let canBuyMore = $derived(
		org?.plan?.network_cents !== undefined && org?.plan?.network_cents !== null
	);

	const tagsQuery = useTagsQuery();
	const networksQuery = useNetworksQuery();
	// Load related data for network cards
	useDaemonsQuery();
	useSubnetsQuery();
	useGroupsQuery();

	// Mutations
	const createNetworkMutation = useCreateNetworkMutation();
	const updateNetworkMutation = useUpdateNetworkMutation();
	const deleteNetworkMutation = useDeleteNetworkMutation();
	const bulkDeleteNetworksMutation = useBulkDeleteNetworksMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);
	let isLoading = $derived(networksQuery.isPending);
	let isAtNetworkLimit = $derived(
		networkLimit !== null && networksData.length >= networkLimit && !canBuyMore
	);
	let isNearNetworkLimit = $derived(
		networkLimit !== null && networksData.length >= networkLimit - 2 && !isAtNetworkLimit
	);

	let showCreateNetworkModal = $state(false);
	let editingNetwork = $state<Network | null>(null);

	// Deep-link: open network editor from URL (handles both fresh open and entity switch)
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'network-editor',
			networksData,
			showCreateNetworkModal,
			editingNetwork?.id
		);
		if (result !== undefined) {
			editingNetwork = result;
			showCreateNetworkModal = true;
		}
	});

	let allowBulkDelete = $derived(
		!isReadOnly && currentUser
			? permissions.getMetadata(currentUser.permissions).manage_org_entities
			: false
	);

	let canManageNetworks = $derived(
		!isReadOnly &&
			currentUser &&
			permissions.getMetadata(currentUser.permissions).manage_org_entities
	);

	function handleDeleteNetwork(network: Network) {
		if (confirm(networks_confirmDelete({ name: network.name }))) {
			deleteNetworkMutation.mutate(network.id);
		}
	}

	function handleCreateNetwork() {
		editingNetwork = null;
		showCreateNetworkModal = true;
	}

	function handleEditNetwork(network: Network) {
		editingNetwork = network;
		showCreateNetworkModal = true;
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(networks_confirmBulkDelete({ count: ids.length }))) {
			await bulkDeleteNetworksMutation.mutateAsync(ids);
		}
	}

	function getNetworkTags(network: Network): string[] {
		return network.tags;
	}

	async function handleNetworkCreate(data: Network) {
		try {
			await createNetworkMutation.mutateAsync(data);
			showCreateNetworkModal = false;
			editingNetwork = null;
		} catch {
			// Error handled by mutation
		}
	}

	async function handleNetworkUpdate(id: string, data: Network) {
		try {
			await updateNetworkMutation.mutateAsync(data);
			showCreateNetworkModal = false;
			editingNetwork = null;
		} catch {
			// Error handled by mutation
		}
	}

	function handleCloseNetworkEditor() {
		showCreateNetworkModal = false;
		editingNetwork = null;
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('Network', {});
	}

	// Define field configuration for the DataTableControls
	const networkFields: FieldConfig<Network>[] = [
		{
			key: 'name',
			label: common_name(),
			type: 'string',
			searchable: true,
			sortable: true
		},
		{
			key: 'tags',
			label: common_tags(),
			type: 'array',
			searchable: true,
			filterable: true,
			getValue: (entity) => {
				// Return tag names for search/filter display
				return entity.tags
					.map((id) => tagsData.find((t) => t.id === id)?.name)
					.filter((name): name is string => !!name);
			}
		},
		{
			key: 'created_at',
			label: common_created(),
			type: 'date',
			sortable: true
		}
	];
</script>

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={common_networks()}>
		<svelte:fragment slot="actions">
			<div class="flex items-center gap-3">
				{#if networkLimit !== null}
					<span
						class="text-sm {isAtNetworkLimit
							? 'text-amber-400'
							: isNearNetworkLimit
								? 'text-yellow-400'
								: 'text-tertiary'}"
					>
						{networksData.length} / {networkLimit}
					</span>
				{/if}
				{#if canManageNetworks}
					{#if isAtNetworkLimit}
						<UpgradeButton feature="networks" />
					{:else}
						{#if isNearNetworkLimit}
							<UpgradeButton feature="networks" />
						{/if}
						<button class="btn-primary flex items-center" onclick={handleCreateNetwork}
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
	{:else if networksData.length === 0}
		<!-- Empty state -->
		<EmptyState
			title={networks_noNetworksYet()}
			subtitle=""
			onClick={handleCreateNetwork}
			cta={common_create()}
		/>
	{:else}
		<DataControls
			items={networksData}
			fields={networkFields}
			onBulkDelete={handleBulkDelete}
			entityType={allowBulkDelete ? 'Network' : undefined}
			getItemTags={getNetworkTags}
			{allowBulkDelete}
			storageKey="scanopy-networks-table-state"
			getItemId={(item) => item.id}
			onCsvExport={handleCsvExport}
		>
			{#snippet children(
				item: Network,
				viewMode: 'card' | 'list',
				isSelected: boolean,
				onSelectionChange: (selected: boolean) => void
			)}
				<NetworkCard
					network={item}
					{viewMode}
					selected={isSelected}
					{onSelectionChange}
					onDelete={handleDeleteNetwork}
					onEdit={handleEditNetwork}
				/>
			{/snippet}
		</DataControls>
	{/if}
</div>

<NetworkEditModal
	name="network-editor"
	isOpen={showCreateNetworkModal}
	network={editingNetwork}
	onCreate={handleNetworkCreate}
	onUpdate={handleNetworkUpdate}
	onClose={handleCloseNetworkEditor}
	onDelete={editingNetwork
		? () => {
				handleDeleteNetwork(editingNetwork!);
				handleCloseNetworkEditor();
			}
		: null}
/>
