<!--
	The global search palette: one box that finds any entity the user can open, on any site they can
	see, and opens it. Typing a tag's name offers the tag as a completion; Tab turns it into a chip,
	and chips narrow every section to entities carrying all of them.

	Separate from the topology's Cmd+F search, which highlights nodes on the canvas in view. This one
	searches the server and navigates; that one filters what is already drawn. `/` opens this one
	on every tab, the topology included.
-->
<script lang="ts">
	import throttle from 'just-throttle';
	import { createForm } from '@tanstack/svelte-form';
	import GenericModal from './GenericModal.svelte';
	import SearchInput from '$lib/shared/components/forms/input/SearchInput.svelte';
	import SearchHint from '$lib/shared/components/forms/input/SearchHint.svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { entityUIConfig } from '$lib/shared/entity-ui-config';
	import { navigateToEntity } from '$lib/shared/stores/modal-registry';
	import { entities } from '$lib/shared/stores/metadata';
	import type { EntityDiscriminants } from '$lib/api/entities';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import { isApplicationTag, tagIcon, tagTooltip } from '$lib/features/tags/groups';
	import {
		acceptCompletion,
		backspaceChip,
		EMPTY_SEARCH,
		flattenGroups,
		globalSearchOpen,
		globalSearchRestoreQuery,
		hasSearchTerms,
		isGlobalSearchShortcut,
		moveHighlight,
		removeChip,
		responseGroups,
		tagCompletion,
		type GlobalSearchState,
		type SearchItem,
		type SearchRow
	} from '$lib/features/search/results';
	import { useGlobalSearch } from '$lib/features/search/queries';
	import {
		common_loading,
		common_search,
		globalSearch_noResults,
		globalSearch_noTaggedResults,
		globalSearch_placeholder
	} from '$lib/paraglide/messages';

	/** Same pause RichSelect gives a server-side search, so a burst of keystrokes costs one request. */
	const SEARCH_THROTTLE_MS = 300;

	const form = createForm(() => ({ defaultValues: { query: '' } }));

	/** Reactive copy of the query field, which `$derived` cannot read off the form. */
	let query = $state('');
	/** The tag chips before the text, in the order they were added. */
	let tagIds = $state<string[]>([]);
	/** The search as last sent to the server; its text trails `query` by the throttle. */
	let search = $state<GlobalSearchState>(EMPTY_SEARCH);
	let highlighted = $state(-1);
	let inputEl: HTMLInputElement | undefined = $state();

	// Trailing throttle, built once: rebuilding it per keystroke would defeat it.
	const sendSearch = throttle(
		(value: string) => {
			search = { text: value, tagIds };
		},
		SEARCH_THROTTLE_MS,
		{ leading: false, trailing: true }
	);

	const searchQuery = useGlobalSearch(() => search);
	const tagsQuery = useTagsQuery();

	let tags = $derived(tagsQuery.data ?? []);
	let chosenTags = $derived(tagIds.flatMap((id) => tags.filter((tag) => tag.id === id)));
	let completion = $derived(tagCompletion(query, tags, tagIds));

	let hasSearch = $derived(hasSearchTerms(search));

	// Only types a row can show and open; anything else never gets a section.
	let groups = $derived(
		hasSearch && searchQuery.data
			? responseGroups(searchQuery.data).filter((group) => {
					const config = entityUIConfig[group.type];
					return !!config?.displayComponent && !!(config.modalName || config.parentType);
				})
			: []
	);
	let rows = $derived(flattenGroups(groups));

	let isLoading = $derived(hasSearch && searchQuery.isPending);

	// A new result set starts with its first row highlighted, so Enter opens the best match. Keyed on
	// the highlight being out of range rather than on `rows` changing, so a refetch of the same
	// results doesn't throw the highlight back to the top mid-list.
	$effect(() => {
		if (rows.length === 0) highlighted = -1;
		else if (highlighted < 0 || highlighted >= rows.length) highlighted = 0;
	});

	// Reopened by "Back to Search" from an entity opened here: restore the search it was opened from.
	$effect(() => {
		const restored = $globalSearchRestoreQuery;
		if (!$globalSearchOpen || restored === null) return;
		globalSearchRestoreQuery.set(null);
		form.setFieldValue('query', restored.text);
		query = restored.text;
		tagIds = restored.tagIds;
		search = restored;
	});

	function groupLabel(type: EntityDiscriminants): string {
		return entities.getMetadata(type)?.entity_name_plural ?? entities.getName(type);
	}

	function rowIndex(row: SearchRow<SearchItem>): number {
		return rows.indexOf(row);
	}

	function close() {
		sendSearch.cancel();
		form.reset();
		query = '';
		tagIds = [];
		search = EMPTY_SEARCH;
		globalSearchOpen.set(false);
	}

	function openRow(row: SearchRow<SearchItem>) {
		const returnSearch = { text: query, tagIds };
		close();
		navigateToEntity(row.type, row.item.id, row.item as unknown as Record<string, unknown>, {
			returnSearch
		});
	}

	/** Applies a change of chips (and the text it cleared) at once: a chip is a deliberate act. */
	function applySearch(next: GlobalSearchState) {
		sendSearch.cancel();
		if (next.text !== query) form.setFieldValue('query', next.text);
		query = next.text;
		tagIds = next.tagIds;
		search = next;
		highlighted = -1;
	}

	function handleInput(value: string) {
		query = value;
		highlighted = -1;
		sendSearch(value);
	}

	function handleInputKeydown(event: KeyboardEvent) {
		if (event.key === 'Tab' && !event.shiftKey && completion) {
			event.preventDefault();
			applySearch(acceptCompletion({ text: query, tagIds }, completion));
		} else if (event.key === 'Backspace' && inputEl?.selectionEnd === 0) {
			const next = backspaceChip({ text: query, tagIds });
			if (next) {
				event.preventDefault();
				applySearch(next);
			}
		} else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
			event.preventDefault();
			highlighted = moveHighlight(highlighted, event.key === 'ArrowDown' ? 1 : -1, rows.length);
			document
				.getElementById(`global-search-row-${highlighted}`)
				?.scrollIntoView({ block: 'nearest' });
		} else if (event.key === 'Enter' && highlighted >= 0 && rows[highlighted]) {
			event.preventDefault();
			openRow(rows[highlighted]);
		}
	}

	function handleWindowKeydown(event: KeyboardEvent) {
		if (!isGlobalSearchShortcut(event)) return;
		event.preventDefault();
		globalSearchOpen.set(true);
		inputEl?.focus();
	}
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<GenericModal
	isOpen={$globalSearchOpen}
	title={common_search()}
	showTitleRow={false}
	showCloseButton={false}
	onClose={close}
	onOpen={() => inputEl?.focus()}
	size="lg"
>
	<form.Field name="query">
		{#snippet children(field)}
			<SearchInput
				{field}
				value={query}
				id="global-search"
				bind:inputEl
				placeholder={globalSearch_placeholder()}
				onInput={handleInput}
				onkeydown={handleInputKeydown}
			>
				{#snippet chips()}
					{#each chosenTags as tag (tag.id)}
						<Tag
							label={tag.name}
							color={tag.color}
							icon={tagIcon(tag)}
							title={tagTooltip(tag)}
							isShiny={isApplicationTag(tag)}
							removable
							onRemove={() => {
								applySearch(removeChip({ text: query, tagIds }, tag.id));
								inputEl?.focus();
							}}
						/>
					{/each}
				{/snippet}
				{#snippet ghost()}
					{#if completion}
						<Tag
							label={completion.name}
							color={completion.color}
							icon={tagIcon(completion)}
							isShiny={isApplicationTag(completion)}
							faded
						/>
					{/if}
				{/snippet}
			</SearchInput>
		{/snippet}
	</form.Field>

	{#if hasSearch}
		<div class="max-h-[60vh] overflow-y-auto px-2 pb-2" role="listbox">
			{#if isLoading && rows.length === 0}
				<p class="text-tertiary px-2 py-3 text-sm">{common_loading()}</p>
			{:else if rows.length === 0}
				<p class="text-tertiary px-2 py-3 text-sm">
					{search.text.trim()
						? globalSearch_noResults({ query: search.text })
						: globalSearch_noTaggedResults()}
				</p>
			{/if}

			{#each groups as group (group.type)}
				<div class="mb-2">
					<h3 class="text-tertiary px-2 py-1 text-xs font-semibold uppercase tracking-wide">
						{groupLabel(group.type)}
					</h3>
					{#each rows.filter((row) => row.type === group.type) as row (row.item.id)}
						{@const index = rowIndex(row)}
						<button
							id="global-search-row-{index}"
							type="button"
							role="option"
							aria-selected={index === highlighted}
							class="w-full rounded-lg px-2 py-1.5 text-left transition-colors {index ===
							highlighted
								? 'bg-gray-100 dark:bg-gray-800'
								: 'hover:bg-gray-50 dark:hover:bg-gray-800/50'}"
							onmousemove={() => (highlighted = index)}
							onclick={() => openRow(row)}
						>
							<EntityDisplayWrapper
								item={row.item}
								context={{}}
								displayComponent={entityUIConfig[row.type]!.displayComponent!}
							/>
						</button>
					{/each}
				</div>
			{/each}
		</div>
	{/if}

	{#snippet footer()}
		<SearchHint tabCompletes={completion !== null} />
	{/snippet}
</GenericModal>
