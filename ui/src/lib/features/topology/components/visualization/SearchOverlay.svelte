<script lang="ts">
	import { Search, X, ChevronUp, ChevronDown } from 'lucide-svelte';
	import { get } from 'svelte/store';
	import { useSvelteFlow } from '@xyflow/svelte';
	import {
		searchMatchNodeIds,
		searchActiveIndex,
		searchOpen,
		searchNavigableNodeIds,
		updateSearchFilter,
		recomputeSearchNavigation,
		clearSearch
	} from '../../interactions';
	import { collapsedContainers } from '../../collapse';
	import { useTopology, selectedTopologyId } from '../../context';
	import type { RenderableTopology } from '../../types/base';
	import { activeView } from '../../queries';
	import {
		topology_searchPlaceholder,
		topology_searchNoMatches,
		topology_searchMatchCount
	} from '$lib/paraglide/messages';

	const { fitView } = useSvelteFlow();

	const topo = useTopology();
	const topoStore = topo.fromContext ? topo.store : null;
	let topology = $derived(
		topoStore
			? $topoStore
			: (topo.query?.data?.find((t) => t.id === $selectedTopologyId) as
					RenderableTopology | undefined)
	);

	let query = $state('');
	let inputEl: HTMLInputElement | undefined = $state();

	// Subscribe to search stores
	let matchNodeIds = $state<string[]>(get(searchMatchNodeIds));
	searchMatchNodeIds.subscribe((value) => {
		matchNodeIds = value;
	});

	let activeIndex = $state(get(searchActiveIndex));
	searchActiveIndex.subscribe((value) => {
		activeIndex = value;
	});

	let navigableIds = $state<string[]>(get(searchNavigableNodeIds));
	searchNavigableNodeIds.subscribe((value) => {
		navigableIds = value;
	});

	let collapsedNodes = $state(get(collapsedContainers));
	collapsedContainers.subscribe((value) => {
		collapsedNodes = value;
	});

	let currentView = $state(get(activeView));
	activeView.subscribe((value) => {
		currentView = value;
	});

	let isOpen = $state(get(searchOpen));
	searchOpen.subscribe((value) => {
		isOpen = value;
		if (value) {
			// Focus the input when opened
			requestAnimationFrame(() => inputEl?.focus());
		} else {
			query = '';
		}
	});

	// Reactively update search filter when query or view changes
	$effect(() => {
		updateSearchFilter(topology ?? undefined, query, currentView);
	});

	// Recompute navigable IDs when matches or collapse state change
	$effect(() => {
		recomputeSearchNavigation(matchNodeIds, collapsedNodes, topology?.nodes ?? []);
	});

	function focusMatch(index: number) {
		if (navigableIds.length === 0) return;
		const wrappedIndex =
			((index % navigableIds.length) + navigableIds.length) % navigableIds.length;
		searchActiveIndex.set(wrappedIndex);
		const nodeId = navigableIds[wrappedIndex];
		fitView({ nodes: [{ id: nodeId }], padding: 0.5, duration: 300 });
	}

	function nextMatch() {
		focusMatch(activeIndex + 1);
	}

	function prevMatch() {
		focusMatch(activeIndex - 1);
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			event.stopPropagation();
			query = '';
			clearSearch();
		} else if (event.key === 'Enter' || event.key === 'ArrowDown') {
			event.preventDefault();
			nextMatch();
		} else if (event.key === 'ArrowUp') {
			event.preventDefault();
			prevMatch();
		}
	}

	function handleClose() {
		query = '';
		clearSearch();
	}
</script>

{#if isOpen}
	<div class="absolute left-1/2 top-4 z-20 -translate-x-1/2">
		<div class="card card-static flex items-center gap-2 px-3 py-2 shadow-lg">
			<Search class="text-tertiary h-4 w-4 flex-shrink-0" />
			<input
				bind:this={inputEl}
				bind:value={query}
				onkeydown={handleKeydown}
				type="text"
				placeholder={topology_searchPlaceholder()}
				class="h-7 w-64 border-none bg-transparent text-sm focus:outline-none"
				style="color: var(--color-text-primary)"
			/>

			{#if query}
				<span class="text-tertiary whitespace-nowrap text-xs">
					{#if navigableIds.length === 0}
						{topology_searchNoMatches()}
					{:else}
						{topology_searchMatchCount({
							current: String(activeIndex + 1),
							total: String(navigableIds.length)
						})}
					{/if}
				</span>

				<div class="flex items-center gap-0.5">
					<button class="btn-icon p-0.5" onclick={prevMatch} disabled={navigableIds.length === 0}>
						<ChevronUp class="h-4 w-4" />
					</button>
					<button class="btn-icon p-0.5" onclick={nextMatch} disabled={navigableIds.length === 0}>
						<ChevronDown class="h-4 w-4" />
					</button>
				</div>
			{/if}

			<button class="btn-icon p-0.5" onclick={handleClose}>
				<X class="h-4 w-4" />
			</button>
		</div>
	</div>
{/if}
