<script lang="ts">
	import { Plus } from 'lucide-svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { useTagsQuery, useCreateTagMutation } from '$lib/features/tags/queries';
	import { createDefaultTag } from '$lib/features/tags/types/base';
	import { createColorHelper, AVAILABLE_COLORS, type Color } from '$lib/shared/utils/styling';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { permissions, billingPlans } from '$lib/shared/stores/metadata';
	import { isApplicationTag, tagIcon, tagTooltip, withTagAdded } from '$lib/features/tags/groups';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';

	/**
	 * TagPicker supports two usage patterns:
	 *
	 * 1. Binding (preferred): Use when parent prop is bindable
	 *    <TagPicker bind:selectedTagIds={formData.tags} />
	 *
	 * 2. Callback: Use when parent prop isn't bindable (e.g., received via slot)
	 *    <TagPicker selectedTagIds={service.tags} onChange={(tags) => handleTagsChange(tags)} />
	 */
	let {
		selectedTagIds = $bindable([]),
		label = 'Tags',
		placeholder = 'Type to add tags...',
		disabled = false,
		onChange
	}: {
		selectedTagIds?: string[];
		label?: string;
		placeholder?: string;
		disabled?: boolean;
		onChange?: (tagIds: string[]) => void;
	} = $props();

	// Supports both bind: and onChange patterns
	function updateTags(newTagIds: string[]) {
		selectedTagIds = newTagIds;
		onChange?.(newTagIds);
	}

	let inputValue = $state('');
	let isFocused = $state(false);
	let inputElement: HTMLInputElement | undefined = $state();

	// Query and mutation
	const tagsQuery = useTagsQuery();
	const createTagMutation = useCreateTagMutation();
	const organizationQuery = useOrganizationQuery();
	const currentUserQuery = useCurrentUserQuery();

	// Derived state
	let tags = $derived(tagsQuery.data ?? []);
	let isCreating = $derived(createTagMutation.isPending);
	let organization = $derived(organizationQuery.data);
	let currentUser = $derived(currentUserQuery.data);

	// Demo orgs are read-only for non-owners (mirrors the credentials tab)
	let isDemoOrg = $derived(
		billingPlans.getMetadata(organization?.plan?.type ?? null).is_demo === true
	);
	let isNonOwnerInDemo = $derived(isDemoOrg && currentUser?.permissions !== 'Owner');

	// Check if user can create tags
	let canCreateTags = $derived(
		!isNonOwnerInDemo &&
			currentUser &&
			permissions.getMetadata(currentUser.permissions).manage_org_entities
	);

	// Check if typed value matches an existing tag name exactly
	let exactMatch = $derived(
		tags.some((t) => t.name.toLowerCase() === inputValue.trim().toLowerCase())
	);

	// Show create option if user typed something, can create, and no exact match exists
	let showCreateOption = $derived(inputValue.trim().length > 0 && canCreateTags && !exactMatch);

	// Get tag by ID, returns null if not found
	function getTag(id: string) {
		return tags.find((t) => t.id === id) ?? null;
	}

	// Filter available tags based on input and exclude already selected
	let availableTags = $derived(
		tags.filter(
			(tag) =>
				!selectedTagIds.includes(tag.id) &&
				tag.name.toLowerCase().includes(inputValue.toLowerCase())
		)
	);

	let showDropdown = $derived(isFocused && (availableTags.length > 0 || showCreateOption));

	function getRandomColor(): Color {
		return AVAILABLE_COLORS[Math.floor(Math.random() * AVAILABLE_COLORS.length)];
	}

	async function handleCreateTag() {
		if (!organization || isCreating) return;

		const name = inputValue.trim();
		if (!name) return;

		try {
			const newTag = createDefaultTag(organization.id);
			newTag.name = name;
			newTag.color = getRandomColor();

			const result = await createTagMutation.mutateAsync(newTag);
			updateTags([...selectedTagIds, result.id]);
			inputValue = '';
		} finally {
			inputElement?.focus();
		}
	}

	function addTag(tagId: string) {
		if (!selectedTagIds.includes(tagId)) {
			// A tag from a tag group replaces the one of that group already selected, as the
			// server would on save.
			updateTags(withTagAdded(selectedTagIds, tagId, tags));
		}
		inputValue = '';
		inputElement?.focus();
	}

	function removeTag(tagId: string) {
		updateTags(selectedTagIds.filter((id) => id !== tagId));
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			e.preventDefault();
			if (showCreateOption && availableTags.length === 0) {
				// No matches, create new tag
				handleCreateTag();
			} else if (availableTags.length > 0) {
				// Add first matching tag
				addTag(availableTags[0].id);
			} else if (showCreateOption) {
				// Create new tag
				handleCreateTag();
			}
		} else if (e.key === 'Backspace' && inputValue === '' && selectedTagIds.length > 0) {
			removeTag(selectedTagIds[selectedTagIds.length - 1]);
		} else if (e.key === 'Escape') {
			inputValue = '';
			inputElement?.blur();
		}
	}

	function handleBlur() {
		// Delay to allow click on dropdown item
		setTimeout(() => {
			isFocused = false;
		}, 150);
	}
</script>

<div class="space-y-2">
	{#if label}
		<div class="text-secondary block text-sm font-medium">{label}</div>
	{/if}

	<div class="relative">
		<!-- Input container with selected tags -->
		<div
			class="input-field flex h-[42px] items-center overflow-hidden p-0"
			class:opacity-50={disabled}
			class:cursor-not-allowed={disabled}
			class:border-blue-500={isFocused}
			class:outline-none={isFocused}
			class:ring-2={isFocused}
			class:ring-blue-500={isFocused}
		>
			<!-- Selected tags (horizontal scroll) -->
			<div class="flex shrink-0 items-center overflow-x-auto">
				{#each selectedTagIds as tagId (tagId)}
					{@const tag = getTag(tagId)}
					<span class="mx-1 shrink-0">
						<Tag
							label={tag?.name}
							color={tag?.color}
							icon={tagIcon(tag)}
							title={tagTooltip(tag)}
							isShiny={isApplicationTag(tag)}
							pill={true}
							removable={!disabled}
							onRemove={() => removeTag(tagId)}
						/>
					</span>
				{/each}
			</div>

			<!-- Text input -->
			<input
				bind:this={inputElement}
				bind:value={inputValue}
				type="text"
				placeholder={selectedTagIds.length === 0 ? placeholder : ''}
				{disabled}
				class="input-field border-0 bg-transparent ring-0"
				style="--tw-ring-shadow: none; --tw-ring-color: none;"
				onfocus={() => (isFocused = true)}
				onblur={handleBlur}
				onkeydown={handleKeydown}
			/>
		</div>

		<!-- Dropdown -->
		{#if showDropdown}
			<div
				class="select-dropdown absolute left-0 right-0 top-full z-50 mt-1 max-h-48 overflow-y-auto rounded-md shadow-lg"
			>
				<!-- Create new tag option -->
				{#if showCreateOption}
					<button
						type="button"
						class="select-option flex w-full items-center gap-2 border-b px-3 py-2 text-left text-sm transition-colors"
						style="border-color: var(--color-border)"
						onmousedown={handleCreateTag}
						disabled={isCreating}
					>
						<Plus class="h-4 w-4 shrink-0 text-green-400" />
						<span class="text-primary">
							{isCreating ? 'Creating...' : `Create "${inputValue.trim()}"`}
						</span>
					</button>
				{/if}

				<!-- Existing tags -->
				{#each availableTags as tag (tag.id)}
					{@const colorHelper = createColorHelper(tag.color)}
					<button
						type="button"
						class="select-option flex w-full items-center gap-2 px-3 py-2 text-left text-sm transition-colors"
						onmousedown={() => addTag(tag.id)}
					>
						<span class="h-3 w-3 shrink-0 rounded-full" style="background-color: {colorHelper.rgb};"
						></span>
						<span class="text-primary">{tag.name}</span>
						{#if tag.description}
							<span class="text-tertiary truncate text-xs">— {tag.description}</span>
						{/if}
					</button>
				{/each}
			</div>
		{/if}
	</div>
</div>
