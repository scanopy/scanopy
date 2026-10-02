<script lang="ts" generics="V, OC">
	import { ChevronDown } from 'lucide-svelte';
	import ListSelectItem from './ListSelectItem.svelte';
	import type { EntityDisplayComponent } from './types';
	import { tick, onMount } from 'svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import throttle from 'just-throttle';
	import {
		common_loading,
		common_noOptionsAvailable,
		common_selectOption,
		common_typeToFilter,
		common_noOptionsMatch
	} from '$lib/paraglide/messages';

	let {
		label = '',
		selectedValue = '',
		options = [],
		placeholder = undefined,
		required = false,
		disabled = false,
		error = null,
		helpText = '',
		onSelect,
		onDisabledClick = null,
		showSearch = false,
		displayComponent,
		getOptionContext = () => new Object() as OC,
		minWidth = null,
		fitToContent = false,
		onSearchChange = null,
		onLoadMore = null,
		hasMore = false,
		loading = false,
		selectedOption = undefined
	}: {
		label?: string;
		selectedValue?: string | null;
		options?: V[];
		placeholder?: string | undefined;
		required?: boolean;
		disabled?: boolean;
		error?: string | null;
		/** Guidance rendered below the control, matching FormField's help text. */
		helpText?: string;
		onSelect: (value: string) => void;
		onDisabledClick?: ((value: string) => void) | null;
		showSearch?: boolean;
		displayComponent: EntityDisplayComponent<V, OC>;
		getOptionContext?: (option: V, index: number) => OC;
		/** Optional minimum width (CSS length, e.g. "18rem" or "280px") applied
		 *  to the trigger. Use in unbounded contexts — like the topology view
		 *  picker in a toolbar — where the trigger otherwise collapses to its
		 *  content width and the dropdown inherits that narrowness. Bounded
		 *  contexts (ConfigPanels, ListManagers) should leave this null so the
		 *  trigger keeps filling its container. */
		minWidth?: string | null;
		/** Size the trigger (and therefore the dropdown) to the widest option it
		 *  contains, rather than a fixed `minWidth` or the selected item's width.
		 *  Uses a hidden CSS-grid sizer so the trigger never collapses below — or
		 *  grows beyond — what the longest label needs. */
		fitToContent?: boolean;
		/** Server-side search. When set, typing is passed here (throttled) instead of filtering
		 *  `options` locally: the caller owns the result set, and a local filter over one loaded
		 *  page would miss every match on the pages not yet fetched. */
		onSearchChange?: ((query: string) => void) | null;
		/** Called when the list is scrolled near its end and `hasMore` is true. */
		onLoadMore?: (() => void) | null;
		hasMore?: boolean;
		/** A fetch is in flight: shows a loading row and holds off `onLoadMore`. */
		loading?: boolean;
		/** The selected option when it may not be in `options`, as with a paginated list whose
		 *  loaded pages don't include it. */
		selectedOption?: V;
	} = $props();

	/** Debounce window for server-side search, in ms. Matches DataControls. */
	const SEARCH_THROTTLE_MS = 300;
	/** How close to the bottom, in px, a scroll has to get before the next page loads. */
	const LOAD_MORE_THRESHOLD_PX = 48;

	// Trailing throttle so a burst of keystrokes costs one request and the last one is never
	// dropped. Built once: rebuilding it per keystroke would defeat the debounce.
	const notifySearchChange = throttle(
		(query: string) => onSearchChange?.(query),
		SEARCH_THROTTLE_MS,
		{ leading: false, trailing: true }
	);

	/** Clear the search box. A server-side search is reset at once, with any pending keystroke
	 *  dropped, so the next open starts from the unsearched list. */
	function resetFilter() {
		filterText = '';
		if (onSearchChange) {
			notifySearchChange.cancel();
			onSearchChange('');
		}
	}

	function handleFilterInput() {
		if (onSearchChange) notifySearchChange(filterText);
	}

	function handleScroll() {
		updateScrollIndicators();
		if (!scrollContainerEl || !onLoadMore || !hasMore || loading) return;
		const { scrollTop, scrollHeight, clientHeight } = scrollContainerEl;
		if (scrollTop + clientHeight >= scrollHeight - LOAD_MORE_THRESHOLD_PX) onLoadMore();
	}

	let isOpen = $state(false);
	let dropdownElement: HTMLDivElement | undefined = $state();
	let triggerElement: HTMLButtonElement | undefined = $state();
	let inputElement: HTMLInputElement | undefined = $state();
	let dropdownPosition = $state({ top: 0, left: 0, width: 0, maxHeight: 384 });
	let openUpward = $state(false);
	let filterText = $state('');
	let scrollContainerEl: HTMLDivElement | undefined = $state();
	let canScrollUp = $state(false);
	let canScrollDown = $state(false);

	function updateScrollIndicators() {
		if (!scrollContainerEl) return;
		const { scrollTop, scrollHeight, clientHeight } = scrollContainerEl;
		canScrollUp = scrollTop > 2;
		canScrollDown = scrollTop + clientHeight < scrollHeight - 2;
	}

	// Portal container for escaping transform contexts (e.g., SvelteFlow)
	let portalContainer: HTMLDivElement | null = $state(null);

	onMount(() => {
		portalContainer = document.createElement('div');
		portalContainer.style.position = 'absolute';
		portalContainer.style.top = '0';
		portalContainer.style.left = '0';
		portalContainer.style.width = '0';
		portalContainer.style.height = '0';
		document.body.appendChild(portalContainer);

		return () => {
			portalContainer?.remove();
		};
	});

	// Lock body scroll when dropdown is open
	$effect(() => {
		if (isOpen) {
			document.body.style.overflow = 'hidden';
			requestAnimationFrame(updateScrollIndicators);
			return () => {
				document.body.style.overflow = '';
			};
		}
	});

	// Portal action to move element to body, escaping any transform contexts
	function portal(node: HTMLElement) {
		if (portalContainer) {
			portalContainer.appendChild(node);
		}
		return {
			destroy() {
				// Node will be removed when portalContainer is cleaned up
				// or when Svelte removes it from the DOM
			}
		};
	}

	let selectedItem = $derived(
		options.find((i) => displayComponent.getId(i) === selectedValue) ??
			(selectedOption && displayComponent.getId(selectedOption) === selectedValue
				? selectedOption
				: undefined)
	);

	// Filter options based on search text
	let filteredOptions = $derived(
		options.filter((option, index) => {
			if (onSearchChange || !filterText.trim()) return true;

			const context = getOptionContext(option, index);

			const searchTerm = filterText.toLowerCase();
			const label = displayComponent.getLabel(option, context).toLowerCase();
			const description = displayComponent.getDescription?.(option, context)?.toLowerCase() || '';
			const tags = displayComponent.getTags?.(option, context) ?? [];
			// Fall back to the tooltip for a tag drawn as an icon alone: it has no visible text, but
			// its name is still what someone types to find it.
			const tagLabels = tags.map((tag) => (tag.label ?? tag.title ?? '').toLowerCase()).join(' ');

			return (
				label.includes(searchTerm) ||
				description.includes(searchTerm) ||
				tagLabels.includes(searchTerm)
			);
		})
	);

	// Group filtered options by category when getCategory is provided
	let groupedOptions = $derived.by(() => {
		const optionsToGroup = filteredOptions;

		if (!displayComponent.getCategory) {
			return [{ category: null, options: optionsToGroup }];
		}

		const groups = new SvelteMap<string | null, V[]>();

		optionsToGroup.forEach((option, index) => {
			const context = getOptionContext(option, index);
			const category = displayComponent.getCategory!(option, context);
			if (!groups.has(category)) {
				groups.set(category, []);
			}
			groups.get(category)!.push(option);
		});

		// Sort categories alphabetically, with null category first
		const sortedEntries = Array.from(groups.entries()).sort(([a], [b]) => {
			if (a === null) return -1;
			if (b === null) return 1;
			return a.localeCompare(b);
		});

		return sortedEntries.map(([category, options]) => ({ category, options }));
	});

	// Simple one-time positioning when dropdown opens
	async function calculatePosition() {
		if (!triggerElement) return;

		await tick();
		const rect = triggerElement.getBoundingClientRect();
		const viewportHeight = window.innerHeight;
		const dropdownMaxHeight = 384; // max-h-96 = 24rem = 384px
		const gap = 1; // Minimal gap to prevent overlap
		const viewportPadding = 8; // Minimum distance from viewport edge

		// Simple logic: if not enough space below, open upward
		const spaceBelow = viewportHeight - rect.bottom - gap;
		openUpward = spaceBelow < dropdownMaxHeight && rect.top > spaceBelow;

		// Constrain dropdown height to available viewport space
		const availableSpace = openUpward
			? rect.top - gap - viewportPadding
			: viewportHeight - rect.bottom - gap - viewportPadding;
		const maxHeight = Math.min(dropdownMaxHeight, availableSpace);

		dropdownPosition = {
			top: openUpward ? rect.top - gap : rect.bottom + gap,
			left: rect.left,
			width: rect.width,
			maxHeight
		};
	}

	async function handleToggle(e: MouseEvent) {
		e.preventDefault();
		e.stopPropagation();
		if (!disabled) {
			if (!isOpen) {
				isOpen = true;
				resetFilter();
				await calculatePosition(); // Calculate once when opening
				// Focus the input after the dropdown is positioned
				setTimeout(() => inputElement?.focus(), 0);
			} else {
				isOpen = false;
				resetFilter();
			}
		}
	}

	function handleSelect(value: string) {
		try {
			let index: number | undefined;
			const item = options.find((o, i) => {
				if (displayComponent.getId(o) === value) {
					index = i;
					return true;
				}
				return false;
			});
			if (item && index !== undefined) {
				isOpen = false;
				resetFilter();
				onSelect(value);
			}
		} catch (e) {
			console.warn('Error in handleSelect:', e);
			isOpen = false;
			resetFilter();
		}
	}

	function handleClickOutside(event: MouseEvent) {
		if (
			dropdownElement &&
			!dropdownElement.contains(event.target as Node) &&
			triggerElement &&
			!triggerElement.contains(event.target as Node)
		) {
			isOpen = false;
			resetFilter();
		}
	}

	function handleInputKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			isOpen = false;
			resetFilter();
			triggerElement?.focus(); // Return focus to trigger
		}
		// Prevent the input keydown from bubbling to parent components
		e.stopPropagation();
	}
</script>

<!-- Only handle outside clicks -->
<svelte:window onclick={handleClickOutside} />

<div class="relative {fitToContent ? 'inline-grid' : ''}">
	<!-- Label -->
	{#if label}
		<div class="text-secondary mb-2 block text-sm font-medium">
			{label}
			{#if required}
				<span class="text-danger ml-1">*</span>
			{/if}
		</div>
	{/if}

	<!-- Hidden sizer: stacks every option in the same grid cell as the trigger so
	     the cell — and thus the trigger — sizes to the widest label. -->
	{#if fitToContent}
		<div class="invisible col-start-1 row-start-1 h-0 overflow-hidden" aria-hidden="true">
			{#each options as option, i (displayComponent.getId(option))}
				{@const sizerContext = getOptionContext(option, i)}
				<div class="flex items-center gap-3 px-3">
					<ListSelectItem
						context={sizerContext}
						item={option}
						{displayComponent}
						staticTags={true}
					/>
					<span class="h-4 w-4 flex-shrink-0"></span>
				</div>
			{/each}
		</div>
	{/if}

	<!-- Dropdown Trigger -->
	<button
		bind:this={triggerElement}
		type="button"
		onclick={handleToggle}
		class="select-trigger text-primary flex w-full items-center justify-between rounded-md px-3 py-2
           {fitToContent ? 'col-start-1 row-start-1' : ''}
           {error ? 'border-red-500' : ''}
           {disabled || options.length == 0 ? 'cursor-not-allowed opacity-50' : ''}"
		style={minWidth ? `min-width: ${minWidth}` : undefined}
		disabled={disabled || options.length == 0}
	>
		<div class="flex min-w-0 flex-1 items-center gap-3">
			{#if selectedItem}
				{@const context = getOptionContext(selectedItem, 0)}
				<ListSelectItem {context} item={selectedItem} {displayComponent} staticTags={true} />
			{:else}
				<span class="text-secondary"
					>{options.length == 0 && loading
						? common_loading()
						: options.length == 0
							? common_noOptionsAvailable()
							: (placeholder ?? common_selectOption())}</span
				>
			{/if}
		</div>

		<ChevronDown
			class="text-tertiary h-4 w-4 flex-shrink-0 transition-transform {isOpen ? 'rotate-180' : ''}"
		/>
	</button>

	<!-- Error Message -->
	{#if error}
		<div class="text-danger mt-1 flex items-center gap-2 text-sm">
			<span>{error}</span>
		</div>
	{/if}

	{#if helpText}
		<p class="text-tertiary mt-1 text-xs">{helpText}</p>
	{/if}
</div>

<!-- Portal dropdown to body - escapes SvelteFlow transform context -->
{#if isOpen && !disabled && portalContainer}
	<div
		bind:this={dropdownElement}
		use:portal
		class="select-dropdown fixed z-[9999] overflow-hidden scroll-smooth rounded-md shadow-lg"
		style="top: {dropdownPosition.top}px; left: {dropdownPosition.left}px; width: {dropdownPosition.width}px; max-height: {dropdownPosition.maxHeight}px;
           {openUpward ? 'transform: translateY(-100%);' : ''}"
	>
		<!-- Search Input -->
		{#if showSearch}
			<div
				class="sticky top-0 border-b p-2"
				style="border-color: var(--color-border); background: var(--color-bg-input)"
			>
				<input
					bind:this={inputElement}
					bind:value={filterText}
					type="text"
					placeholder={common_typeToFilter()}
					class="input-field text-primary w-full rounded px-2 py-1 text-sm"
					onkeydown={handleInputKeydown}
					onclick={(e) => e.stopPropagation()}
					oninput={handleFilterInput}
				/>
			</div>
		{/if}

		<!-- Options list with scroll container -->
		<div class="relative">
			{#if canScrollUp}
				<div
					class="pointer-events-none absolute inset-x-0 top-0 z-10 h-6 rounded-t-md bg-gradient-to-b from-[var(--color-bg-elevated)] to-transparent"
				></div>
			{/if}
			<div
				bind:this={scrollContainerEl}
				class="overflow-y-auto"
				style="max-height: {dropdownPosition.maxHeight - (showSearch ? 44 : 0)}px"
				onscroll={handleScroll}
			>
				{#if loading && options.length === 0}
					<div class="text-tertiary px-3 py-4 text-center text-sm">{common_loading()}</div>
				{:else if groupedOptions.length === 0 || groupedOptions.every((group) => group.options.length === 0)}
					<div class="text-tertiary px-3 py-4 text-center text-sm">
						{common_noOptionsMatch({ filterText })}
					</div>
				{:else}
					{#each groupedOptions as group, groupIndex (group.category ?? '__ungrouped__')}
						{#if group.options.length > 0}
							<!-- Category Header -->
							{#if group.category !== null}
								<div
									class="text-secondary sticky top-0 border-b px-3 py-2 text-xs font-semibold uppercase tracking-wide"
									style="border-color: var(--color-border); background: var(--color-bg-surface)"
								>
									{group.category}
								</div>
							{/if}

							<!-- Options in this category -->
							{#each group.options as option, optionIndex (displayComponent.getId(option))}
								{@const context = getOptionContext(option, optionIndex)}
								{@const isLastInGroup = optionIndex === group.options.length - 1}
								{@const isLastGroup = groupIndex === groupedOptions.length - 1}
								{@const isDisabled = displayComponent.getDisabled?.(option, context) ?? false}
								{@const isClickableDisabled = isDisabled && onDisabledClick != null}
								{@const disabledReason = isDisabled
									? (displayComponent.getDisabledReason?.(option, context) ?? null)
									: null}
								<button
									title={disabledReason}
									type="button"
									onclick={(e) => {
										e.preventDefault();
										e.stopPropagation();
										if (isDisabled) {
											isOpen = false;
											resetFilter();
											onDisabledClick?.(displayComponent.getId(option));
										} else {
											handleSelect(displayComponent.getId(option));
										}
									}}
									class="select-option w-full px-3 py-3 text-left transition-colors
                       {isDisabled
										? isClickableDisabled
											? 'cursor-pointer'
											: 'cursor-not-allowed opacity-60'
										: ''}
                       {!isLastInGroup || !isLastGroup ? 'border-b' : ''}"
									style={!isLastInGroup || !isLastGroup ? 'border-color: var(--color-border)' : ''}
								>
									<ListSelectItem {context} item={option} {displayComponent} staticTags={true} />
								</button>
							{/each}
						{/if}
					{/each}
					{#if loading && hasMore}
						<div class="text-tertiary px-3 py-2 text-center text-sm">{common_loading()}</div>
					{/if}
				{/if}
			</div>
			{#if canScrollDown}
				<div
					class="pointer-events-none absolute inset-x-0 bottom-0 z-10 h-6 rounded-b-md bg-gradient-to-t from-[var(--color-bg-elevated)] to-transparent"
				></div>
			{/if}
		</div>
	</div>
{/if}
