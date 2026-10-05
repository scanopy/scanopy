<script lang="ts">
	import {
		useTagsQuery,
		useCreateTagMutation,
		useUpdateTagMutation,
		useDeleteTagMutation,
		useBulkDeleteTagsMutation
	} from '../queries';
	import TagEditModal from './TagEditModal.svelte';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import type { Tag } from '../types/base';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import { defineFields, type CardAction } from '$lib/shared/components/data/types';
	import { Plus, Trash2, Edit, Tag as TagIcon } from 'lucide-svelte';
	import { createColorHelper } from '$lib/shared/utils/styling';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { permissions, billingPlans, concepts } from '$lib/shared/stores/metadata';
	import type { TabProps } from '$lib/shared/types';
	import { exclusiveSetLabel, isApplicationTag, tagIcon } from '../sets';
	import type { components } from '$lib/api/schema';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import {
		common_color,
		common_confirmBulkDelete,
		common_confirmDeleteName,
		common_create,
		common_created,
		common_delete,
		common_edit,
		common_description,
		common_name,
		common_noEntityYet,
		common_tags,
		common_updated,
		tags_exclusiveSet,
		tags_noTagsHelp,
		tags_subtitle
	} from '$lib/paraglide/messages';

	type TagOrderField = components['schemas']['TagOrderField'];

	let { isReadOnly = false }: TabProps = $props();

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
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	// Demo orgs are read-only for non-owners (mirrors the credentials tab)
	let isDemoOrg = $derived(
		billingPlans.getMetadata(organization?.plan?.type ?? null).is_demo === true
	);
	let isNonOwnerInDemo = $derived(isDemoOrg && currentUser?.permissions !== 'Owner');

	const tagsQuery = useTagsQuery();
	const createTagMutation = useCreateTagMutation();
	const updateTagMutation = useUpdateTagMutation();
	const deleteTagMutation = useDeleteTagMutation();
	const bulkDeleteTagsMutation = useBulkDeleteTagsMutation();

	// Derived state
	let tags = $derived(tagsQuery.data ?? []);
	let isLoading = $derived(tagsQuery.isLoading);

	let canManage = $derived(
		!isReadOnly &&
			!isNonOwnerInDemo &&
			currentUser &&
			permissions.getMetadata(currentUser.permissions).manage_org_entities
	);

	let allowBulkDelete = $derived(
		!isReadOnly && !isNonOwnerInDemo && currentUser
			? permissions.getMetadata(currentUser.permissions).manage_org_entities
			: false
	);

	function handleCreateTag() {
		editingTag = null;
		showTagEditor = true;
	}

	/** Row actions for table mode, matching what the card offers. */
	function tagActions(tag: Tag): CardAction[] {
		if (!canManage) return [];

		return [
			{ label: common_edit(), icon: Edit, onClick: () => handleEditTag(tag) },
			{
				label: common_delete(),
				icon: Trash2,
				class: 'btn-icon-danger',
				onClick: () => handleDeleteTag(tag)
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
		await downloadCsv('Tag', {});
	}

	// Define field configuration for the DataTableControls
	// Uses defineFields to ensure all TagOrderField values are covered
	const tagFields = defineFields<Tag, TagOrderField>(
		{
			// Identity field: grouping by it would render a header per tag.
			name: {
				label: common_name(),
				type: 'string',
				searchable: true,
				groupable: false,
				display: { primary: true, width: 220 }
			},
			color: {
				label: common_color(),
				type: 'string',
				searchable: true,
				filterable: true,
				// A colour name is worth showing in its own colour.
				display: {
					getItems: (tag) => [
						{
							id: tag.color,
							label: tag.color.charAt(0).toUpperCase() + tag.color.slice(1),
							color: tag.color
						}
					]
				}
			},
			exclusive_set: {
				label: tags_exclusiveSet(),
				type: 'string',
				filterable: true,
				getValue: (tag) => exclusiveSetLabel(tag.exclusive_set)
			},
			created_at: { label: common_created(), type: 'date', display: { hiddenByDefault: true } },
			updated_at: { label: common_updated(), type: 'date', display: { hiddenByDefault: true } }
		},
		[{ key: 'description', label: common_description(), type: 'string', searchable: true }]
	);
</script>

<div class="space-y-6">
	<TabHeader title={common_tags()} subtitle={tags_subtitle()}>
		<svelte:fragment slot="actions">
			{#if canManage}
				<button class="btn-primary flex items-center" onclick={handleCreateTag}>
					<Plus class="h-5 w-5" />{common_create()}
				</button>
			{/if}
		</svelte:fragment>
	</TabHeader>

	{#if isLoading}
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
			items={tags}
			fields={tagFields}
			{allowBulkDelete}
			storageKey="scanopy-tags-table-state"
			onBulkDelete={handleBulkDelete}
			getItemId={(item) => item.id}
			getIcon={(tag) => ({
				icon: tagIcon(tag) ?? TagIcon,
				color: isApplicationTag(tag)
					? concepts.getColorHelper('Application')?.icon
					: createColorHelper(tag.color).icon
			})}
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
