<script lang="ts">
	import { Search, X, Download } from 'lucide-svelte';
	import type { Snippet } from 'svelte';
	import {
		common_export,
		common_exporting,
		common_searchPlaceholder
	} from '$lib/paraglide/messages';

	let {
		searchQuery = $bindable(''),
		hasActiveSearch,
		onClearSearch,
		onExport,
		/** Column visibility and order menu. */
		columnMenu,
		/** Beside the search: actions on the filters set from the column headers. */
		filterActions = undefined
	}: {
		searchQuery: string;
		hasActiveSearch: boolean;
		onClearSearch: () => void;
		/** Export handler; the button shows only when there is one. */
		onExport: (() => void | Promise<void>) | null;
		columnMenu?: Snippet;
		filterActions?: Snippet;
	} = $props();

	let isExporting = $state(false);

	async function runExport() {
		if (!onExport || isExporting) return;
		isExporting = true;
		try {
			await onExport();
		} finally {
			isExporting = false;
		}
	}
</script>

<!--
	flex-wrap plus min-w-0: a flex item defaults to `min-width: auto`, which resolves to its
	min-content size, so neither group could shrink and the row stayed wider than the panel below
	about 1160px of viewport. That overflow reached `main` and left it scrollable sideways into
	blank space. The left group shrinks (the search keeps its own `min-w-48` floor) and the right
	group holds its size and wraps to a second line rather than squashing its buttons.
-->
<div class="flex flex-wrap items-end justify-between gap-y-3">
	<!-- Left: Search. Filters, grouping and sort live in the column headers. -->
	<div class="flex min-w-0 items-end gap-4">
		<!-- Search Input -->
		<div class="relative w-96 min-w-48">
			<Search class="text-tertiary absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2" />
			<input
				type="text"
				bind:value={searchQuery}
				placeholder={common_searchPlaceholder()}
				class="input-field w-full pl-10 pr-10"
			/>
			{#if hasActiveSearch}
				<button
					onclick={onClearSearch}
					class="text-tertiary hover:text-secondary absolute right-3 top-1/2 -translate-y-1/2 transition-colors"
				>
					<X class="h-4 w-4" />
				</button>
			{/if}
		</div>

		{#if filterActions}
			{@render filterActions()}
		{/if}
	</div>

	<!-- Right: Actions Group -->
	<div class="flex flex-shrink-0 items-end gap-2">
		{#if columnMenu}
			{@render columnMenu()}
		{/if}

		<!-- Export Button -->
		{#if onExport}
			<button
				onclick={runExport}
				disabled={isExporting}
				class="btn-secondary h-[42px] disabled:cursor-not-allowed disabled:opacity-50"
				title={isExporting ? common_exporting() : common_export()}
			>
				<Download class="h-5 w-5" />
			</button>
		{/if}
	</div>
</div>
