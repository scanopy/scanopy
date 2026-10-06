<script lang="ts">
	import { X, ChevronUp, ChevronDown } from 'lucide-svelte';
	import { createForm } from '@tanstack/svelte-form';
	import SearchInput from '$lib/shared/components/forms/input/SearchInput.svelte';
	import SearchHint from '$lib/shared/components/forms/input/SearchHint.svelte';
	import { views } from '$lib/shared/stores/metadata';
	import { getContext } from 'svelte';
	import { get, type Writable } from 'svelte/store';
	import { useSvelteFlow, type Edge, type Node } from '@xyflow/svelte';
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
	import {
		activeView,
		selectedNode as globalSelectedNode,
		selectedEdge as globalSelectedEdge,
		selectedNodes as globalSelectedNodes
	} from '../../queries';
	import { selectNode, type SelectionStores } from '../../selection';
	import { formatEntityLabel, viewEntityTypes } from '../../labels';
	import { focusNodes } from '../../viewport-fit';
	import { browser } from '$app/environment';
	import { shortcutLabel } from '$lib/features/search/results';
	import {
		topology_searchPlaceholder,
		topology_searchNoMatches,
		topology_searchMatchCount,
		common_close,
		common_next,
		common_previous
	} from '$lib/paraglide/messages';

	const { fitBounds, getNode, getInternalNode } = useSvelteFlow();

	const findShortcut = shortcutLabel(browser ? navigator.platform : '', 'F');

	/** Move the camera onto a node. See `boundsOfAdoptedNodes` for why this is not `fitView`. */
	function focusNode(id: string) {
		focusNodes({ fitBounds, getInternalNode }, [id]);
	}

	// The share and embed viewers scope selection to their own stores; see BaseTopologyViewer.
	/* eslint-disable svelte/require-store-reactive-access */
	const selectionStores: SelectionStores = {
		selectedNode: getContext<Writable<Node | null>>('selectedNode') ?? globalSelectedNode,
		selectedEdge: getContext<Writable<Edge | null>>('selectedEdge') ?? globalSelectedEdge,
		selectedNodes: getContext<Writable<Node[]>>('selectedNodes') ?? globalSelectedNodes
	};
	/* eslint-enable svelte/require-store-reactive-access */

	const topo = useTopology();
	const topoStore = topo.fromContext ? topo.store : null;
	let topology = $derived(
		topoStore
			? $topoStore
			: (topo.query?.data?.find((t) => t.id === $selectedTopologyId) as
					RenderableTopology | undefined)
	);

	const form = createForm(() => ({ defaultValues: { query: '' } }));

	/** Reactive copy of the query field, which `$derived` cannot read off the form. */
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
			resetQuery();
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
		focusNode(nodeId);
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
			resetQuery();
			clearSearch();
		} else if (event.key === 'Enter') {
			event.preventDefault();
			openMatch();
		} else if (event.key === 'ArrowDown') {
			event.preventDefault();
			nextMatch();
		} else if (event.key === 'ArrowUp') {
			event.preventDefault();
			prevMatch();
		}
	}

	/**
	 * Enter opens the current match, as it opens the highlighted row in the Cmd+K palette: the node is
	 * selected, which shows it in the inspector and closes the search.
	 */
	function openMatch() {
		const node = getNode(navigableIds[activeIndex] ?? '');
		if (!node) return;
		focusNode(node.id);
		resetQuery();
		selectNode(node, selectionStores);
	}

	/** Gap between the search box and the options panel or canvas controls beside it. */
	const OBSTACLE_GAP_PX = 16;

	let anchorEl: HTMLDivElement | undefined = $state();
	/** How far the options panel (left) and the canvas controls (right) reach into the view area. */
	let insets = $state({ left: 0, right: 0 });

	// Measured rather than derived from constants: the panel's width depends on whether it is
	// expanded, the controls' width on which buttons the viewer shows, and the share viewer has no
	// panel at all.
	$effect(() => {
		const area = anchorEl?.offsetParent;
		if (!isOpen || !area) return;
		const viewArea = document.getElementById('topology-view-area') ?? area;
		const panel = viewArea.querySelector<HTMLElement>('.topology-options');
		const controls = area.querySelector<HTMLElement>('.svelte-flow__panel.top-right');
		const measure = () => {
			const bounds = area.getBoundingClientRect();
			insets = {
				left: panel ? Math.max(0, panel.getBoundingClientRect().right - bounds.left) : 0,
				right: controls ? Math.max(0, bounds.right - controls.getBoundingClientRect().left) : 0
			};
		};
		measure();
		const observer = new ResizeObserver(measure);
		for (const el of [area, panel, controls]) if (el) observer.observe(el);
		return () => observer.disconnect();
	});

	function resetQuery() {
		form.reset();
		query = '';
	}

	function handleClose() {
		resetQuery();
		clearSearch();
	}
</script>

{#if isOpen}
	<!-- Spans the canvas between the options panel and the canvas controls, the same gap from each,
	     and centres the search box in it, so it never slides under either. -->
	<div
		bind:this={anchorEl}
		class="pointer-events-none absolute top-4 z-20 flex justify-center"
		style="left: {insets.left + OBSTACLE_GAP_PX}px; right: {insets.right + OBSTACLE_GAP_PX}px"
	>
		<!-- Same layout as the Cmd+K palette: the shared search box, then a hint footer. -->
		<div
			class="card card-static pointer-events-auto w-[42rem] max-w-full overflow-hidden p-0 shadow-lg"
		>
			<form.Field name="query">
				{#snippet children(field)}
					<SearchInput
						{field}
						value={query}
						id="topology-search"
						bind:inputEl
						placeholder={topology_searchPlaceholder({
							entities: formatEntityLabel(viewEntityTypes(currentView)),
							view: views.getName(currentView) ?? currentView
						})}
						shortcut={findShortcut}
						onInput={(value) => (query = value)}
						onkeydown={handleKeydown}
					>
						{#snippet trailing()}
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
									<button
										class="btn-icon p-0.5"
										onclick={prevMatch}
										disabled={navigableIds.length === 0}
										aria-label={common_previous()}
									>
										<ChevronUp class="h-4 w-4" />
									</button>
									<button
										class="btn-icon p-0.5"
										onclick={nextMatch}
										disabled={navigableIds.length === 0}
										aria-label={common_next()}
									>
										<ChevronDown class="h-4 w-4" />
									</button>
								</div>
							{/if}

							<button class="btn-icon p-0.5" onclick={handleClose} aria-label={common_close()}>
								<X class="h-4 w-4" />
							</button>
						{/snippet}
					</SearchInput>
				{/snippet}
			</form.Field>
			<SearchHint />
		</div>
	</div>
{/if}
