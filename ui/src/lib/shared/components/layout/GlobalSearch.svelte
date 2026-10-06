<!--
	The Cmd+K palette: one box that finds a host, service, subnet or VLAN on any site the user can
	see and opens it.

	Separate from the topology's Cmd+F search, which highlights nodes on the canvas in view. This one
	searches the server and navigates; that one filters what is already drawn. Cmd+K opens this one
	on every tab, the topology included.
-->
<script lang="ts">
	import throttle from 'just-throttle';
	import { createForm } from '@tanstack/svelte-form';
	import GenericModal from './GenericModal.svelte';
	import SearchInput from '$lib/shared/components/forms/input/SearchInput.svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { entityUIConfig } from '$lib/shared/entity-ui-config';
	import { navigateToEntity } from '$lib/shared/stores/modal-registry';
	import type { EntityDiscriminants } from '$lib/api/entities';
	import {
		flattenGroups,
		globalSearchOpen,
		isGlobalSearchShortcut,
		moveHighlight,
		type SearchGroup,
		type SearchRow
	} from '$lib/features/search/results';
	import {
		useGlobalHostSearch,
		useGlobalServiceSearch,
		useGlobalSubnetSearch,
		useGlobalVlanSearch
	} from '$lib/features/search/queries';
	import {
		common_hosts,
		common_loading,
		common_search,
		common_services,
		common_subnets,
		common_vlans,
		globalSearch_navigateHint,
		globalSearch_noResults,
		globalSearch_placeholder,
		globalSearch_scope
	} from '$lib/paraglide/messages';

	/** Same pause RichSelect gives a server-side search, so a burst of keystrokes costs one request. */
	const SEARCH_THROTTLE_MS = 300;

	type Entity = { id: string };

	const form = createForm(() => ({ defaultValues: { query: '' } }));

	/** Reactive copy of the query field, which `$derived` cannot read off the form. */
	let query = $state('');
	/** The query as last sent to the server; trails `query` by the throttle. */
	let search = $state('');
	let highlighted = $state(-1);
	let inputEl: HTMLInputElement | undefined = $state();

	// Trailing throttle, built once: rebuilding it per keystroke would defeat it.
	const sendSearch = throttle(
		(value: string) => {
			search = value;
		},
		SEARCH_THROTTLE_MS,
		{ leading: false, trailing: true }
	);

	const hostsQuery = useGlobalHostSearch(() => search);
	const servicesQuery = useGlobalServiceSearch(() => search);
	const subnetsQuery = useGlobalSubnetSearch(() => search);
	const vlansQuery = useGlobalVlanSearch(() => search);

	let hasSearch = $derived(search.trim().length > 0);

	let groups = $derived<SearchGroup<Entity>[]>(
		hasSearch
			? [
					{ type: 'Host', items: hostsQuery.data ?? [] },
					{ type: 'Service', items: servicesQuery.data ?? [] },
					{ type: 'Subnet', items: subnetsQuery.data ?? [] },
					{ type: 'Vlan', items: vlansQuery.data ?? [] }
				]
			: []
	);
	let rows = $derived(flattenGroups(groups));

	let isLoading = $derived(
		hasSearch &&
			(hostsQuery.isPending ||
				servicesQuery.isPending ||
				subnetsQuery.isPending ||
				vlansQuery.isPending)
	);

	// A new result set starts with its first row highlighted, so Enter opens the best match.
	$effect(() => {
		highlighted = rows.length > 0 ? 0 : -1;
	});

	const groupLabels: Partial<Record<EntityDiscriminants, () => string>> = {
		Host: common_hosts,
		Service: common_services,
		Subnet: common_subnets,
		Vlan: common_vlans
	};

	function rowIndex(row: SearchRow<Entity>): number {
		return rows.indexOf(row);
	}

	function close() {
		sendSearch.cancel();
		form.reset();
		query = '';
		search = '';
		globalSearchOpen.set(false);
	}

	function openRow(row: SearchRow<Entity>) {
		close();
		navigateToEntity(row.type, row.item.id, row.item as unknown as Record<string, unknown>);
	}

	function handleInput(value: string) {
		query = value;
		sendSearch(value);
	}

	function handleInputKeydown(event: KeyboardEvent) {
		if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
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
	onClose={close}
	onOpen={() => inputEl?.focus()}
	size="md"
>
	<div class="border-b" style="border-color: var(--color-border)">
		<form.Field name="query">
			{#snippet children(field)}
				<SearchInput
					{field}
					value={query}
					id="global-search"
					bind:inputEl
					placeholder={globalSearch_placeholder()}
					scope={globalSearch_scope()}
					onInput={handleInput}
					onkeydown={handleInputKeydown}
				/>
			{/snippet}
		</form.Field>
	</div>

	<div class="max-h-[60vh] overflow-y-auto p-2" role="listbox">
		{#if isLoading && rows.length === 0}
			<p class="text-tertiary px-2 py-3 text-sm">{common_loading()}</p>
		{:else if hasSearch && rows.length === 0}
			<p class="text-tertiary px-2 py-3 text-sm">{globalSearch_noResults({ query: search })}</p>
		{/if}

		{#each groups as group (group.type)}
			{#if group.items.length > 0}
				<div class="mb-2">
					<h3 class="text-tertiary px-2 py-1 text-xs font-semibold uppercase tracking-wide">
						{groupLabels[group.type]?.()}
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
							onmouseenter={() => (highlighted = index)}
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
			{/if}
		{/each}
	</div>

	{#snippet footer()}
		<p class="text-tertiary px-4 py-2 text-xs">{globalSearch_navigateHint()}</p>
	{/snippet}
</GenericModal>
