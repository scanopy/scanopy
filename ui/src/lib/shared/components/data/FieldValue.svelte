<script lang="ts" generics="T">
	import Tag from './Tag.svelte';
	import EntityTag from './EntityTag.svelte';
	import { MAX_ITEMS_IN_CELL, type CardFieldItem } from './types';
	import { getFieldValue } from './controls/fieldValues';
	import { formatDate, formatRelativeTime } from '$lib/shared/utils/formatting';
	import { attributeSourceExplanation } from '$lib/shared/utils/attribute-source';
	import type { EntityColumn } from './table/columns';
	import { common_moreItems, common_no, common_none, common_yes } from '$lib/paraglide/messages';

	let { item, column }: { item: T; column: EntityColumn<T> } = $props();

	let showAll = $state(false);

	let value = $derived(getFieldValue(item, column.field));

	/**
	 * Chips to render, if this cell is a list of things.
	 *
	 * A field can supply rich chips through `getItems`, but an array-typed field
	 * that only has a `getValue` is still a list — joining it into "a, b, c"
	 * would render it as prose in the table while the card shows the same field
	 * as tags. Falling back to plain chips keeps the two views consistent
	 * without every field having to opt in.
	 */
	let items = $derived<CardFieldItem[] | null>(
		column.display.getItems?.(item) ??
			// Index-suffixed: repeated values are legitimate (two hosts can carry the
			// same label) and a bare value as the key would collide in the {#each}.
			(Array.isArray(value)
				? value.map((entry, index) => ({ id: `${index}:${entry}`, label: entry }))
				: null)
	);

	let visible = $derived(items === null ? [] : showAll ? items : items.slice(0, MAX_ITEMS_IN_CELL));
	let overflow = $derived(items === null ? 0 : items.length - visible.length);

	/** Dates arrive as ISO strings and render in the user's chosen date format. */
	function formatValue(raw: Exclude<ReturnType<typeof getFieldValue>, string[]>): string {
		if (raw === null || raw === undefined || raw === '') return '';
		if (raw instanceof Date) return formatDate(raw);
		if (typeof raw === 'boolean') return raw ? common_yes() : common_no();
		if (column.field.type === 'date') {
			return column.field.display?.recency ? formatRelativeTime(raw) : formatDate(raw);
		}
		return raw;
	}

	// `items` is non-null for every array value, so this branch never sees one.
	let text = $derived(items === null ? formatValue(value as Exclude<typeof value, string[]>) : '');

	/** The value, and how it reached Scanopy when the field records a source. */
	let title = $derived.by(() => {
		const source = column.display.getSource?.(item);
		return source ? `${text}\n${attributeSourceExplanation(source)}` : text;
	});
</script>

{#if column.display.cell}
	{@render column.display.cell(item)}
{:else if items !== null}
	{#if items.length === 0}
		<!-- An em dash on its own is read as "dash", or skipped entirely. -->
		<span class="text-muted" aria-hidden="true">—</span>
		<span class="sr-only">{common_none()}</span>
	{:else}
		<div class="flex flex-wrap items-center gap-1">
			{#each visible as entry (entry.id)}
				{#if entry.entityRef}
					<EntityTag
						entityRef={entry.entityRef}
						icon={entry.icon}
						disabled={entry.disabled}
						color={entry.color}
						badge={entry.badge}
						label={entry.label}
					/>
				{:else}
					<Tag
						icon={entry.icon}
						disabled={entry.disabled}
						color={entry.color}
						badge={entry.badge}
						label={entry.label}
						title={entry.title}
					/>
				{/if}
			{/each}
			{#if overflow > 0}
				<button
					type="button"
					onclick={() => (showAll = true)}
					class="text-tertiary hover:text-secondary text-xs transition-colors"
				>
					{common_moreItems({ count: overflow })}
				</button>
			{/if}
		</div>
	{/if}
{:else if text === ''}
	<span class="text-muted" aria-hidden="true">—</span>
	<span class="sr-only">{common_none()}</span>
{:else}
	<span class="text-tertiary block truncate" {title}>{text}</span>
{/if}
