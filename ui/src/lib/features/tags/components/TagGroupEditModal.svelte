<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '#lib/shared/components/forms/form-context.ts';
	import { required, max } from '#lib/shared/components/forms/validators.ts';
	import GenericModal from '#lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '#lib/shared/components/layout/ModalHeaderIcon.svelte';
	import EntityMetadataSection from '#lib/shared/components/forms/EntityMetadataSection.svelte';
	import TextInput from '#lib/shared/components/forms/input/TextInput.svelte';
	import TextArea from '#lib/shared/components/forms/input/TextArea.svelte';
	import RadioGroup from '#lib/shared/components/forms/input/RadioGroup.svelte';
	import ListManager from '#lib/shared/components/forms/selection/ListManager.svelte';
	import { TagDisplay } from '#lib/shared/components/forms/selection/display/TagDisplay.svelte';
	import {
		EntityTypeDisplay,
		type EntityTypeOption
	} from '#lib/shared/components/forms/selection/display/EntityTypeDisplay.svelte';
	import { entities } from '#lib/shared/stores/metadata.ts';
	import { useOrganizationQuery } from '#lib/features/organizations/queries.ts';
	import { pushError } from '#lib/shared/stores/feedback.ts';
	import type { components } from '#lib/api/schema';
	import { createDefaultTagGroup, type Tag as TagType, type TagGroup } from '../types/base';
	import { compareTagOrder, isSystemGroup } from '../groups';
	import { useTagsQuery } from '../queries';
	import type { ModalAction } from '#lib/shared/components/layout/modalActions.ts';
	import {
		common_cancel,
		common_couldNotLoadOrganization,
		common_create,
		common_delete,
		common_deleting,
		common_description,
		common_editName,
		common_entityTypes,
		common_name,
		common_saving,
		common_tags,
		common_update,
		tags_appliesTo,
		tags_appliesToEvery,
		tags_appliesToEveryHelp,
		tags_appliesToSome,
		tags_appliesToSomeHelp,
		tags_chooseEntityType,
		tags_createTagGroup,
		tags_groupDescriptionPlaceholder,
		tags_noEntityTypesChosen,
		tags_scopeComputedHelp,
		tags_groupNamePlaceholder,
		tags_noTagsInGroup,
		tags_orderByName,
		tags_orderByNameHelp,
		tags_orderSet,
		tags_orderSetHelp,
		tags_systemGroupFixed,
		tags_tagOrder
	} from '#lib/paraglide/messages.js';

	type EntityDiscriminants = components['schemas']['EntityDiscriminants'];
	type Scope = 'every' | 'some';
	type Ordering = 'set' | 'name';

	let {
		group = null,
		isOpen = false,
		onCreate,
		onUpdate,
		onClose,
		onDelete = null,
		name = undefined
	}: {
		group?: TagGroup | null;
		isOpen?: boolean;
		onCreate: (data: TagGroup) => Promise<void> | void;
		/** Saves the group, then its tag order when `tagIds` differs from the group's order. */
		onUpdate: (data: TagGroup, tagIds: string[] | null) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
		name?: string;
	} = $props();

	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);
	const tagsQuery = useTagsQuery();

	/** Every taggable entity type, from the entity metadata. */
	const eligibleTypes = (entities.getItems() as EntityTypeOption[]).filter(
		(item) => item.metadata?.is_taggable
	);
	/**
	 * A scope the server computes from entity metadata (Status, Visibility): shown as the list it
	 * returns, never edited.
	 */
	let scopeFixed = $derived(group?.scope_computed ?? false);

	let loading = $state(false);
	let deleting = $state(false);

	let isEditing = $derived(group !== null);
	let isSystem = $derived(isSystemGroup(group));
	let title = $derived(
		isEditing ? common_editName({ name: group?.name ?? '' }) : tags_createTagGroup()
	);

	// Scope, ordering, chosen types and tag order are source of truth here and synced into the
	// form: TanStack's field values aren't tracked by Svelte, and these change programmatically.
	let scope = $state<Scope>('every');
	let ordering = $state<Ordering>('set');
	let chosenTypes = $state<EntityDiscriminants[]>([]);
	let groupTags = $state<TagType[]>([]);
	let initialOrder = $state<string[]>([]);

	const form = createForm(() => ({
		defaultValues: createDefaultTagGroup(''),
		onSubmit: async ({ value }) => {
			if (!organization) {
				pushError(common_couldNotLoadOrganization());
				onClose();
				return;
			}
			if (scope === 'some' && chosenTypes.length === 0) {
				pushError(tags_chooseEntityType());
				return;
			}
			const data: TagGroup = {
				...(value as TagGroup),
				name: value.name.trim(),
				description: value.description?.trim() || null,
				entity_types: scope === 'every' ? null : [...chosenTypes],
				ordered: ordering === 'set',
				organization_id: organization.id
			};
			const order = groupTags.map((tag) => tag.id);
			const orderChanged = data.ordered && order.some((id, index) => id !== initialOrder[index]);

			loading = true;
			try {
				if (isEditing) await onUpdate(data, orderChanged ? order : null);
				else await onCreate(data);
			} finally {
				loading = false;
			}
		}
	}));

	function handleOpen() {
		const defaults = group ? { ...group } : createDefaultTagGroup(organization?.id ?? '');
		form.reset(defaults);
		scope = defaults.entity_types ? 'some' : 'every';
		ordering = defaults.ordered ? 'set' : 'name';
		chosenTypes = defaults.entity_types ? [...defaults.entity_types] : [];
		groupTags = group
			? (tagsQuery.data ?? []).filter((tag) => tag.tag_group_id === group.id).sort(compareTagOrder)
			: [];
		initialOrder = groupTags.map((tag) => tag.id);
	}

	function handleScopeChange(next: Scope) {
		scope = next;
		form.setFieldValue('entity_types', next === 'every' ? null : [...chosenTypes]);
	}

	function handleOrderingChange(next: Ordering) {
		ordering = next;
		form.setFieldValue('ordered', next === 'set');
		// An unordered group lists by name, which is also where a newly ordered one starts.
		groupTags = [...groupTags].sort((a, b) => a.name.localeCompare(b.name));
		if (next === 'name') initialOrder = groupTags.map((tag) => tag.id);
	}

	let chosenTypeItems = $derived(
		chosenTypes
			.map((type) => eligibleTypes.find((item) => item.id === type))
			.filter((item): item is EntityTypeOption => item !== undefined)
	);
	let typeOptions = $derived(
		eligibleTypes.filter((item) => !chosenTypes.includes(item.id as EntityDiscriminants))
	);

	function setChosenTypes(next: EntityDiscriminants[]) {
		chosenTypes = next;
		form.setFieldValue('entity_types', [...next]);
	}

	function addType(id: string) {
		setChosenTypes([...chosenTypes, id as EntityDiscriminants]);
	}

	function removeType(index: number) {
		setChosenTypes(chosenTypes.filter((_, i) => i !== index));
	}

	function move(fromIndex: number, toIndex: number) {
		if (toIndex < 0 || toIndex >= groupTags.length) return;
		const next = [...groupTags];
		[next[fromIndex], next[toIndex]] = [next[toIndex], next[fromIndex]];
		groupTags = next;
	}

	async function handleDelete() {
		if (!onDelete || !group) return;
		deleting = true;
		try {
			await onDelete(group.id);
		} finally {
			deleting = false;
		}
	}

	let actions = $derived<ModalAction[]>([
		...(isEditing && onDelete && !isSystem
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
			label: loading ? common_saving() : isEditing ? common_update() : common_create(),
			onClick: () => submitForm(form),
			disabled: loading || deleting
		}
	]);
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	entityId={group?.id}
	{form}
	size="xl"
	{onClose}
	onOpen={handleOpen}
	showCloseButton={true}
	{actions}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('TagGroup')} color="Yellow" />
	{/snippet}

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			submitForm(form);
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div class="min-h-0 flex-1 overflow-auto p-6">
			<div class="space-y-8">
				<div class="space-y-4">
					<form.Field
						name="name"
						validators={{ onBlur: ({ value }) => required(value) || max(100)(value) }}
					>
						{#snippet children(field)}
							<TextInput
								label={common_name()}
								id="name"
								{field}
								placeholder={tags_groupNamePlaceholder()}
								disabled={isSystem}
								helpText={isSystem ? tags_systemGroupFixed() : ''}
								required
							/>
						{/snippet}
					</form.Field>

					<form.Field
						name="description"
						validators={{ onBlur: ({ value }) => max(500)(value || '') }}
					>
						{#snippet children(field)}
							<TextArea
								label={common_description()}
								id="description"
								{field}
								placeholder={tags_groupDescriptionPlaceholder()}
							/>
						{/snippet}
					</form.Field>

					<form.Field name="entity_types">
						{#snippet children(field)}
							<div class="space-y-3">
								<RadioGroup
									label={tags_appliesTo()}
									id="entity_types"
									{field}
									value={scope}
									onChange={handleScopeChange}
									disabled={scopeFixed}
									options={[
										{
											value: 'every',
											label: tags_appliesToEvery(),
											helpText: tags_appliesToEveryHelp()
										},
										{
											value: 'some',
											label: tags_appliesToSome(),
											helpText: tags_appliesToSomeHelp()
										}
									]}
								/>
								{#if scope === 'some'}
									<ListManager
										label={common_entityTypes()}
										emptyMessage={tags_noEntityTypesChosen()}
										helpText={scopeFixed ? tags_scopeComputedHelp() : ''}
										items={chosenTypeItems}
										options={typeOptions}
										itemDisplayComponent={EntityTypeDisplay}
										optionDisplayComponent={EntityTypeDisplay}
										allowAddFromOptions={!scopeFixed}
										allowReorder={false}
										allowItemEdit={false}
										allowItemRemove={!scopeFixed}
										onAdd={addType}
										onRemove={removeType}
									/>
								{/if}
							</div>
						{/snippet}
					</form.Field>

					<!-- Fixed for a system group: statuses have an order, applications don't. -->
					<form.Field name="ordered">
						{#snippet children(field)}
							<RadioGroup
								label={tags_tagOrder()}
								id="ordered"
								{field}
								value={ordering}
								onChange={handleOrderingChange}
								disabled={isSystem}
								options={[
									{ value: 'set', label: tags_orderSet(), helpText: tags_orderSetHelp() },
									{ value: 'name', label: tags_orderByName(), helpText: tags_orderByNameHelp() }
								]}
							/>
						{/snippet}
					</form.Field>
				</div>

				{#if isEditing}
					<ListManager
						label={common_tags()}
						emptyMessage={tags_noTagsInGroup()}
						items={groupTags}
						itemDisplayComponent={TagDisplay}
						optionDisplayComponent={TagDisplay}
						allowAddFromOptions={false}
						allowReorder={ordering === 'set'}
						allowItemEdit={false}
						allowItemRemove={false}
						onMoveUp={move}
						onMoveDown={move}
					/>
				{/if}
			</div>
		</div>

		{#if isEditing && group}
			<EntityMetadataSection entities={[group]} />
		{/if}
	</form>
</GenericModal>
