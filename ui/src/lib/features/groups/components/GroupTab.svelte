<script lang="ts">
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import type { Group } from '../types/base';
	import GroupCard from './GroupCard.svelte';
	import GroupEditModal from './GroupEditModal/GroupEditModal.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import { defineFields } from '$lib/shared/components/data/types';
	import { Plus } from 'lucide-svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import {
		useGroupsQuery,
		useCreateGroupMutation,
		useUpdateGroupMutation,
		useDeleteGroupMutation,
		useBulkDeleteGroupsMutation
	} from '../queries';
	import { useServicesCacheQuery } from '$lib/features/services/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useHostsQuery } from '$lib/features/hosts/queries';
	import type { TabProps } from '$lib/shared/types';
	import type { components } from '$lib/api/schema';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import {
		common_confirmDeleteName,
		common_create,
		common_created,
		common_description,
		common_groupsLabel,
		common_name,
		common_network,
		common_tags,
		common_unknownNetwork,
		common_updated,
		groups_confirmBulkDelete,
		groups_groupType,
		groups_noGroupsHelp,
		groups_noGroupsYet,
		groups_subtitle
	} from '$lib/paraglide/messages';

	type GroupOrderField = components['schemas']['GroupOrderField'];
	type OnboardingOperation = components['schemas']['OnboardingOperation'];

	let { isReadOnly = false }: TabProps = $props();

	// Organization query for onboarding state
	const organizationQuery = useOrganizationQuery();
	let onboarding = $derived((organizationQuery.data?.onboarding ?? []) as OnboardingOperation[]);

	// Queries
	const tagsQuery = useTagsQuery();
	const groupsQuery = useGroupsQuery();
	const networksQuery = useNetworksQuery();
	// Load all hosts to populate services cache for GroupCard display
	const hostsQuery = useHostsQuery({ limit: 0 });
	useServicesCacheQuery();

	// Mutations
	const createGroupMutation = useCreateGroupMutation();
	const updateGroupMutation = useUpdateGroupMutation();
	const deleteGroupMutation = useDeleteGroupMutation();
	const bulkDeleteGroupsMutation = useBulkDeleteGroupsMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let groupsData = $derived(groupsQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);
	let isLoading = $derived(groupsQuery.isPending || hostsQuery.isPending);

	let showGroupEditor = $state(false);
	let editingGroup = $state<Group | null>(null);

	// Deep-link: open group editor from URL (handles both fresh open and entity switch)
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'group-editor',
			groupsData,
			showGroupEditor,
			editingGroup?.id
		);
		if (result !== undefined) {
			editingGroup = result;
			showGroupEditor = true;
		}
	});

	function handleCreateGroup() {
		editingGroup = null;
		showGroupEditor = true;
	}

	function handleEditGroup(group: Group) {
		editingGroup = group;
		showGroupEditor = true;
	}

	function handleDeleteGroup(group: Group) {
		if (confirm(common_confirmDeleteName({ name: group.name }))) {
			deleteGroupMutation.mutate(group.id);
		}
	}

	async function handleGroupCreate(data: Group) {
		try {
			await createGroupMutation.mutateAsync(data);
			showGroupEditor = false;
			editingGroup = null;
		} catch {
			// Error handled by mutation
		}
	}

	async function handleGroupUpdate(id: string, data: Group) {
		try {
			await updateGroupMutation.mutateAsync(data);
			showGroupEditor = false;
			editingGroup = null;
		} catch {
			// Error handled by mutation
		}
	}

	function handleCloseGroupEditor() {
		showGroupEditor = false;
		editingGroup = null;
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(groups_confirmBulkDelete({ count: ids.length }))) {
			await bulkDeleteGroupsMutation.mutateAsync(ids);
		}
	}

	function getGroupTags(group: Group): string[] {
		return group.tags;
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('Group', {});
	}

	// Define field configuration for the DataTableControls
	// Uses defineFields to ensure all GroupOrderField values are covered
	let groupFields = $derived(
		defineFields<Group, GroupOrderField>(
			{
				name: { label: common_name(), type: 'string', searchable: true },
				group_type: {
					label: groups_groupType(),
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
	<TabHeader title={common_groupsLabel()} subtitle={groups_subtitle()}>
		<svelte:fragment slot="actions">
			{#if hasDaemon(onboarding) && !isReadOnly}
				<button class="btn-primary flex items-center" onclick={handleCreateGroup}
					><Plus class="h-5 w-5" />{common_create()}</button
				>
			{/if}
		</svelte:fragment>
	</TabHeader>

	{#if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title="Install a daemon to start organizing groups on your network." />
	{:else if isLoading}
		<Loading />
	{:else if groupsData.length === 0}
		<!-- Empty state -->
		<EmptyState
			title={groups_noGroupsYet()}
			subtitle={groups_noGroupsHelp()}
			onClick={handleCreateGroup}
			cta={common_create()}
		/>
	{:else}
		<DataControls
			items={groupsData}
			fields={groupFields}
			storageKey="scanopy-groups-table-state"
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			entityType={isReadOnly ? undefined : 'Group'}
			getItemTags={getGroupTags}
			getItemId={(item) => item.id}
			onCsvExport={handleCsvExport}
		>
			{#snippet children(
				item: Group,
				viewMode: 'card' | 'list',
				isSelected: boolean,
				onSelectionChange: (selected: boolean) => void
			)}
				<GroupCard
					group={item}
					selected={isSelected}
					{onSelectionChange}
					{viewMode}
					onEdit={isReadOnly ? undefined : () => handleEditGroup(item)}
					onDelete={isReadOnly ? undefined : () => handleDeleteGroup(item)}
				/>
			{/snippet}
		</DataControls>
	{/if}
</div>

<!-- Modal -->
<GroupEditModal
	name="group-editor"
	isOpen={showGroupEditor}
	group={editingGroup}
	onCreate={handleGroupCreate}
	onUpdate={handleGroupUpdate}
	onClose={handleCloseGroupEditor}
	onDelete={editingGroup
		? () => {
				handleDeleteGroup(editingGroup!);
				handleCloseGroupEditor();
			}
		: null}
/>
