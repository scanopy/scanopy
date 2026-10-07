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
	import KbdKey from '$lib/shared/components/feedback/KbdKey.svelte';
	import { keyLabel } from '$lib/shared/utils/shortcuts';
	import {
		acceptCompletion,
		addChip,
		backspaceChip,
		EMPTY_SEARCH,
		flattenGroups,
		globalSearchOpen,
		globalSearchRestoreQuery,
		hasSearchTerms,
		isGlobalSearchShortcut,
		moveHighlight,
		moveChipCursor,
		removeChip,
		removeSelectedChip,
		responseGroups,
		withMorePages,
		tagCompletion,
		type GlobalSearchState,
		type SearchItem,
		type SearchRow
	} from '$lib/features/search/results';
	import {
		fetchMoreMatches,
		GLOBAL_SEARCH_MORE_PAGE,
		useGlobalSearch
	} from '$lib/features/search/queries';
	import {
		common_loading,
		common_search,
		globalSearch_noResults,
		globalSearch_noTaggedResults,
		globalSearch_placeholder,
		globalSearch_showMore
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
	/** The chip ← → has selected, as an index into the chips; `null` while the text has focus. */
	let chipCursor = $state<number | null>(null);
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
	let tagsById = $derived(new Map(tags.map((tag) => [tag.id, tag])));
	let completion = $derived(tagCompletion(query, tags, tagIds));

	let hasSearch = $derived(hasSearchTerms(search));

	/**
	 * Pages "Show more" loaded, per type, for the search they were loaded under. Tied to that search
	 * rather than cleared on change, so every path that sets `search` drops them without a reset.
	 */
	let morePages = $state<{
		for: GlobalSearchState;
		pages: Partial<Record<EntityDiscriminants, SearchItem[]>>;
	}>({ for: EMPTY_SEARCH, pages: {} });
	let loadingMore = $state<EntityDiscriminants | null>(null);
	let currentMore = $derived(morePages.for === search ? morePages.pages : {});

	// Only types a row can show and open; anything else never gets a section.
	let groups = $derived(
		hasSearch && searchQuery.data
			? withMorePages(
					responseGroups(searchQuery.data).filter((group) => {
						const config = entityUIConfig[group.type];
						return !!config?.displayComponent && !!(config.modalName || config.parentType);
					}),
					currentMore
				)
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

	/** The tags a result carries, to show on its row and to click into chips. */
	function tagsOf(item: SearchItem) {
		const ids = 'tags' in item ? item.tags : [];
		return ids.flatMap((id) => tagsById.get(id) ?? []);
	}

	function rowIndex(row: SearchRow<SearchItem>): number {
		return rows.indexOf(row);
	}

	function close() {
		chipCursor = null;
		sendSearch.cancel();
		form.reset();
		query = '';
		tagIds = [];
		search = EMPTY_SEARCH;
		globalSearchOpen.set(false);
	}

	/** Appends the next page of `type`'s matches to its section. */
	async function loadMore(type: EntityDiscriminants) {
		if (loadingMore) return;
		const group = groups.find((g) => g.type === type);
		if (!group) return;
		const forSearch = search;
		loadingMore = type;
		try {
			const page = await fetchMoreMatches(forSearch, type, group.items.length);
			if (forSearch !== search) return;
			const pages = morePages.for === forSearch ? morePages.pages : {};
			morePages = {
				for: forSearch,
				pages: { ...pages, [type]: [...(pages[type] ?? []), ...page] }
			};
		} finally {
			loadingMore = null;
		}
	}

	/** Enter or a click on a row: open the match, or load the section's next page. */
	function activateRow(row: SearchRow<SearchItem>) {
		if ('more' in row) loadMore(row.type);
		else openRow(row);
	}

	function openRow(row: { type: EntityDiscriminants; item: SearchItem }) {
		const returnSearch = { text: query, tagIds };
		close();
		navigateToEntity(row.type, row.item.id, row.item as unknown as Record<string, unknown>, {
			returnSearch
		});
	}

	/** Applies a change of chips (and the text it cleared) at once: a chip is a deliberate act. */
	function applySearch(next: GlobalSearchState) {
		sendSearch.cancel();
		chipCursor = null;
		if (next.text !== query) form.setFieldValue('query', next.text);
		query = next.text;
		tagIds = next.tagIds;
		search = next;
		highlighted = -1;
	}

	function handleInput(value: string) {
		chipCursor = null;
		query = value;
		highlighted = -1;
		sendSearch(value);
	}

	function handleInputKeydown(event: KeyboardEvent) {
		if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
			const caretAtStart = inputEl?.selectionStart === 0 && inputEl?.selectionEnd === 0;
			const next = moveChipCursor(
				chipCursor,
				event.key === 'ArrowLeft' ? 'left' : 'right',
				tagIds.length,
				caretAtStart
			);
			if (next !== null || chipCursor !== null) event.preventDefault();
			chipCursor = next;
			return;
		}
		if (chipCursor !== null) {
			if (event.key === 'Backspace' || event.key === 'Delete') {
				event.preventDefault();
				const next = removeSelectedChip({ text: query, tagIds }, chipCursor);
				applySearch(next.state);
				chipCursor = next.cursor;
				return;
			}
			// Typing goes back to the text.
			if (event.key.length === 1) chipCursor = null;
		}
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
			activateRow(rows[highlighted]);
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
					{#each tagIds as id, index (id)}
						{@const tag = tagsById.get(id)}
						{#if tag}
							<span
								class="rounded-md {index === chipCursor
									? 'ring-2 ring-blue-500 ring-offset-1 dark:ring-offset-gray-900'
									: ''}"
							>
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
							</span>
						{/if}
					{/each}
				{/snippet}
				{#snippet ghost()}
					{#if completion}
						{@const tag = completion}
						<!-- Tab accepts from the keyboard; the click is for the mouse, so it takes no focus. -->
						<button
							type="button"
							tabindex="-1"
							class="pointer-events-auto ml-1.5 inline-flex items-center gap-1.5 opacity-60 transition-opacity hover:opacity-100"
							onclick={(event) => {
								event.stopPropagation();
								applySearch(acceptCompletion({ text: query, tagIds }, tag));
								inputEl?.focus();
							}}
						>
							<Tag
								label={tag.name}
								color={tag.color}
								icon={tagIcon(tag)}
								isShiny={isApplicationTag(tag)}
							/>
							<KbdKey key={keyLabel('Tab')} size="sm" />
						</button>
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
					<h3
						class="text-tertiary flex items-baseline gap-1.5 px-2 py-1 text-xs font-semibold uppercase tracking-wide"
					>
						{groupLabel(group.type)}
						<span class="font-normal tabular-nums">{group.total}</span>
					</h3>
					{#each rows.filter((row) => row.type === group.type) as row ('more' in row ? 'more' : row.item.id)}
						{@const index = rowIndex(row)}
						{#if 'more' in row}
							<button
								id="global-search-row-{index}"
								type="button"
								role="option"
								tabindex="-1"
								aria-selected={index === highlighted}
								disabled={loadingMore === row.type}
								class="text-secondary w-full rounded-lg px-2 py-1.5 text-left text-sm transition-colors {index ===
								highlighted
									? 'bg-gray-100 dark:bg-gray-800'
									: 'hover:bg-gray-50 dark:hover:bg-gray-800/50'}"
								onmousemove={() => (highlighted = index)}
								onclick={() => loadMore(row.type)}
							>
								{loadingMore === row.type
									? common_loading()
									: globalSearch_showMore({ count: Math.min(row.more, GLOBAL_SEARCH_MORE_PAGE) })}
							</button>
						{:else}
							<!-- Keys reach rows through the input (arrows, Enter); a row is clicked, not focused. -->
							<!-- svelte-ignore a11y_click_events_have_key_events -->
							<div
								id="global-search-row-{index}"
								role="option"
								tabindex="-1"
								aria-selected={index === highlighted}
								class="flex w-full cursor-pointer items-center gap-2 rounded-lg px-2 py-1.5 text-left transition-colors {index ===
								highlighted
									? 'bg-gray-100 dark:bg-gray-800'
									: 'hover:bg-gray-50 dark:hover:bg-gray-800/50'}"
								onmousemove={() => (highlighted = index)}
								onclick={() => openRow(row)}
							>
								<div class="min-w-0 flex-1">
									<EntityDisplayWrapper
										item={row.item}
										context={{}}
										displayComponent={entityUIConfig[row.type]!.displayComponent!}
									/>
								</div>
								{#if tagsOf(row.item).length > 0}
									<div class="flex shrink-0 flex-wrap justify-end gap-1">
										{#each tagsOf(row.item) as tag (tag.id)}
											<Tag
												label={tag.name}
												color={tag.color}
												icon={tagIcon(tag)}
												title={tagTooltip(tag)}
												isShiny={isApplicationTag(tag)}
												onclick={(event) => {
													event.stopPropagation();
													applySearch(addChip({ text: query, tagIds }, tag.id));
													inputEl?.focus();
												}}
											/>
										{/each}
									</div>
								{/if}
							</div>
						{/if}
					{/each}
				</div>
			{/each}
		</div>
	{/if}

	{#snippet footer()}
		<SearchHint tabCompletes={completion !== null} chipsSelectable={tagIds.length > 0} />
	{/snippet}
</GenericModal>
