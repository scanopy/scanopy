<script lang="ts">
	import CreateButton from '$lib/shared/components/data/controls/CreateButton.svelte';
	import {
		useTagsQuery,
		useTagsListQuery,
		useTagGroupsQuery,
		useCreateTagMutation,
		useUpdateTagMutation,
		useDeleteTagMutation,
		useBulkDeleteTagsMutation
	} from '../queries';
	import TagEditModal from './TagEditModal.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import LoadFailedState from '$lib/shared/components/layout/LoadFailedState.svelte';
	import { loadFailure } from '$lib/shared/utils/load-failure';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import type { Tag } from '../types/base';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { TableDefaults } from '$lib/shared/components/data/types';
	import type { CardAction } from '$lib/shared/components/data/types';
	import { tagColumns } from './TagTab.columns';
	import { collectionLookup } from '$lib/shared/entity-fields';
	import { Trash2, Edit } from '@lucide/svelte';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import type { TabProps } from '$lib/shared/types';
	import { applicationGroupId } from '../groups';
	import type { components } from '$lib/api/schema';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { listPageState } from '$lib/shared/components/data/listPage.svelte';
	import { useFieldValuesQuery } from '$lib/shared/api/field-values';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import {
		common_confirmBulkDelete,
		common_confirmDeleteName,
		common_create,
		common_delete,
		common_edit,
		common_noEntityYet,
		common_tags,
		tags_noTagsHelp,
		tags_subtitle
	} from '$lib/paraglide/messages';

	type TagOrderField = components['schemas']['TagOrderField'];

	import { refusedUnless, useCan } from '$lib/features/auth/access';

	let { isActive = false }: TabProps = $props();
	const can = useCan();

	let showTagEditor = $state(false);
	let editingTag: Tag | null = $state(null);

	// Deep-link: open tag editor from URL (handles both fresh open and entity switch)
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'tag-editor',
			tags,
			showTagEditor,
			editingTag?.id
		);
		if (result !== undefined) {
			editingTag = result;
			showTagEditor = true;
		}
	});

	// Queries and mutations
	const organizationQuery = useOrganizationQuery();

	// Every tag, for the empty state and deep links; the table reads its own page.
	const tagsQuery = useTagsQuery();
	const page = listPageState(() => isActive);
	const tagsPageQuery = useTagsListQuery(() => page.params());
	const TAG_FIELD_VALUES = '/api/v1/tags/field-values/{field}';
	const colorValuesQuery = useFieldValuesQuery(
		TAG_FIELD_VALUES,
		'color',
		undefined,
		() => isActive
	);
	const iconValuesQuery = useFieldValuesQuery(TAG_FIELD_VALUES, 'icon', undefined, () => isActive);
	const tagGroupsQuery = useTagGroupsQuery();
	let tagGroups = $derived(tagGroupsQuery.data ?? []);
	let applicationGroup = $derived(applicationGroupId(tagGroups));
	const createTagMutation = useCreateTagMutation();
	const updateTagMutation = useUpdateTagMutation();
	const deleteTagMutation = useDeleteTagMutation();
	const bulkDeleteTagsMutation = useBulkDeleteTagsMutation();

	// Derived state
	let tags = $derived(tagsQuery.data ?? []);
	let pageTags = $derived(tagsPageQuery.data?.items ?? []);
	let tagsPagination = $derived(tagsPageQuery.data?.pagination ?? null);
	let isLoading = $derived(tagsQuery.isLoading || (tagsPageQuery.isPending && !tagsPageQuery.data));
	let pageLoad = $derived(loadFailure(organizationQuery, tagsQuery, tagsPageQuery));

	let canManage = $derived(can('Tag', 'Write'));

	function handleCreateTag() {
		editingTag = null;
		showTagEditor = true;
	}

	/** Row actions. */
	function tagActions(tag: Tag): CardAction[] {
		if (!canManage) return [];

		return [
			{ label: common_edit(), icon: Edit, isOpen: true, onClick: () => handleEditTag(tag) },
			{
				label: common_delete(),
				icon: Trash2,
				tone: 'danger',
				isDelete: true,
				onBulk: handleBulkDelete,
				onClick: () => handleDeleteTag(tag),
				unavailable: refusedUnless(can('Tag', 'Delete'))
			}
		];
	}

	function handleEditTag(tag: Tag) {
		editingTag = tag;
		showTagEditor = true;
	}

	async function handleDeleteTag(tag: Tag) {
		if (confirm(common_confirmDeleteName({ name: tag.name }))) {
			await deleteTagMutation.mutateAsync(tag.id);
		}
	}

	async function handleTagCreate(data: Tag) {
		await createTagMutation.mutateAsync(data);
		showTagEditor = false;
		editingTag = null;
	}

	async function handleTagUpdate(_id: string, data: Tag) {
		await updateTagMutation.mutateAsync(data);
		showTagEditor = false;
		editingTag = null;
	}

	function handleCloseTagEditor() {
		showTagEditor = false;
		editingTag = null;
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(common_confirmBulkDelete({ count: ids.length, entity: common_tags() }))) {
			await bulkDeleteTagsMutation.mutateAsync(ids);
		}
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('Tag', page.listQuery);
	}

	const tableDefaults: TableDefaults<TagOrderField> = {
		group: 'tag_group',
		// Within a group, the group's own order.
		sort: { field: 'tag_group', direction: 'asc' }
	};

	let lookup = $derived(collectionLookup({ TagGroup: tagGroups }));
	let tagFields = $derived(
		tagColumns({
			lookup: () => lookup,
			tagGroups: () => tagGroups,
			applicationGroup: () => applicationGroup,
			colorValues: () => colorValuesQuery.data,
			iconValues: () => iconValuesQuery.data
		})
	);
</script>

<!-- The page's own actions, last in the table toolbar beside the filter and columns. -->
{#snippet toolbarActions()}
	{#if canManage}
		<CreateButton onclick={handleCreateTag} />
	{/if}
{/snippet}

<div class="space-y-6">
	{#if pageLoad.failed}
		<LoadFailedState onRetry={pageLoad.retry} />
	{:else if isLoading}
		<Loading />
	{:else if tags.length === 0}
		<EmptyState
			title={common_noEntityYet({ entity: common_tags() })}
			subtitle={tags_noTagsHelp()}
			onClick={canManage ? handleCreateTag : undefined}
			cta={canManage ? common_create() : ''}
		/>
	{:else}
		<DataControls
			title={common_tags()}
			subtitle={tags_subtitle()}
			{toolbarActions}
			items={pageTags}
			fields={tagFields}
			storageKey="scanopy-tags-table-state"
			defaults={tableDefaults}
			getItemId={(item) => item.id}
			serverPagination={tagsPagination}
			onPageChange={page.onPageChange}
			onQueryChange={page.onQueryChange}
			serverQuery={{ stale: false }}
			onCsvExport={handleCsvExport}
			getActions={tagActions}
			entityLabel={common_tags()}
		></DataControls>
	{/if}
</div>

<TagEditModal
	name="tag-editor"
	isOpen={showTagEditor}
	tag={editingTag}
	onCreate={handleTagCreate}
	onUpdate={handleTagUpdate}
	onClose={handleCloseTagEditor}
	onDelete={editingTag ? () => handleDeleteTag(editingTag!) : null}
/>
