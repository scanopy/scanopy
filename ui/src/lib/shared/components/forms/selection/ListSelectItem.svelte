<!-- T: Item type, C: type of context passed to item -->
<!-- eslint-disable-next-line @typescript-eslint/no-explicit-any -->
<script lang="ts" generics="T, C">
	import { onMount, getContext } from 'svelte';
	import Tag from '../../data/Tag.svelte';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import InlineDescription from '$lib/features/topology/components/panel/inspectors/InlineDescription.svelte';
	import type { EntityDisplayComponent } from './types';
	import { displayTags, fitTags, HIDDEN_TAGS_CHIP_WIDTH } from './display-tags';
	import DisplayTag from './DisplayTag.svelte';
	import HiddenTagsChip from './HiddenTagsChip.svelte';

	export let item: T;
	export let displayComponent: EntityDisplayComponent<T, C>;
	export let context: C;
	export let staticTags: boolean = false;
	/** Show every tag instead of fitting them to the row (a hover popover, which can widen to fit). */
	export let showAllTags: boolean = false;

	const staticTagsContext = getContext<boolean>('staticTags') ?? false;

	$: icon = displayComponent.getIcon?.(item, context);
	$: tags = displayTags(displayComponent, item, context);
	$: label = displayComponent.getLabel(item, context);
	$: description = displayComponent.getDescription?.(item, context) || '';
	$: tagPickerProps = displayComponent.getTagPickerProps?.(item, context) ?? null;
	$: showTagPicker =
		tagPickerProps &&
		context &&
		typeof context === 'object' &&
		'showEntityTagPicker' in context &&
		(context as Record<string, unknown>).showEntityTagPicker;
	$: tagPickerDisabled =
		context &&
		typeof context === 'object' &&
		'tagPickerDisabled' in context &&
		!!(context as Record<string, unknown>).tagPickerDisabled;

	$: showEditableDescription =
		context &&
		typeof context === 'object' &&
		'showEditableEntityDescription' in context &&
		(context as Record<string, unknown>).showEditableEntityDescription;
	$: descriptionValue = showEditableDescription
		? (((context as Record<string, unknown>).entityDescription as string | null) ?? null)
		: null;
	$: descriptionDisabled = showEditableDescription
		? !!(context as Record<string, unknown>).entityDescriptionDisabled
		: true;
	$: descriptionOnSave = showEditableDescription
		? ((context as Record<string, unknown>).onEntityDescriptionSave as
				((value: string | null) => void) | undefined)
		: undefined;

	let containerEl: HTMLDivElement;
	let labelMeasureEl: HTMLSpanElement;
	let measureEl: HTMLDivElement;
	let visibleTagCount = 0;

	const SPACING = {
		gap: 8, // gap-2 = 0.5rem = 8px
		tagGap: 4, // gap-1 = 0.25rem = 4px
		moreWidth: HIDDEN_TAGS_CHIP_WIDTH
	};

	// The label keeps its full width and tags take what's left (see `fitTags`). The tag group never
	// shrinks, so when the label alone is wider than the row, it is the label that gives way to the
	// hidden-tags (i) icon, wrapping onto a second line rather than hiding part of the value. Its one-line
	// width comes from the hidden copy: the wrapped label's own width is the row's, not its text's.
	function calculateVisibleTags() {
		if (!containerEl || !measureEl || !labelMeasureEl || tags.length === 0) return;

		const tagWidths: number[] = [];
		measureEl
			.querySelectorAll('[data-tag]')
			.forEach((el) => tagWidths.push((el as HTMLElement).offsetWidth));
		if (tagWidths.length === 0) return;

		visibleTagCount = fitTags(
			containerEl.offsetWidth,
			labelMeasureEl.offsetWidth,
			tagWidths,
			SPACING
		);
	}

	onMount(() => {
		calculateVisibleTags();
		const observer = new ResizeObserver(() => calculateVisibleTags());
		observer.observe(containerEl);
		return () => observer.disconnect();
	});

	$: if ((tags || label) && containerEl) {
		// Recalculate when the tags or the label change
		requestAnimationFrame(() => calculateVisibleTags());
	}

	$: visibleTags = showAllTags ? tags : tags.slice(0, visibleTagCount);
	$: hiddenTags = showAllTags ? [] : tags.slice(visibleTagCount);
</script>

<div class="flex min-w-0 items-center gap-3" class:list-select-item-container={showTagPicker}>
	<!-- Icon -->
	{#if icon}
		<div class="flex h-7 w-7 flex-shrink-0 items-center justify-center">
			<svelte:component
				this={icon}
				class="h-5 w-5 {displayComponent.getIconColor?.(item, context) || 'text-secondary'}"
			/>
		</div>
	{/if}

	<!-- Label and description -->
	<div class="min-w-0 flex-1 overflow-hidden text-left">
		<div bind:this={containerEl} class="flex min-w-0 items-center gap-2">
			<span class="text-secondary min-w-0 [overflow-wrap:anywhere]">{label}</span>
			{#if tags.length > 0}
				<div class="flex flex-shrink-0 items-center gap-1">
					{#each visibleTags as tag, i (`${tag.label}-${i}`)}
						<DisplayTag {tag} interactive={!staticTags && !staticTagsContext} />
					{/each}
					{#if hiddenTags.length > 0}
						<HiddenTagsChip tags={hiddenTags} interactive={!staticTags && !staticTagsContext} />
					{/if}
				</div>
			{/if}
		</div>
		{#if description.length > 0}
			<span class="text-tertiary mt-1 block text-xs">{description}</span>
		{/if}
		{#if showEditableDescription && descriptionOnSave}
			<div class="mt-2 border-t border-gray-700/50 pt-2">
				<InlineDescription
					value={descriptionValue}
					editable={!descriptionDisabled}
					onSave={descriptionOnSave}
				/>
			</div>
		{/if}
	</div>

	<!-- Tag picker as direct child for container query positioning -->
	{#if showTagPicker && tagPickerProps && (!tagPickerDisabled || tagPickerProps.selectedTagIds.length > 0)}
		<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
		<div class="tag-picker-section flex items-start gap-1.5" onclick={(e) => e.stopPropagation()}>
			<TagPickerInline
				selectedTagIds={tagPickerProps.selectedTagIds}
				entityId={tagPickerProps.entityId}
				entityType={tagPickerProps.entityType}
				disabled={tagPickerDisabled}
				availableTags={tagPickerProps.availableTags}
				allowCreate={tagPickerProps.allowCreate ?? true}
			/>
		</div>
	{/if}
</div>

<!-- Hidden measurement container -->
{#if tags.length > 0}
	<div bind:this={measureEl} class="invisible absolute -left-[9999px]" aria-hidden="true">
		<span bind:this={labelMeasureEl} class="whitespace-nowrap">{label}</span>
		<div class="flex gap-1">
			{#each tags as tag, i (`measure-${tag.label}-${i}`)}
				<span data-tag
					><Tag
						label={tag.label}
						color={tag.color}
						icon={tag.icon ?? null}
						href={tag.href ?? ''}
					/></span
				>
			{/each}
		</div>
	</div>
{/if}

<style>
	.list-select-item-container {
		container-type: inline-size;
		flex-wrap: wrap;
	}

	/* Default (narrow): tag picker takes full width below */
	.list-select-item-container :global(.tag-picker-section) {
		width: 100%;
		border-top: 1px solid rgb(55 65 81 / 0.5);
		padding-top: 0.5rem;
		margin-top: 0.25rem;
	}

	/* Wide: tag picker sits inline at the end */
	@container (min-width: 500px) {
		.list-select-item-container :global(.tag-picker-section) {
			width: auto;
			border-top: none;
			padding-top: 0;
			margin-top: 0;
			margin-left: auto;
			flex-shrink: 0;
		}
	}
</style>
