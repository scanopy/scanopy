<script lang="ts" generics="T">
	import { getFieldKey, type FieldConfig } from '../types';
	import type { FieldFilter } from './filtering';
	import { filterLabelOf } from './fieldValues';
	import type { Tag as TagType, TagGroup } from '#lib/features/tags/types/base.ts';
	import Tag from '../Tag.svelte';
	import {
		applicationGroupId,
		compareTagOrder,
		isApplicationTag,
		tagIcon,
		tagTooltip
	} from '#lib/features/tags/groups.ts';
	import { tagGroupIdOf } from '#lib/features/tags/groupFields.ts';
	import { scrollFade } from '#lib/shared/utils/scrollFade.ts';
	import type { Color } from '#lib/shared/utils/styling.ts';
	import {
		common_staleOnly,
		common_showTrue,
		common_showFalse,
		common_noTagsAvailable,
		common_noValuesAvailable
	} from '#lib/paraglide/messages.js';

	/**
	 * One field's filter controls, rendered in its column's header popover.
	 */
	let {
		field,
		filter,
		allTags,
		tagGroups,
		/** The parent applies staleness and this field carries the toggle. */
		showStale,
		staleOnly,
		getFilterOptions,
		onToggleBoolean,
		onToggleString,
		onToggleTag,
		onToggleStale
	}: {
		field: FieldConfig<T>;
		filter: FieldFilter | undefined;
		allTags: TagType[];
		tagGroups: TagGroup[];
		showStale: boolean;
		staleOnly: boolean;
		getFilterOptions: (field: FieldConfig<T>) => string[];
		onToggleBoolean: (fieldKey: string, which: 'showTrue' | 'showFalse') => void;
		onToggleString: (fieldKey: string, value: string) => void;
		onToggleTag: (tagId: string) => void;
		onToggleStale: () => void;
	} = $props();

	let fieldKey = $derived(getFieldKey(field));
	/** The tag group this field is the column of, if any. */
	let groupId = $derived(tagGroupIdOf(fieldKey));
	let groupTags = $derived(
		groupId ? allTags.filter((tag) => tag.tag_group_id === groupId).sort(compareTagOrder) : []
	);
</script>

{#snippet tagChips(
	tags: TagType[],
	isSelected: (tag: TagType) => boolean,
	onToggle: (tag: TagType) => void
)}
	<div
		use:scrollFade
		class="flex min-h-0 flex-wrap gap-1.5 overflow-y-auto rounded-md bg-black/5 p-2 dark:bg-white/5"
	>
		{#if tags.length === 0}
			<p class="text-xs text-tertiary">{common_noTagsAvailable()}</p>
		{:else}
			{#each tags as tag (tag.id)}
				{@const selected = isSelected(tag)}
				<button
					onclick={() => onToggle(tag)}
					aria-pressed={selected}
					class="transition-opacity {selected ? 'opacity-100' : 'opacity-50 hover:opacity-75'}"
				>
					<Tag
						label={tag.name}
						color={tag.color as Color}
						icon={tagIcon(tag, applicationGroupId(tagGroups))}
						title={tagTooltip(tag, tagGroups)}
						isShiny={isApplicationTag(tag, applicationGroupId(tagGroups))}
					/>
				</button>
			{/each}
		{/if}
	</div>
{/snippet}

<div class="flex min-h-0 flex-col gap-2">
	<div class="text-sm font-medium text-secondary">{field.label}</div>

	{#if showStale}
		<!-- Server-side: the list holds one page, so only the server can filter by staleness. -->
		<label class="flex cursor-pointer items-center gap-2">
			<input
				type="checkbox"
				checked={staleOnly}
				onchange={onToggleStale}
				class="checkbox-card h-4 w-4 rounded-sm"
			/>
			<span class="text-sm text-secondary">{common_staleOnly()}</span>
		</label>
	{/if}

	{#if !field.filterable}
		<!-- Stale-only column: no value filter of its own. -->
	{:else if field.type === 'boolean'}
		<div class="space-y-1.5">
			<label class="flex cursor-pointer items-center gap-2">
				<input
					type="checkbox"
					checked={filter?.showTrue}
					onchange={() => onToggleBoolean(fieldKey, 'showTrue')}
					class="checkbox-card h-4 w-4 rounded-sm"
				/>
				<span class="text-sm text-secondary">{common_showTrue()}</span>
			</label>
			<label class="flex cursor-pointer items-center gap-2">
				<input
					type="checkbox"
					checked={filter?.showFalse}
					onchange={() => onToggleBoolean(fieldKey, 'showFalse')}
					class="checkbox-card h-4 w-4 rounded-sm"
				/>
				<span class="text-sm text-secondary">{common_showFalse()}</span>
			</label>
		</div>
	{:else if fieldKey === 'tags'}
		<!-- Special tag filter with colored tags (stores tag IDs for server-side filtering) -->
		{@render tagChips(
			allTags,
			(tag) => !!filter?.values.has(tag.id),
			(tag) => onToggleTag(tag.id)
		)}
	{:else if groupId}
		<!-- A tag group's column: its tags as chips, in the group's order. Values are tag ids, which
		     the column's filter value and the server's tag sets match on. -->
		{@render tagChips(
			groupTags,
			(tag) => !!filter?.values.has(tag.id),
			(tag) => onToggleString(fieldKey, tag.id)
		)}
	{:else}
		{@const uniqueValues = getFilterOptions(field)}
		<div
			use:scrollFade
			class="min-h-0 space-y-1.5 overflow-y-auto rounded-md bg-black/5 p-2 dark:bg-white/5"
		>
			{#if uniqueValues.length === 0}
				<p class="text-xs text-tertiary">{common_noValuesAvailable()}</p>
			{:else}
				{#each uniqueValues as value (value)}
					{@const label = filterLabelOf(field, value)}
					<label class="flex cursor-pointer items-center gap-2">
						<input
							type="checkbox"
							checked={filter?.values.has(value)}
							onchange={() => onToggleString(fieldKey, value)}
							class="checkbox-card h-4 w-4 rounded-sm"
						/>
						<span class="truncate text-sm text-secondary" title={label}>{label}</span>
					</label>
				{/each}
			{/if}
		</div>
	{/if}
</div>
