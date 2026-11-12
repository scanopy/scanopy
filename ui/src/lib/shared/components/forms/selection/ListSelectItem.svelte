<!-- T: Item type, C: type of context passed to item -->
<!-- eslint-disable-next-line @typescript-eslint/no-explicit-any -->
<script lang="ts" generics="T, C">
	import { onMount, getContext } from 'svelte';
	import Tag from '../../data/Tag.svelte';
	import TagPickerInline from '#lib/features/tags/components/TagPickerInline.svelte';
	import InlineDescription from '#lib/features/topology/components/panel/inspectors/InlineDescription.svelte';
	import type { EntityDisplayComponent } from './types';
	import { displayTags, fitTags, HIDDEN_TAGS_CHIP_WIDTH } from './display-tags';
	import DisplayTag from './DisplayTag.svelte';
	import HiddenTagsChip from './HiddenTagsChip.svelte';

	interface Props {
		item: T;
		displayComponent: EntityDisplayComponent<T, C>;
		context: C;
		staticTags?: boolean;
	}

	let { item, displayComponent, context, staticTags = false }: Props = $props();

	const staticTagsContext = getContext<boolean>('staticTags') ?? false;

	// Runes rather than `$:`: a legacy `$:` deep-reads every object it names, and a binding row's
	// context carries the whole host form, so each row walked it on every edit to any service.
	let icon = $derived(displayComponent.getIcon?.(item, context));
	let tags = $derived(displayTags(displayComponent, item, context));
	let label = $derived(displayComponent.getLabel(item, context));
	let description = $derived(displayComponent.getDescription?.(item, context) || '');
	let tagPickerProps = $derived(displayComponent.getTagPickerProps?.(item, context) ?? null);
	let contextFields = $derived(
		context && typeof context === 'object' ? (context as Record<string, unknown>) : null
	);
	let showTagPicker = $derived(!!tagPickerProps && !!contextFields?.showEntityTagPicker);
	let tagPickerDisabled = $derived(!!contextFields?.tagPickerDisabled);

	let showEditableDescription = $derived(!!contextFields?.showEditableEntityDescription);
	let descriptionValue = $derived(
		showEditableDescription ? ((contextFields?.entityDescription as string | null) ?? null) : null
	);
	let descriptionDisabled = $derived(
		showEditableDescription ? !!contextFields?.entityDescriptionDisabled : true
	);
	let descriptionOnSave = $derived(
		showEditableDescription
			? (contextFields?.onEntityDescriptionSave as ((value: string | null) => void) | undefined)
			: undefined
	);

	let containerEl = $state<HTMLDivElement>();
	let labelMeasureEl = $state<HTMLSpanElement>();
	let measureEl = $state<HTMLDivElement>();
	let visibleTagCount = $state(0);

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
		// Recount on the next frame, not inside the delivery: hiding a tag can unwrap the label and
		// change the row's height, which inside the callback raises the ResizeObserver loop error.
		let frame = 0;
		const observer = new ResizeObserver(() => {
			cancelAnimationFrame(frame);
			frame = requestAnimationFrame(calculateVisibleTags);
		});
		if (containerEl) observer.observe(containerEl);
		return () => {
			observer.disconnect();
			cancelAnimationFrame(frame);
		};
	});

	// Recalculate when the tags or the label change
	$effect(() => {
		void tags;
		void label;
		if (containerEl) requestAnimationFrame(() => calculateVisibleTags());
	});

	let visibleTags = $derived(tags.slice(0, visibleTagCount));
	let hiddenTags = $derived(tags.slice(visibleTagCount));
</script>

<div class="flex min-w-0 items-center gap-3" class:list-select-item-container={showTagPicker}>
	<!-- Icon -->
	{#if icon}
		{@const Icon = icon}
		<div class="flex h-7 w-7 shrink-0 items-center justify-center">
			<!-- `size` sizes a logo's round frame to the slot, as a host's tile fills it; a lucide glyph
			     keeps the 20px its classes give it. -->
			<Icon
				size={28}
				class="h-5 w-5 {displayComponent.getIconColor?.(item, context) || 'text-secondary'}"
			/>
		</div>
	{/if}

	<!-- Label and description -->
	<div class="min-w-0 flex-1 overflow-hidden text-left">
		<div bind:this={containerEl} class="flex min-w-0 items-center gap-2">
			<span class="min-w-0 wrap-anywhere text-secondary">{label}</span>
			{#if tags.length > 0}
				<div class="flex shrink-0 items-center gap-1">
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
			<span class="mt-1 block text-xs text-tertiary">{description}</span>
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
		<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
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
	<div bind:this={measureEl} class="invisible absolute left-[-9999px]" aria-hidden="true">
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
