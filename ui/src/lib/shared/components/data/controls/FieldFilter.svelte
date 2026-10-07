<script lang="ts" generics="T">
	import { getFieldKey, type FieldConfig } from '../types';
	import type { FieldFilter } from './filtering';
	import type { Tag as TagType } from '$lib/features/tags/types/base';
	import Tag from '../Tag.svelte';
	import { isApplicationTag, tagIcon, tagTooltip } from '$lib/features/tags/groups';
	import { scrollFade } from '$lib/shared/utils/scrollFade';
	import type { Color } from '$lib/shared/utils/styling';
	import {
		common_staleOnly,
		common_showTrue,
		common_showFalse,
		common_noTagsAvailable,
		common_noValuesAvailable
	} from '$lib/paraglide/messages';

	/**
	 * One field's filter controls, rendered in its column's header popover.
	 */
	let {
		field,
		filter,
		allTags,
		/** The parent applies staleness and this field carries the toggle. */
		showStale,
		staleOnly,
		getUniqueValues,
		onToggleBoolean,
		onToggleString,
		onToggleTag,
		onToggleStale
	}: {
		field: FieldConfig<T>;
		filter: FieldFilter | undefined;
		allTags: TagType[];
		showStale: boolean;
		staleOnly: boolean;
		getUniqueValues: (field: FieldConfig<T>) => string[];
		onToggleBoolean: (fieldKey: string, which: 'showTrue' | 'showFalse') => void;
		onToggleString: (fieldKey: string, value: string) => void;
		onToggleTag: (tagId: string) => void;
		onToggleStale: () => void;
	} = $props();

	let fieldKey = $derived(getFieldKey(field));
</script>

<div class="space-y-2">
	<div class="text-secondary text-sm font-medium">{field.label}</div>

	{#if showStale}
		<!-- Server-side: the list is server-paginated, so filtering
		     client-side would only filter the loaded page. -->
		<label class="flex cursor-pointer items-center gap-2">
			<input
				type="checkbox"
				checked={staleOnly}
				onchange={onToggleStale}
				class="checkbox-card h-4 w-4 rounded"
			/>
			<span class="text-secondary text-sm">{common_staleOnly()}</span>
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
					class="checkbox-card h-4 w-4 rounded"
				/>
				<span class="text-secondary text-sm">{common_showTrue()}</span>
			</label>
			<label class="flex cursor-pointer items-center gap-2">
				<input
					type="checkbox"
					checked={filter?.showFalse}
					onchange={() => onToggleBoolean(fieldKey, 'showFalse')}
					class="checkbox-card h-4 w-4 rounded"
				/>
				<span class="text-secondary text-sm">{common_showFalse()}</span>
			</label>
		</div>
	{:else if fieldKey === 'tags'}
		<!-- Special tag filter with colored tags (stores tag IDs for server-side filtering) -->
		<div
			use:scrollFade
			class="flex max-h-32 flex-wrap gap-1.5 overflow-y-scroll rounded-md bg-black/5 p-2 dark:bg-white/5"
		>
			{#if allTags.length === 0}
				<p class="text-tertiary text-xs">{common_noTagsAvailable()}</p>
			{:else}
				{#each allTags as tag (tag.id)}
					{@const isSelected = filter?.values.has(tag.id)}
					<button
						onclick={() => onToggleTag(tag.id)}
						aria-pressed={isSelected}
						class="transition-opacity {isSelected ? 'opacity-100' : 'opacity-50 hover:opacity-75'}"
					>
						<Tag
							label={tag.name}
							color={tag.color as Color}
							icon={tagIcon(tag)}
							title={tagTooltip(tag)}
							isShiny={isApplicationTag(tag)}
						/>
					</button>
				{/each}
			{/if}
		</div>
	{:else}
		{@const uniqueValues = field.filterOptions ?? getUniqueValues(field)}
		<div
			use:scrollFade
			class="max-h-32 space-y-1.5 overflow-y-scroll rounded-md bg-black/5 p-2 dark:bg-white/5"
		>
			{#if uniqueValues.length === 0}
				<p class="text-tertiary text-xs">{common_noValuesAvailable()}</p>
			{:else}
				{#each uniqueValues as value (value)}
					<label class="flex cursor-pointer items-center gap-2">
						<input
							type="checkbox"
							checked={filter?.values.has(value)}
							onchange={() => onToggleString(fieldKey, value)}
							class="checkbox-card h-4 w-4 rounded"
						/>
						<span class="text-secondary truncate text-sm" title={value}>{value}</span>
					</label>
				{/each}
			{/if}
		</div>
	{/if}
</div>
