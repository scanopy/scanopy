<script lang="ts">
	import { onMount, type Snippet } from 'svelte';
	import { createForm } from '@tanstack/svelte-form';
	import { browser } from '$app/environment';
	import { Download } from 'lucide-svelte';
	import PageTitle from '$lib/shared/components/layout/PageTitle.svelte';
	import SearchInput from '$lib/shared/components/forms/input/SearchInput.svelte';
	import { isFindShortcut, shortcutLabel, registerPageFilter } from '$lib/features/search/results';
	import { lowercasePreservingAcronyms } from '$lib/shared/utils/formatting';
	import {
		common_export,
		common_exporting,
		common_filter,
		common_filterEntity
	} from '$lib/paraglide/messages';

	let {
		searchQuery = $bindable(''),
		onClearSearch,
		onExport,
		entityLabel = null,
		title = null,
		subtitle = null,
		/** Column visibility and order menu. */
		columnMenu,
		/** The page's own actions (Create), last in the toolbar. */
		actions = undefined
	}: {
		searchQuery: string;
		onClearSearch: () => void;
		/** Export handler; the button shows only when there is one. */
		onExport: (() => void | Promise<void>) | null;
		/** The entities the page lists, as its title shows them ("Hosts"). */
		entityLabel?: string | null;
		/** The page title, first in the row. */
		title?: string | null;
		/** Shown on hover over an (i) beside the title. */
		subtitle?: string | null;
		columnMenu?: Snippet;
		actions?: Snippet;
	} = $props();

	let isExporting = $state(false);
	let inputEl: HTMLInputElement | undefined = $state();
	const findShortcut = shortcutLabel(browser ? navigator.platform : '', 'F');

	const form = createForm(() => ({ defaultValues: { query: searchQuery } }));

	/** "hosts", for the placeholder and the palette's filter row. */
	let entity = $derived(entityLabel ? lowercasePreservingAcronyms(entityLabel) : null);
	let placeholder = $derived(entity ? common_filterEntity({ entity }) : common_filter());

	/**
	 * Whether this page is the one on screen. Every list page stays mounted; an inactive one sits in a
	 * zero-height, overflow-hidden wrapper, so it still lays out and `offsetParent` can't tell.
	 */
	function isVisible(): boolean {
		for (let el: HTMLElement | null = inputEl ?? null; el; el = el.parentElement) {
			if (el.clientHeight === 0 && getComputedStyle(el).overflow === 'hidden') return false;
		}
		return !!inputEl;
	}

	function setQuery(next: string) {
		form.setFieldValue('query', next);
		searchQuery = next;
	}

	function clear() {
		form.setFieldValue('query', '');
		onClearSearch();
	}

	onMount(() =>
		registerPageFilter({
			get entity() {
				return entity ?? '';
			},
			isVisible,
			apply: (query) => {
				setQuery(query);
				inputEl?.focus();
			}
		})
	);

	function handleWindowKeydown(event: KeyboardEvent) {
		// Only the page on screen takes Cmd/Ctrl+F; everywhere else the browser keeps its own find.
		if (!isFindShortcut(event) || !isVisible()) return;
		event.preventDefault();
		inputEl?.focus();
	}

	function handleInputKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && searchQuery) {
			event.stopPropagation();
			clear();
		}
	}

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
	<div class="flex min-w-0 items-center gap-4">
		<div class="w-96 min-w-48">
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
						onkeydown={handleInputKeydown}
					/>
				{/snippet}
			</form.Field>
		</div>
	</div>

	<!-- Right: table actions, then the page's own -->
	<div class="flex flex-shrink-0 items-center gap-2">
		{#if columnMenu}
			{@render columnMenu()}
		{/if}

		{#if onExport}
			<button
				onclick={runExport}
				disabled={isExporting}
				class="btn-secondary toolbar-control disabled:cursor-not-allowed disabled:opacity-50"
				title={isExporting ? common_exporting() : common_export()}
				aria-label={isExporting ? common_exporting() : common_export()}
			>
				<Download class="h-5 w-5" />
			</button>
		{/if}

		{#if actions}
			{@render actions()}
		{/if}
	</div>
</div>
