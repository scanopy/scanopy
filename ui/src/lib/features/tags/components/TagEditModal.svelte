<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, max } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import type { Tag } from '../types/base';
	import { createDefaultTag } from '../types/base';
	import { createColorHelper, AVAILABLE_COLORS } from '$lib/shared/utils/styling';
	import { TagIcon } from 'lucide-svelte';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { pushError } from '$lib/shared/stores/feedback';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';
	import IconPicker from '$lib/shared/components/forms/IconPicker.svelte';
	import TagGroupSelect from './TagGroupSelect.svelte';
	import { useTagsQuery } from '../queries';
	import { groupNames, type TagGroup } from '../groups';
	import tagIconsFixture from '$lib/data/tag-icons.json';
	import {
		common_cancel,
		common_color,
		common_couldNotLoadOrganization,
		common_create,
		common_delete,
		common_deleting,
		common_description,
		common_details,
		common_editName,
		common_icon,
		common_name,
		common_saving,
		common_update,
		tags_applicationHelp,
		tags_createTag,
		tags_descriptionPlaceholder,
		tags_tagGroup,
		tags_tagGroupHelp,
		tags_tagGroupPlaceholder,
		tags_iconApplicationFixed,
		tags_tagNamePlaceholder
	} from '$lib/paraglide/messages';

	let {
		tag = null,
		isOpen = false,
		onCreate,
		onUpdate,
		onClose,
		onDelete = null,
		name = undefined
	}: {
		tag?: Tag | null;
		isOpen?: boolean;
		onCreate: (data: Tag) => Promise<void> | void;
		onUpdate: (id: string, data: Tag) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
		name?: string;
	} = $props();

	// TanStack Query for organization
	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	// Every tag, for the names of the tag groups already in use.
	const tagsQuery = useTagsQuery();
	let allTags = $derived(tagsQuery.data ?? []);
	const tagIconNames: string[] = tagIconsFixture;

	let loading = $state(false);
	let deleting = $state(false);

	let isEditing = $derived(tag !== null);
	let title = $derived(isEditing ? common_editName({ name: tag?.name ?? '' }) : tags_createTag());
	let saveLabel = $derived(isEditing ? common_update() : common_create());

	function getDefaultValues(): Tag {
		if (tag) return { ...tag };
		if (organization) return createDefaultTag(organization.id);
		return createDefaultTag('');
	}

	// Create form
	const form = createForm(() => ({
		defaultValues: createDefaultTag(''),
		onSubmit: async ({ value }) => {
			if (!organization) {
				pushError(common_couldNotLoadOrganization());
				onClose();
				return;
			}

			const tagData: Tag = {
				...(value as Tag),
				name: value.name.trim(),
				description: value.description?.trim() || null,
				organization_id: organization.id
			};

			loading = true;
			try {
				if (isEditing && tag) {
					await onUpdate(tag.id, tagData);
				} else {
					await onCreate(tagData);
				}
			} finally {
				loading = false;
			}
		}
	}));

	// The group and icon are source of truth here and synced into the form: switching to the
	// Application group clears the icon programmatically, which the form store alone would not show.
	let selectedGroup = $state<TagGroup | null>(null);
	let selectedIcon = $state<string | null>(null);

	function handleGroupChange(group: TagGroup | null) {
		selectedGroup = group;
		form.setFieldValue('tag_group', group);
		if (group?.type === 'Application') handleIconChange(null);
	}

	function handleIconChange(icon: string | null) {
		selectedIcon = icon;
		form.setFieldValue('icon', icon);
	}

	// Reset form when modal opens
	function handleOpen() {
		const defaults = getDefaultValues();
		form.reset(defaults);
		selectedGroup = defaults.tag_group ?? null;
		selectedIcon = defaults.icon ?? null;
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	async function handleDelete() {
		if (onDelete && tag) {
			deleting = true;
			try {
				await onDelete(tag.id);
			} finally {
				deleting = false;
			}
		}
	}

	let colorHelper = $derived(createColorHelper(form.state.values.color));
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	entityId={tag?.id}
	size="xl"
	{onClose}
	onOpen={handleOpen}
	showCloseButton={true}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={TagIcon} color={colorHelper.color} />
	{/snippet}

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			handleSubmit();
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div class="min-h-0 flex-1 overflow-auto p-6">
			<div class="space-y-8">
				<!-- Tag Details Section -->
				<div class="space-y-4">
					<h3 class="text-primary text-lg font-medium">{common_details()}</h3>

					<form.Field
						name="name"
						validators={{
							onBlur: ({ value }) => required(value) || max(100)(value)
						}}
					>
						{#snippet children(field)}
							<TextInput
								label={common_name()}
								id="name"
								{field}
								placeholder={tags_tagNamePlaceholder()}
								required
							/>
						{/snippet}
					</form.Field>

					<form.Field
						name="description"
						validators={{
							onBlur: ({ value }) => max(500)(value || '')
						}}
					>
						{#snippet children(field)}
							<TextArea
								label={common_description()}
								id="description"
								{field}
								placeholder={tags_descriptionPlaceholder()}
							/>
						{/snippet}
					</form.Field>

					<!-- Tag group. Held in $state: choosing Application clears the icon, a
					     programmatic write TanStack Form would not re-render. -->
					<form.Field name="tag_group">
						{#snippet children()}
							<div class="space-y-2">
								<label for="tag_group" class="text-secondary block text-sm font-medium">
									{tags_tagGroup()}
								</label>
								<TagGroupSelect
									id="tag_group"
									value={selectedGroup}
									groups={groupNames(allTags)}
									placeholder={tags_tagGroupPlaceholder()}
									onChange={handleGroupChange}
								/>
								<p class="text-tertiary text-xs">
									{selectedGroup?.type === 'Application'
										? tags_applicationHelp()
										: tags_tagGroupHelp()}
								</p>
							</div>
						{/snippet}
					</form.Field>

					<form.Field name="icon">
						{#snippet children()}
							<IconPicker
								id="icon"
								label={common_icon()}
								value={selectedGroup?.type === 'Application' ? null : selectedIcon}
								icons={tagIconNames}
								disabled={selectedGroup?.type === 'Application'}
								helpText={selectedGroup?.type === 'Application' ? tags_iconApplicationFixed() : ''}
								onChange={handleIconChange}
							/>
						{/snippet}
					</form.Field>

					<!-- Color Selector -->
					<form.Field name="color">
						{#snippet children(field)}
							<div class="space-y-2">
								<div class="text-secondary block text-sm font-medium">{common_color()}</div>
								<div class="flex flex-wrap gap-1.5">
									{#each AVAILABLE_COLORS as color (color)}
										{@const ch = createColorHelper(color)}
										<button
											type="button"
											onclick={() => field.handleChange(color)}
											class="group relative h-7 w-7 rounded-md border-2 transition-all hover:scale-110"
											class:border-gray-500={field.state.value !== color}
											class:border-white={field.state.value === color}
											class:ring-2={field.state.value === color}
											class:ring-white={field.state.value === color}
											style="background-color: {ch.rgb};"
											title={color}
										></button>
									{/each}
								</div>
							</div>
						{/snippet}
					</form.Field>
				</div>
			</div>
		</div>

		{#if isEditing && tag}
			<EntityMetadataSection entities={[tag]} />
		{/if}

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex items-center justify-between">
				<div>
					{#if isEditing && onDelete}
						<button
							type="button"
							disabled={deleting || loading}
							onclick={handleDelete}
							class="btn-danger"
						>
							{deleting ? common_deleting() : common_delete()}
						</button>
					{/if}
				</div>
				<div class="flex items-center gap-3">
					<button
						type="button"
						disabled={loading || deleting}
						onclick={onClose}
						class="btn-secondary"
					>
						{common_cancel()}
					</button>
					<button type="submit" disabled={loading || deleting} class="btn-primary">
						{loading ? common_saving() : saveLabel}
					</button>
				</div>
			</div>
		</div>
	</form>
</GenericModal>
