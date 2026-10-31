<script lang="ts" module>
	import type { IconComponent } from '$lib/shared/utils/types';

	/** One compared option: a column of the grid. */
	export interface ComparisonColumn {
		id: string;
		label: string;
	}

	/**
	 * A heading and the rows under it. A section with an icon heads a group; one without reads as a
	 * quieter sub-heading of the section before it.
	 */
	export interface ComparisonSection<Row> {
		id: string;
		label: string;
		icon?: IconComponent;
		rows: Row[];
	}
</script>

<script lang="ts" generics="Row">
	import type { Snippet } from 'svelte';

	/**
	 * A grid of rows against columns, grouped under section headings, with a sticky header row and
	 * label column. The caller supplies what a row's label and each of its cells show.
	 */
	let {
		labelHeading,
		columns,
		sections,
		getRowKey,
		rowLabel,
		cell,
		class: className = ''
	}: {
		/** Heads the label column. */
		labelHeading: string;
		columns: ComparisonColumn[];
		sections: ComparisonSection<Row>[];
		getRowKey: (row: Row) => string;
		rowLabel: Snippet<[Row]>;
		cell: Snippet<[Row, ComparisonColumn]>;
		class?: string;
	} = $props();

	let gridColumns = $derived(
		columns.length === 0
			? '120px 1fr'
			: `minmax(100px, 20%) repeat(${columns.length}, minmax(100px, 1fr))`
	);
</script>

<div class="card overflow-auto p-0 {className}">
	<div class="comparison-row comparison-header-row" style="grid-template-columns: {gridColumns}">
		<div class="comparison-label-cell">
			<div class="text-xs font-medium lg:text-sm">{labelHeading}</div>
		</div>
		{#each columns as column (column.id)}
			<div class="comparison-value-cell">
				<span class="text-xs font-semibold text-primary lg:text-sm">{column.label}</span>
			</div>
		{/each}
	</div>

	{#each sections as section (section.id)}
		{@const SectionIcon = section.icon}
		<div class="comparison-category-row">
			{#if SectionIcon}
				<span
					class="flex items-center gap-2 p-2 text-xs font-semibold tracking-wide text-secondary uppercase lg:p-3 lg:text-sm"
				>
					<SectionIcon class="h-4 w-4 shrink-0" />
					{section.label}
				</span>
			{:else}
				<span class="p-2 text-xs font-medium text-tertiary lg:p-3 lg:text-sm">
					{section.label}
				</span>
			{/if}
		</div>

		{#each section.rows as row (getRowKey(row))}
			<div class="comparison-row" style="grid-template-columns: {gridColumns}">
				<div class="comparison-label-cell gap-2">{@render rowLabel(row)}</div>
				{#each columns as column (column.id)}
					<div class="comparison-value-cell">{@render cell(row, column)}</div>
				{/each}
			</div>
		{/each}
	{/each}
</div>

<style>
	.comparison-header-row {
		background: var(--color-bg-surface);
		position: sticky;
		top: 0;
		z-index: 11;
	}

	.comparison-category-row {
		border-bottom: 1px solid var(--color-border);
	}

	.comparison-row {
		display: grid;
		min-width: 500px;
		border-bottom: 1px solid var(--color-border);
	}

	.comparison-row:last-child {
		border-bottom: none;
	}

	.comparison-label-cell {
		padding: 0.5rem;
		color: var(--color-text-tertiary);
		text-align: left;
		display: flex;
		align-items: center;
		position: sticky;
		left: 0;
		z-index: 10;
		background: var(--color-bg-surface);
		border-right: 1px solid var(--color-border);
	}

	.comparison-value-cell {
		padding: 0.5rem;
		display: flex;
		align-items: center;
		justify-content: center;
		text-align: center;
		border-right: 1px solid var(--color-border);
	}

	.comparison-value-cell:last-child {
		border-right: none;
	}

	@media (min-width: 1024px) {
		.comparison-label-cell,
		.comparison-value-cell {
			padding: 0.75rem;
		}
	}
</style>
