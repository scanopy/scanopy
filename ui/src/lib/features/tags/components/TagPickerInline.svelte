<script lang="ts">
	import { Plus } from 'lucide-svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import {
		useTagsQuery,
		useCreateTagMutation,
		useBulkAddTagMutation,
		useBulkRemoveTagMutation,
		type EntityDiscriminants
	} from '$lib/features/tags/queries';
	import { createDefaultTag } from '$lib/features/tags/types/base';
	import { AVAILABLE_COLORS, type Color } from '$lib/shared/utils/styling';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { permissions, billingPlans } from '$lib/shared/stores/metadata';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { onMount } from 'svelte';
	import { common_creating, tags_addTag, tags_createTagQuoted } from '$lib/paraglide/messages';
	import CreatableOptionList from '$lib/shared/components/forms/selection/CreatableOptionList.svelte';
	import { isApplicationTag, sameGroup, tagIcon, tagTooltip } from '$lib/features/tags/groups';

	/**
	 * Compact inline tag picker for use in cards and bulk actions.
	 * Shows tags as pills with remove buttons, and a + button to add tags.
	 *
	 * Two modes of operation:
	 * 1. **Entity mode**: Provide `entityId` and `entityType` - mutations handled internally
	 * 2. **Callback mode**: Provide `onAdd`/`onRemove` callbacks - for bulk operations or custom handling
	 */
	let {
		selectedTagIds = [],
		disabled = false,
		// Entity mode props - when provided, uses generic tag assignment API
		entityId,
		entityType,
		// Callback mode props - for bulk operations or custom handling
		onAdd,
		onRemove,
		// Optional pre-resolved tags (e.g. from topology snapshot on share pages)
		availableTags: availableTagsProp,
		// When true, tags created inline will have is_application set
		createAsApplication = false,
		// When false, hides the inline tag creation option
		allowCreate = true,
		// Bindable: mirrors the dropdown's open state. Set to true to open programmatically;
		// updates back to false when the dropdown closes (e.g. user blurs away).
		open = $bindable(false),
		// When true, hide the "+" affordance. The dropdown input still renders while `open`.
		hideAddButton = false
	}: {
		selectedTagIds?: string[];
		disabled?: boolean;
		entityId?: string;
		entityType?: EntityDiscriminants;
		onAdd?: (tagId: string) => void;
		onRemove?: (tagId: string) => void;
		availableTags?: import('$lib/features/tags/types/base').Tag[];
		createAsApplication?: boolean;
		allowCreate?: boolean;
		open?: boolean;
		hideAddButton?: boolean;
	} = $props();

	// Entity mode: use generic mutations
	const bulkAddTagMutation = useBulkAddTagMutation();
	const bulkRemoveTagMutation = useBulkRemoveTagMutation();

	// Determine if entity mode is enabled
	let isEntityMode = $derived(entityId !== undefined && entityType !== undefined);

	let inputValue = $state('');
	let inputElement: HTMLInputElement | undefined = $state();
	let triggerElement: HTMLDivElement | undefined = $state();
	let dropdownElement: HTMLDivElement | undefined = $state();
	let dropdownPosition = $state({ top: 0, left: 0 });

	// Portal container for escaping stacking contexts
	let portalContainer: HTMLDivElement | null = $state(null);

	onMount(() => {
		portalContainer = document.createElement('div');
		portalContainer.style.position = 'absolute';
		portalContainer.style.top = '0';
		portalContainer.style.left = '0';
		portalContainer.style.width = '0';
		portalContainer.style.height = '0';
		document.body.appendChild(portalContainer);

		return () => {
			portalContainer?.remove();
		};
	});

	// Portal action to move element to body
	function portal(node: HTMLElement) {
		if (portalContainer) {
			portalContainer.appendChild(node);
		}
		return {
			destroy() {}
		};
	}

	function calculatePosition() {
		if (!triggerElement) return;
		const rect = triggerElement.getBoundingClientRect();
		const viewportHeight = window.innerHeight;
		const viewportWidth = window.innerWidth;

		// Vertical: flip above if not enough space below
		let top: number;
		const dropdownHeight = dropdownElement?.getBoundingClientRect().height ?? 192; // max-h-48 = 12rem = 192px
		const spaceBelow = viewportHeight - rect.bottom;
		const spaceAbove = rect.top;

		if (spaceBelow >= dropdownHeight + 8 || spaceBelow >= spaceAbove) {
			top = rect.bottom + 4;
		} else {
			top = rect.top - dropdownHeight - 4;
		}

		// Horizontal: clamp to viewport edges
		let left = rect.left;
		left = Math.max(8, Math.min(left, viewportWidth - 160 - 8)); // min-w-40 = 160px

		dropdownPosition = { top, left };
	}

	// Recalculate position once dropdown renders (so we measure real height for flip)
	$effect(() => {
		if (open && dropdownElement) {
			calculatePosition();
		}
	});

	// Reposition dropdown on scroll when open
	$effect(() => {
		if (!open) return;

		const handleScroll = () => calculatePosition();
		window.addEventListener('scroll', handleScroll, true);
		return () => window.removeEventListener('scroll', handleScroll, true);
	});

	// Query and mutation
	const tagsQuery = useTagsQuery();
	const createTagMutation = useCreateTagMutation();
	const organizationQuery = useOrganizationQuery();
	const currentUserQuery = useCurrentUserQuery();

	// Derived state
	let tags = $derived(availableTagsProp ?? tagsQuery.data ?? []);
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
		allowCreate &&
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

	// Get tag by ID
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

	let showDropdown = $derived(open && (availableTags.length > 0 || showCreateOption));

	function getRandomColor(): Color {
		return AVAILABLE_COLORS[Math.floor(Math.random() * AVAILABLE_COLORS.length)];
	}

	async function handleCreateTag() {
		if (!organization || isCreating) return;
		if (!isEntityMode && !onAdd) return;

		const name = inputValue.trim();
		if (!name) return;

		try {
			const newTag = createDefaultTag(organization.id);
			newTag.name = name;
			newTag.color = getRandomColor();
			if (createAsApplication) {
				newTag.tag_group = { type: 'Application' };
			}

			const result = await createTagMutation.mutateAsync(newTag);
			await handleAddTag(result.id);
			inputValue = '';
			open = false;
		} finally {
			inputElement?.focus();
		}
	}

	async function handleAddTag(tagId: string) {
		if (isEntityMode && entityId && entityType) {
			await bulkAddTagMutation.mutateAsync({
				entity_ids: [entityId],
				entity_type: entityType,
				tag_id: tagId
			});
		} else {
			// The server replaces a held tag of the same tag group on assignment; in callback
			// mode the caller holds the list, so drop the displaced tag here to match.
			const added = getTag(tagId);
			for (const heldId of selectedTagIds) {
				if (heldId !== tagId && sameGroup(getTag(heldId)?.tag_group, added?.tag_group)) {
					onRemove?.(heldId);
				}
			}
			onAdd?.(tagId);
		}
		inputValue = '';
		open = false;
	}

	async function handleRemoveTag(tagId: string) {
		if (isEntityMode && entityId && entityType) {
			await bulkRemoveTagMutation.mutateAsync({
				entity_ids: [entityId],
				entity_type: entityType,
				tag_id: tagId
			});
		} else {
			onRemove?.(tagId);
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			e.preventDefault();
			if (showCreateOption && availableTags.length === 0) {
				handleCreateTag();
			} else if (availableTags.length > 0) {
				handleAddTag(availableTags[0].id);
			} else if (showCreateOption) {
				handleCreateTag();
			}
		} else if (e.key === 'Escape') {
			inputValue = '';
			open = false;
			inputElement?.blur();
		}
	}

	function handleBlur() {
		// Delay to allow click on dropdown item
		setTimeout(() => {
			open = false;
		}, 150);
	}

	function handleAddClick() {
		if (disabled) return;
		open = true;
	}

	// When the dropdown opens (via button click or parent programmatically setting `open`),
	// compute position and focus the input.
	$effect(() => {
		if (open && !disabled) {
			calculatePosition();
			setTimeout(() => inputElement?.focus(), 0);
		}
	});
</script>

<div class="flex min-w-0 flex-wrap items-center gap-1">
	<!-- Selected tags -->
	{#each selectedTagIds.filter((id) => getTag(id)) as tagId (tagId)}
		{@const tag = getTag(tagId)}
		<Tag
			label={tag?.name}
			color={tag?.color}
			icon={tagIcon(tag)}
			title={tagTooltip(tag)}
			isShiny={isApplicationTag(tag)}
			pill={!disabled}
			removable={!disabled && !!(onRemove || isEntityMode)}
			onRemove={() => handleRemoveTag(tagId)}
		/>
	{/each}

	<!-- Add button / dropdown (hide when no tags to add and creation disabled).
	     `hideAddButton` hides only the "+" affordance — the input still renders while `open`. -->
	{#if (onAdd || isEntityMode) && !disabled && (canCreateTags || tags.some((t) => !selectedTagIds.includes(t.id)))}
		<div bind:this={triggerElement} class="relative flex h-5 items-center">
			{#if open}
				<!-- Input for searching/creating tags -->
				<input
					bind:this={inputElement}
					bind:value={inputValue}
					type="text"
					placeholder={tags_addTag()}
					class="h-5 w-24 rounded-full px-2 text-xs focus:outline-none focus:ring-1 focus:ring-blue-500"
					style="border: 1px solid var(--color-border-input); background: var(--color-bg-input); color: var(--color-text-primary)"
					onfocus={() => (open = true)}
					onblur={handleBlur}
					onkeydown={handleKeydown}
				/>
			{:else if !hideAddButton}
				<!-- Add button -->
				<button
					type="button"
					onclick={handleAddClick}
					class="text-tertiary hover:text-secondary inline-flex h-5 w-5 items-center justify-center rounded-full border border-dashed border-gray-400 transition-colors dark:border-gray-500"
				>
					<Plus class="h-3 w-3" />
				</button>
			{/if}
		</div>
	{/if}
</div>

<!-- Portal dropdown to body to escape stacking contexts -->
{#if showDropdown && portalContainer}
	<div
		bind:this={dropdownElement}
		use:portal
		class="select-dropdown fixed z-[9999] max-h-48 min-w-40 overflow-y-auto rounded-md shadow-lg"
		style="top: {dropdownPosition.top}px; left: {dropdownPosition.left}px;"
	>
		<CreatableOptionList
			options={availableTags.map((tag) => ({
				id: tag.id,
				label: tag.name
			}))}
			onSelect={handleAddTag}
			createLabel={showCreateOption
				? isCreating
					? common_creating()
					: tags_createTagQuoted({ name: inputValue.trim() })
				: null}
			onCreate={handleCreateTag}
			creating={isCreating}
		>
			{#snippet row(option)}
				{@const tag = getTag(option.id)}
				<Tag
					label={tag?.name}
					color={tag?.color}
					icon={tagIcon(tag)}
					title={tagTooltip(tag)}
					isShiny={isApplicationTag(tag)}
				/>
			{/snippet}
		</CreatableOptionList>
	</div>
{/if}
