<script lang="ts">
	import { Plus } from '@lucide/svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import Popover from '$lib/shared/components/data/Popover.svelte';
	import TagCompletionGhost from './TagCompletionGhost.svelte';
	import InputGhost from '$lib/shared/components/forms/input/InputGhost.svelte';
	import { tagCompletion } from '$lib/features/tags/completion';
	import {
		useTagsQuery,
		useTagGroupsQuery,
		useCreateTagMutation,
		useBulkAddTagMutation,
		useBulkRemoveTagMutation,
		type EntityDiscriminants
	} from '$lib/features/tags/queries';
	import { createDefaultTag } from '$lib/features/tags/types/base';
	import { AVAILABLE_COLORS, type Color } from '$lib/shared/utils/styling';
	import { useCan } from '$lib/features/auth/access';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { tick, untrack } from 'svelte';
	import { triggerUpgrade } from '$lib/features/billing/trigger-upgrade';
	import { SvelteSet } from 'svelte/reactivity';
	import {
		common_creating,
		tags_addTag,
		tags_alreadyApplied,
		tags_createTagQuoted,
		tags_notAvailableHere,
		tags_planRequired,
		billing_lapsedBannerCta
	} from '$lib/paraglide/messages';
	import CreatableOptionList, {
		CREATE_ROW
	} from '$lib/shared/components/forms/selection/CreatableOptionList.svelte';
	import {
		applicationGroupId,
		compareTagsByGroup,
		groupOf,
		isApplicationTag,
		sameGroup,
		tagAllowedOn,
		tagGroupLabel,
		tagIcon,
		tagTooltip,
		withTagAdded
	} from '$lib/features/tags/groups';

	/**
	 * Compact inline tag picker for use in cards and bulk actions.
	 * Shows tags as pills with remove buttons, and a + button to add tags.
	 *
	 * Three modes of operation:
	 * 1. **Entity mode**: Provide `entityId` and `entityType` - mutations handled internally
	 * 2. **Callback mode**: Provide `onAdd`/`onRemove` callbacks - for bulk operations or custom handling
	 * 3. **Form mode**: Provide `onChange` - gets the whole new list, for a form that saves it later
	 *
	 * With `label`, the label sits above the picker, as above a modal's other fields.
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
		// Form mode prop - receives the full tag list after each change
		onChange,
		// Heading above the picker, for a form row
		label,
		// Optional pre-resolved tags (e.g. from topology snapshot on share pages)
		availableTags: availableTagsProp,
		// When true, tags created inline go in the Application group
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
		onChange?: (tagIds: string[]) => void;
		label?: string;
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
	// Whether any mode can change the tags at all
	let canEdit = $derived(!disabled && (isEntityMode || !!onAdd || !!onChange));

	let inputValue = $state('');
	let inputElement: HTMLInputElement | undefined = $state();
	let triggerElement: HTMLDivElement | undefined = $state();
	const dropdownId = $props.id();
	// Query and mutation
	const tagsQuery = useTagsQuery();
	const tagGroupsQuery = useTagGroupsQuery();
	let tagGroups = $derived(tagGroupsQuery.data ?? []);
	let applicationGroup = $derived(applicationGroupId(tagGroups));
	const createTagMutation = useCreateTagMutation();
	const organizationQuery = useOrganizationQuery();
	const can = useCan();

	// Derived state
	let tags = $derived(availableTagsProp ?? tagsQuery.data ?? []);
	let isCreating = $derived(createTagMutation.isPending);
	let organization = $derived(organizationQuery.data);

	// Demo orgs are read-only for non-owners (mirrors the credentials tab)
	// Free has no tagging: the picker still opens, says so, and offers a plan. The
	// server refuses the change either way.
	let taggingAllowed = $derived(
		billingPlans.getMetadata(organization?.plan?.type ?? null).features?.tagging !== false
	);

	// Check if user can create tags
	let canCreateTags = $derived(allowCreate && can('Tag', 'Write'));

	// Check if typed value matches an existing tag name exactly. Names are unique across the org, so
	// this checks every tag, including any the caller left out of `availableTags`.
	let existingTag = $derived(
		(tagsQuery.data ?? tags).find((t) => t.name.toLowerCase() === inputValue.trim().toLowerCase())
	);
	let exactMatch = $derived(existingTag !== undefined);

	// Show create option if user typed something, can create, and no exact match exists
	let showCreateOption = $derived(inputValue.trim().length > 0 && canCreateTags && !exactMatch);

	// Get tag by ID
	function getTag(id: string) {
		return tags.find((t) => t.id === id) ?? null;
	}

	// Optimistic: an added tag shows as a chip, and a removed one disappears, as soon as it's
	// asked for. The caller's list catches up when its refetch lands; until then these sets
	// bridge the gap, so an accepted ghost tag turns into its chip in place instead of vanishing
	// and reappearing. A failed change drops out of the sets and the chip reverts.
	const pendingAdds = new SvelteSet<string>();
	const pendingRemoves = new SvelteSet<string>();

	let shownTagIds = $derived([
		...selectedTagIds.filter((id) => !pendingRemoves.has(id)),
		...[...pendingAdds].filter((id) => !selectedTagIds.includes(id))
	]);

	// Filter available tags based on input, exclude already selected, and leave out tags whose group
	// doesn't allow the entity type, in group order
	let availableTags = $derived(
		tags
			.filter(
				(tag) =>
					!shownTagIds.includes(tag.id) &&
					(!entityType || tagAllowedOn(tag, tagGroups, entityType)) &&
					tag.name.toLowerCase().includes(inputValue.toLowerCase())
			)
			.sort(compareTagsByGroup(tagGroups))
	);

	// The typed name is taken by a tag this picker doesn't offer, so there is nothing to create and
	// nothing to pick. Say why rather than showing an empty menu.
	let unavailableNote = $derived.by(() => {
		if (!existingTag || availableTags.some((t) => t.id === existingTag.id)) return null;
		return shownTagIds.includes(existingTag.id)
			? tags_alreadyApplied({ name: existingTag.name })
			: tags_notAvailableHere({ name: existingTag.name });
	});

	let showDropdown = $derived(
		open &&
			(!taggingAllowed || availableTags.length > 0 || showCreateOption || unavailableNote !== null)
	);

	// The existing tag the typed text completes to, ghosted in the input; Tab applies it.
	let completion = $derived(tagCompletion(inputValue, availableTags, shownTagIds));

	// Up and Down move through the dropdown's rows (the create row, then the tags). The ghost
	// follows: it shows the highlighted tag, nothing while Create is highlighted, and the typed
	// text's completion before any arrow press. Typing starts over.
	let highlighted = $state<string | null>(null);
	let rowIds = $derived([
		...(showCreateOption ? [CREATE_ROW] : []),
		...availableTags.map((tag) => tag.id)
	]);
	let ghostTag = $derived(
		highlighted === null
			? completion
			: (availableTags.find((tag) => tag.id === highlighted) ?? null)
	);

	$effect(() => {
		void inputValue;
		untrack(() => (highlighted = null));
	});

	function moveHighlight(step: 1 | -1) {
		if (rowIds.length === 0) return;
		const current = rowIds.indexOf(highlighted ?? completion?.id ?? '');
		const next =
			current === -1
				? step === 1
					? 0
					: rowIds.length - 1
				: (current + step + rowIds.length) % rowIds.length;
		highlighted = rowIds[next];
	}

	function getRandomColor(): Color {
		return AVAILABLE_COLORS[Math.floor(Math.random() * AVAILABLE_COLORS.length)];
	}

	async function handleCreateTag() {
		if (!organization || isCreating) return;
		if (!canEdit) return;

		const name = inputValue.trim();
		if (!name) return;

		try {
			const newTag = createDefaultTag(organization.id);
			newTag.name = name;
			newTag.color = getRandomColor();
			if (createAsApplication) {
				newTag.tag_group_id = applicationGroup;
			}

			const result = await createTagMutation.mutateAsync(newTag);
			await handleAddTag(result.id);
			inputValue = '';
			open = false;
		} finally {
			inputElement?.focus();
		}
	}

	// Settle each pending change once the caller's list shows it.
	$effect(() => {
		const held = selectedTagIds;
		untrack(() => {
			for (const id of pendingAdds) if (held.includes(id)) pendingAdds.delete(id);
			for (const id of pendingRemoves) if (!held.includes(id)) pendingRemoves.delete(id);
		});
	});

	/** Run a change, reverting its optimistic state when it fails (throws, or reports `false`). */
	async function settle(pending: SvelteSet<string>, tagId: string, run: () => unknown) {
		pending.add(tagId);
		try {
			if ((await run()) === false) pending.delete(tagId);
		} catch (error) {
			pending.delete(tagId);
			throw error;
		}
	}

	/** `keepOpen` leaves the input up for the next tag, as Tab on a completion does. */
	async function handleAddTag(tagId: string, keepOpen = false) {
		inputValue = '';
		if (!keepOpen) open = false;
		if (isEntityMode && entityId && entityType) {
			await settle(pendingAdds, tagId, () =>
				bulkAddTagMutation.mutateAsync({
					entity_ids: [entityId],
					entity_type: entityType,
					tag_id: tagId
				})
			);
		} else if (onChange) {
			const next = withTagAdded(shownTagIds, tagId, tags);
			for (const heldId of shownTagIds) if (!next.includes(heldId)) pendingRemoves.add(heldId);
			await settle(pendingAdds, tagId, () => onChange(next));
		} else {
			// The server replaces a held tag of the same tag group on assignment; in callback
			// mode the caller holds the list, so drop the displaced tag here to match.
			const added = getTag(tagId);
			for (const heldId of shownTagIds) {
				if (heldId !== tagId && sameGroup(getTag(heldId), added)) {
					void settle(pendingRemoves, heldId, () => onRemove?.(heldId));
				}
			}
			await settle(pendingAdds, tagId, () => onAdd?.(tagId));
		}
	}

	async function handleRemoveTag(tagId: string) {
		if (isEntityMode && entityId && entityType) {
			await settle(pendingRemoves, tagId, () =>
				bulkRemoveTagMutation.mutateAsync({
					entity_ids: [entityId],
					entity_type: entityType,
					tag_id: tagId
				})
			);
		} else if (onChange) {
			const next = shownTagIds.filter((id) => id !== tagId);
			await settle(pendingRemoves, tagId, () => onChange(next));
		} else {
			await settle(pendingRemoves, tagId, () => onRemove?.(tagId));
		}
	}

	/** Open the input and focus it, for a keyboard shortcut. */
	export function focusInput() {
		if (!disabled) open = true;
	}

	let rootElement: HTMLDivElement | undefined = $state();
	/** Which remove button had focus when its tag was removed, so focus can land on a neighbour. */
	let refocusIndex: number | null = null;

	function removeButtons(): HTMLElement[] {
		return Array.from(rootElement?.querySelectorAll<HTMLElement>('[data-tag-remove]') ?? []);
	}

	function removeTag(tagId: string) {
		// Only a keyboard removal moves focus on. A clicked button also takes focus, unringed, and
		// handing that to the next tag's button would let Enter remove it instead of saving.
		const active = document.activeElement as HTMLElement | null;
		const index = active?.matches(':focus-visible') ? removeButtons().indexOf(active) : -1;
		refocusIndex = index === -1 ? null : index;
		handleRemoveTag(tagId);
	}

	// A tag removed from the keyboard unmounts the focused button, dropping focus to the page.
	// Once the list updates (in callback mode the caller's refetch lands later), put focus on the
	// tag that took its place, or on the input when none is left.
	$effect(() => {
		void shownTagIds;
		if (refocusIndex === null) return;
		const index = refocusIndex;
		tick().then(() => {
			if (rootElement?.contains(document.activeElement)) return;
			refocusIndex = null;
			const remaining = removeButtons();
			const next = remaining[Math.min(index, remaining.length - 1)];
			if (next) next.focus();
			else focusInput();
		});
	});

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
			e.preventDefault();
			moveHighlight(e.key === 'ArrowDown' ? 1 : -1);
		} else if (e.key === 'Tab' && !e.shiftKey && ghostTag) {
			e.preventDefault();
			handleAddTag(ghostTag.id, true);
		} else if (e.key === 'Enter') {
			// Enter takes the row the arrows reached. Before any arrow press it takes the text as
			// typed: it creates that tag, and only Tab takes the ghosted completion. With nothing to
			// create (the name exists, or creating isn't allowed), it applies the closest match.
			e.preventDefault();
			if (highlighted === CREATE_ROW) {
				handleCreateTag();
			} else if (highlighted !== null && ghostTag) {
				handleAddTag(ghostTag.id);
			} else if (showCreateOption) {
				handleCreateTag();
			} else if (availableTags.length > 0) {
				handleAddTag((completion ?? availableTags[0]).id);
			}
		} else if (e.key === 'Backspace' && inputValue === '' && shownTagIds.length > 0) {
			e.preventDefault();
			handleRemoveTag(shownTagIds[shownTagIds.length - 1]);
		} else if (e.key === 'Escape') {
			// Handled here, so a modal around the picker stays open.
			e.preventDefault();
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
	// focus the input.
	$effect(() => {
		if (open && !disabled) {
			setTimeout(() => inputElement?.focus(), 0);
		}
	});
</script>

{#snippet picker()}
	<div bind:this={rootElement} class="flex min-w-0 flex-wrap items-center gap-1">
		<!-- Selected tags -->
		{#each shownTagIds.filter((id) => getTag(id)) as tagId (tagId)}
			{@const tag = getTag(tagId)}
			<Tag
				label={tag?.name}
				color={tag?.color}
				icon={tagIcon(tag, applicationGroup)}
				title={tagTooltip(tag, tagGroups)}
				isShiny={isApplicationTag(tag, applicationGroup)}
				pill={!disabled}
				removable={canEdit && !!(onRemove || onChange || isEntityMode)}
				onRemove={() => removeTag(tagId)}
			/>
		{/each}

		<!-- Add button / dropdown (hide when no tags to add and creation disabled).
		     `hideAddButton` hides only the "+" affordance — the input still renders while `open`. -->
		{#if canEdit && (canCreateTags || tags.some((t) => !shownTagIds.includes(t.id)))}
			<div bind:this={triggerElement} class="relative flex min-h-5 items-center">
				{#if open}
					<!-- Input for searching/creating tags. It widens while it shows a completion, so the
					     ghosted tag fits inside it after the typed text, as in global search. -->
					<div class="relative">
						<input
							bind:this={inputElement}
							bind:value={inputValue}
							type="text"
							placeholder={ghostTag ? '' : tags_addTag()}
							aria-label={tags_addTag()}
							role="combobox"
							aria-expanded={showDropdown}
							aria-controls={dropdownId}
							aria-autocomplete="list"
							class="h-6 rounded-full px-2 text-xs transition-[width] focus:ring-1 focus:ring-blue-500 focus:outline-hidden {ghostTag
								? 'w-56'
								: 'w-24'}"
							style="border: 1px solid var(--color-border-input); background: var(--color-bg-input); color: var(--color-text-primary)"
							onfocus={() => (open = true)}
							onblur={handleBlur}
							onkeydown={handleKeydown}
						/>
						{#if ghostTag}
							{@const tag = ghostTag}
							<!-- px-[9px]: the input's px-2 plus its 1px border. -->
							<InputGhost value={inputValue} class="px-[9px] text-xs">
								<TagCompletionGhost
									{tag}
									{applicationGroup}
									onAccept={() => handleAddTag(tag.id, true)}
								/>
							</InputGhost>
						{/if}
					</div>
				{:else if !hideAddButton}
					<!-- Add button -->
					<button
						type="button"
						onclick={handleAddClick}
						class="inline-flex h-5 w-5 items-center justify-center rounded-full border border-dashed border-gray-400 text-tertiary transition-colors hover:text-secondary dark:border-gray-500"
					>
						<Plus class="h-3 w-3" />
					</button>
				{/if}
			</div>
		{/if}
	</div>
{/snippet}

{#if label}
	<div class="space-y-2">
		<div class="block text-sm font-medium text-secondary">{label}</div>
		{@render picker()}
	</div>
{:else}
	{@render picker()}
{/if}

<!-- Focus stays in the input, whose keydown drives the list. -->
<Popover
	triggerElement={triggerElement ?? null}
	isOpen={showDropdown}
	role="presentation"
	id={dropdownId}
	align="start"
	minWidth="10rem"
	onClose={() => (open = false)}
>
	{#if !taggingAllowed}
		<div class="space-y-2 px-3 py-2 text-xs text-secondary">
			<p>{tags_planRequired()}</p>
			<button
				type="button"
				class="text-link hover:underline"
				onclick={() => {
					open = false;
					triggerUpgrade({
						feature: 'tagging',
						source: 'tag_picker',
						surface: 'hosts_tab',
						gate_type: 'plan_required'
					});
				}}
			>
				{billing_lapsedBannerCta()}
			</button>
		</div>
	{:else if unavailableNote}
		<p class="border-b px-3 py-2 text-xs text-tertiary" style="border-color: var(--color-border)">
			{unavailableNote}
		</p>
	{/if}
	{#if taggingAllowed}
		<CreatableOptionList
			options={availableTags.map((tag) => ({
				id: tag.id,
				label: tag.name,
				group: tagGroupLabel(groupOf(tag, tagGroups))
			}))}
			onSelect={handleAddTag}
			createLabel={showCreateOption
				? isCreating
					? common_creating()
					: tags_createTagQuoted({ name: inputValue.trim() })
				: null}
			onCreate={handleCreateTag}
			createOnEnter
			{highlighted}
			creating={isCreating}
		>
			{#snippet row(option)}
				{@const tag = getTag(option.id)}
				<Tag
					label={tag?.name}
					color={tag?.color}
					icon={tagIcon(tag, applicationGroup)}
					title={tagTooltip(tag, tagGroups)}
					isShiny={isApplicationTag(tag, applicationGroup)}
				/>
			{/snippet}
		</CreatableOptionList>
	{/if}
</Popover>
