<script lang="ts">
	import { collectionLookup } from '$lib/shared/entity-fields';
	import CreateButton from '$lib/shared/components/data/controls/CreateButton.svelte';
	import {
		useTagsQuery,
		useTagGroupsQuery,
		useTagGroupsListQuery,
		useCreateTagGroupMutation,
		useUpdateTagGroupMutation,
		useDeleteTagGroupMutation,
		useBulkDeleteTagGroupsMutation,
		useSetTagOrderMutation
	} from '../queries';
	import TagGroupEditModal from './TagGroupEditModal.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import LoadFailedState from '$lib/shared/components/layout/LoadFailedState.svelte';
	import { loadFailure } from '$lib/shared/utils/load-failure';
	import type { TagGroup } from '../types/base';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import { disabledWhen, type RowAction } from '$lib/shared/components/data/types';
	import { listPageState } from '$lib/shared/components/data/listPage.svelte';
	import { Trash2, Edit } from '@lucide/svelte';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import type { TabProps } from '$lib/shared/types';
	import { isSystemGroup } from '../groups';
	import { tagGroupColumns } from './TagGroupTab.columns';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import {
		common_confirmBulkDelete,
		common_confirmDeleteName,
		common_delete,
		common_edit,
		tags_tagGroups,
		tags_tagGroupsSubtitle,
		tags_systemGroupFixed,
		tags_systemGroupFixedBulk
	} from '$lib/paraglide/messages';

	import { refusedUnless, useCan } from '$lib/features/auth/access';

	let { isActive = false }: TabProps = $props();
	const can = useCan();

	let showEditor = $state(false);
	let editingGroup: TagGroup | null = $state(null);

	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'tag-group-editor',
			groups,
			showEditor,
			editingGroup?.id
		);
		if (result !== undefined) {
			editingGroup = result;
			showEditor = true;
		}
	});

	const organizationQuery = useOrganizationQuery();

	const tagsQuery = useTagsQuery();
	// Every group, for the tag chips' group labels and deep links; the table reads its own page.
	const tagGroupsQuery = useTagGroupsQuery();
	const page = listPageState(() => isActive);
	const groupsPageQuery = useTagGroupsListQuery(() => page.params());
	const createMutation = useCreateTagGroupMutation();
	const updateMutation = useUpdateTagGroupMutation();
	const deleteMutation = useDeleteTagGroupMutation();
	const bulkDeleteMutation = useBulkDeleteTagGroupsMutation();
	const setTagOrderMutation = useSetTagOrderMutation();

	let tags = $derived(tagsQuery.data ?? []);
	let groups = $derived(tagGroupsQuery.data ?? []);
	let pageGroups = $derived(groupsPageQuery.data?.items ?? []);
	let groupsPagination = $derived(groupsPageQuery.data?.pagination ?? null);
	let isLoading = $derived(groupsPageQuery.isPending && !groupsPageQuery.data);
	let pageLoad = $derived(loadFailure(organizationQuery, groupsPageQuery));

	let canManage = $derived(can('Tag', 'Write'));

	function handleCreate() {
		editingGroup = null;
		showEditor = true;
	}

	function handleEdit(group: TagGroup) {
		editingGroup = group;
		showEditor = true;
	}

	function close() {
		showEditor = false;
		editingGroup = null;
	}

	async function handleDelete(group: TagGroup) {
		if (confirm(common_confirmDeleteName({ name: group.name }))) {
			await deleteMutation.mutateAsync(group.id);
			close();
		}
	}

	function actions(group: TagGroup): RowAction[] {
		if (!canManage) return [];
		return [
			{ label: common_edit(), icon: Edit, isOpen: true, onClick: () => handleEdit(group) },
			{
				label: common_delete(),
				icon: Trash2,
				tone: 'danger',
				isDelete: true,
				onBulk: handleBulkDelete,
				onClick: () => handleDelete(group),
				unavailable:
					refusedUnless(can('Tag', 'Delete')) ??
					disabledWhen(isSystemGroup(group), tags_systemGroupFixed(), tags_systemGroupFixedBulk())
			}
		];
	}

	async function handleCreateSubmit(data: TagGroup) {
		await createMutation.mutateAsync(data);
		close();
	}

	async function handleUpdateSubmit(data: TagGroup, tagIds: string[] | null) {
		await updateMutation.mutateAsync(data);
		if (tagIds) await setTagOrderMutation.mutateAsync({ groupId: data.id, tagIds });
		close();
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(common_confirmBulkDelete({ count: ids.length, entity: tags_tagGroups() }))) {
			await bulkDeleteMutation.mutateAsync(ids);
		}
	}

	// The registry's tag group fields reach no related entity.
	const lookup = collectionLookup({});

	let fields = $derived(
		tagGroupColumns({
			lookup: () => lookup,
			tags: () => tags,
			groups: () => groups
		})
	);
</script>

{#snippet toolbarActions()}
	{#if canManage}
		<CreateButton onclick={handleCreate} />
	{/if}
{/snippet}

<div class="space-y-6">
	{#if pageLoad.failed}
		<LoadFailedState onRetry={pageLoad.retry} />
	{:else if isLoading}
		<Loading />
	{:else}
		<DataControls
			title={tags_tagGroups()}
			subtitle={tags_tagGroupsSubtitle()}
			{toolbarActions}
			items={pageGroups}
			{fields}
			storageKey="scanopy-tag-groups-table-state"
			getItemId={(item) => item.id}
			serverPagination={groupsPagination}
			onPageChange={page.onPageChange}
			onQueryChange={page.onQueryChange}
			serverQuery={{ stale: false }}
			getActions={actions}
			entityLabel={tags_tagGroups()}
		></DataControls>
	{/if}
</div>

<TagGroupEditModal
	name="tag-group-editor"
	isOpen={showEditor}
	group={editingGroup}
	onCreate={handleCreateSubmit}
	onUpdate={handleUpdateSubmit}
	onClose={close}
	onDelete={editingGroup && !isSystemGroup(editingGroup) ? () => handleDelete(editingGroup!) : null}
/>
