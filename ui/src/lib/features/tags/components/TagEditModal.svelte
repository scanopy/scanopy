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
	import { TagIcon } from '@lucide/svelte';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { pushError } from '$lib/shared/stores/feedback';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';
	import IconPicker from '$lib/shared/components/forms/IconPicker.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import {
		NO_TAG_GROUP,
		TagGroupOptionDisplay
	} from '$lib/shared/components/forms/selection/display/TagGroupDisplay.svelte';
	import { useTagGroupsQuery } from '../queries';
	import tagIconsFixture from '$lib/data/tag-icons.json';
	import type { ModalAction } from '$lib/shared/components/layout/modalActions';
	import {
		common_cancel,
		common_color,
		common_couldNotLoadOrganization,
		common_create,
		common_delete,
		common_deleting,
		common_description,
		common_editName,
		common_icon,
		common_name,
		common_saving,
		common_update,
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

	const tagGroupsQuery = useTagGroupsQuery();
	let tagGroups = $derived(tagGroupsQuery.data ?? []);
	const tagIconNames: string[] = tagIconsFixture;

	let loading = $state(false);
	let deleting = $state(false);

	let isEditing = $derived(tag !== null);
	let title = $derived(isEditing ? common_editName({ name: tag?.name ?? '' }) : tags_createTag());
	let saveLabel = $derived(isEditing ? common_update() : common_create());

	// The form leaves out `source`: the server sets it, and its nested union is too deep for the
	// form's field-path types.
	type TagFormValues = Omit<Tag, 'source'>;
	function formValues(t: Tag): TagFormValues {
		// eslint-disable-next-line @typescript-eslint/no-unused-vars
		const { source, ...values } = t;
		return values;
	}

	function getDefaultValues(): TagFormValues {
		if (tag) return formValues(tag);
		if (organization) return formValues(createDefaultTag(organization.id));
		return formValues(createDefaultTag(''));
	}

	// Create form
	const form = createForm(() => ({
		defaultValues: formValues(createDefaultTag('')),
		onSubmit: async ({ value }) => {
			if (!organization) {
				pushError(common_couldNotLoadOrganization());
				onClose();
				return;
			}

			const tagData: Tag = {
				...(value as TagFormValues),
				source: tag?.source ?? { type: 'Manual' },
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
	let selectedGroupId = $state<string | null>(null);
	let selectedGroup = $derived(tagGroups.find((group) => group.id === selectedGroupId) ?? null);
	let isApplicationGroup = $derived(selectedGroup?.group_type === 'Application');
	let selectedIcon = $state<string | null>(null);

	function handleGroupChange(groupId: string | null) {
		selectedGroupId = groupId;
		form.setFieldValue('tag_group_id', groupId);
		if (tagGroups.find((group) => group.id === groupId)?.group_type === 'Application')
			handleIconChange(null);
	}

	// The None row, which clears the group, is offered only while there is a group to clear.
	let groupOptions = $derived(selectedGroupId ? [null, ...tagGroups] : tagGroups);

	function handleIconChange(icon: string | null) {
		selectedIcon = icon;
		form.setFieldValue('icon', icon);
	}

	// Reset form when modal opens
	function handleOpen() {
		const defaults = getDefaultValues();
		form.reset(defaults);
		selectedGroupId = defaults.tag_group_id ?? null;
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

	// Mirrored from the form store: form.state.values is not tracked by $derived.
	let colorValue = $state(createDefaultTag('').color);
	$effect(() => {
		const { unsubscribe } = form.store.subscribe(() => {
			colorValue = form.state.values.color;
		});
		return unsubscribe;
	});
	let colorHelper = $derived(createColorHelper(colorValue));

	let actions = $derived<ModalAction[]>([
		...(isEditing && onDelete
			? [
					{
						role: 'danger' as const,
						label: deleting ? common_deleting() : common_delete(),
						onClick: handleDelete,
						disabled: deleting || loading
					}
				]
			: []),
		{ role: 'cancel', label: common_cancel(), onClick: onClose, disabled: loading || deleting },
		{
			role: 'primary',
			label: loading ? common_saving() : saveLabel,
			onClick: handleSubmit,
			disabled: loading || deleting
		}
	]);
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	entityId={tag?.id}
	{form}
	size="xl"
	{onClose}
	onOpen={handleOpen}
	showCloseButton={true}
	{actions}
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
					<form.Field name="tag_group_id">
						<RichSelect
							label={tags_tagGroup()}
							selectedValue={selectedGroupId}
							options={groupOptions}
							displayComponent={TagGroupOptionDisplay}
							placeholder={tags_tagGroupPlaceholder()}
							helpText={selectedGroup?.description || tags_tagGroupHelp()}
							showSearch
							onSelect={(id) => handleGroupChange(id === NO_TAG_GROUP ? null : id)}
						/>
					</form.Field>

					<!-- Icon and colour side by side: the icon grid is tall, and stacked it pushed the
					     colour swatches below the fold. -->
					<div class="grid grid-cols-1 items-start gap-4 sm:grid-cols-2">
						<form.Field name="icon">
							<IconPicker
								id="icon"
								label={common_icon()}
								value={isApplicationGroup ? null : selectedIcon}
								icons={tagIconNames}
								disabled={isApplicationGroup}
								helpText={isApplicationGroup ? tags_iconApplicationFixed() : ''}
								onChange={handleIconChange}
							/>
						</form.Field>

						<form.Field name="color">
							{#snippet children(field)}
								<div class="space-y-2">
									<div class="block text-sm font-medium text-secondary">{common_color()}</div>
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
		</div>

		{#if isEditing && tag}
			<EntityMetadataSection entities={[tag]} />
		{/if}
	</form>
</GenericModal>
