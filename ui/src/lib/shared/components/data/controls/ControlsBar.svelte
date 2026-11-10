<script lang="ts">
	import type { Snippet } from 'svelte';
	import { createForm } from '@tanstack/svelte-form';
	import PageTitle from '$lib/shared/components/layout/PageTitle.svelte';
	import SearchInput from '$lib/shared/components/forms/input/SearchInput.svelte';
	import { isFindShortcut } from '$lib/features/search/results';
	import { keyLabel, shortcutLabel } from '$lib/shared/utils/shortcuts';
	import { isOnVisiblePage } from '$lib/shared/stores/modal-registry';
	import { lowercasePreservingAcronyms } from '$lib/shared/utils/formatting';
	import { common_filter, common_filterEntity } from '$lib/paraglide/messages';

	let {
		searchQuery = $bindable(''),
		search = true,
		onClearSearch,
		entityLabel = null,
		title = null,
		subtitle = null,
		/** The page's own actions (Create), last in the toolbar. */
		actions = undefined,
		/** First among the buttons while rows are selected (bulk delete). */
		selectionActions = undefined,
		onSearchFocusChange = undefined
	}: {
		searchQuery: string;
		/** Whether the list's endpoint takes a search, so the box shows. */
		search?: boolean;
		onClearSearch: () => void;
		/** The entities the page lists, as its title shows them ("Hosts"). */
		entityLabel?: string | null;
		/** The page title, first in the row. */
		title?: string | null;
		/** Shown on hover over an (i) beside the title. */
		subtitle?: string | null;
		actions?: Snippet;
		/** The search box gained or lost focus; while it has it, the list's single-key shortcuts are off. */
		onSearchFocusChange?: (focused: boolean) => void;
		selectionActions?: Snippet;
	} = $props();

	let inputEl: HTMLInputElement | undefined = $state();
	const findShortcut = shortcutLabel('F');
	// Esc clears from inside the box (handleInputKeydown); shown beside its clear button.
	const clearSearchShortcut = keyLabel('Escape');

	$effect(() => {
		const input = inputEl;
		if (!input) return;
		const update = () => onSearchFocusChange?.(document.activeElement === input);
		input.addEventListener('focus', update);
		input.addEventListener('blur', update);
		return () => {
			input.removeEventListener('focus', update);
			input.removeEventListener('blur', update);
		};
	});

	const form = createForm(() => ({ defaultValues: { query: searchQuery } }));

	/** "hosts", for the placeholder. */
	let entity = $derived(entityLabel ? lowercasePreservingAcronyms(entityLabel) : null);
	let placeholder = $derived(entity ? common_filterEntity({ entity }) : common_filter());

	function clear() {
		form.setFieldValue('query', '');
		onClearSearch();
	}

	function handleWindowKeydown(event: KeyboardEvent) {
		// Only the page on screen takes Cmd/Ctrl+F; everywhere else the browser keeps its own find.
		if (!isFindShortcut(event) || !isOnVisiblePage(inputEl)) return;
		event.preventDefault();
		// A second press leaves the search, so the list's single-key shortcuts work again.
		if (document.activeElement === inputEl) inputEl?.blur();
		else inputEl?.focus();
	}

	function handleInputKeydown(event: KeyboardEvent) {
		// Esc clears the search and leaves it, so the list's single-key shortcuts work again.
		if (event.key === 'Escape') {
			event.stopPropagation();
			if (searchQuery) clear();
			inputEl?.blur();
		}
	}
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<!--
	flex-wrap plus min-w-0: a flex item defaults to `min-width: auto`, which resolves to its
	min-content size, so neither group could shrink and the row stayed wider than the panel below
	about 1160px of viewport. That overflow reached `main` and left it scrollable sideways into
	blank space. The left group shrinks (the filter keeps its own `min-w-48` floor) and the right
	group holds its size and wraps to a second line rather than squashing its buttons.
-->
{#if title}
	<PageTitle {title} {subtitle} />
{/if}
<div class="flex flex-wrap items-center justify-between gap-y-3">
	<!-- Left: the page filter. Filters, grouping and sort live in the column headers. -->
	<div class="flex min-w-0 items-center gap-4 max-md:w-full">
		{#if search}
			<div class="w-full min-w-48 md:w-96">
				<form.Field name="query">
					{#snippet children(field)}
						<SearchInput
							{field}
							value={searchQuery}
							id="page-filter-{entity ?? 'items'}"
							bind:inputEl
							{placeholder}
							shortcut={findShortcut}
							inset={false}
							onInput={(next) => (searchQuery = next)}
							onClear={clear}
							clearShortcut={clearSearchShortcut}
							onkeydown={handleInputKeydown}
						/>
					{/snippet}
				</form.Field>
			</div>
		{/if}
	</div>

	<!-- Right: table actions, then the page's own -->
	<div class="flex shrink-0 items-center gap-2">
		{#if selectionActions}
			{@render selectionActions()}
		{/if}

		{#if actions}
			{@render actions()}
		{/if}
	</div>
</div>
