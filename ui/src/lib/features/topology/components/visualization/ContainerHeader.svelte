<script lang="ts">
	import { onMount } from 'svelte';
	import { Search } from 'lucide-svelte';
	// ChevronDown / ChevronRight replaced by the CSS `.caret` below — see the note on that rule.
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import type { ColorStyle, Color } from '$lib/shared/utils/styling';
	import type { IconComponent } from '$lib/shared/utils/types';
	import type { TagProps } from '$lib/shared/components/data/types';
	import {
		common_moreItems,
		topology_elementCount,
		topology_ungroupedCount,
		topology_searchContainerMatches
	} from '$lib/paraglide/messages';

	/** A group container's pill: a category or tag the group collects, with its tooltip. */
	export type GroupPill = { label: string; color: Color; title?: string };

	export type SubgroupRow = {
		logoComponent: IconComponent | null;
		headerText: string;
		labels: GroupPill[];
		childCount: number;
		/** Pre-formatted entity-count summary, e.g. "5 services" or "3 services, 2 hosts". */
		childSummary: string;
		hideCount?: boolean;
		countOnly?: boolean;
	};

	let {
		isCollapsed,
		isCollapsible,
		headerText,
		iconComponent = null,
		logoComponent = null,
		fillIcon = false,
		colorHelper,
		groupLabels = [],
		childCount = 0,
		childSummary = '',
		ungroupedSummary = '',
		onToggleCollapse,
		variant,
		subgroupSummaries = [],
		tagHoverRingStyle = '',
		hideCount = false,
		countOnly = false,
		searchMatchCount = 0,
		searchHighlightRingStyle = '',
		staleTag = null
	}: {
		isCollapsed: boolean;
		isCollapsible: boolean;
		headerText: string;
		iconComponent: IconComponent | null;
		logoComponent: IconComponent | null;
		fillIcon: boolean;
		colorHelper: ColorStyle;
		groupLabels: GroupPill[];
		childCount: number;
		/** Pre-formatted entity-count summary for the container's children, e.g. "5 services, 2 hosts". */
		childSummary: string;
		/** Pre-formatted summary for the ungrouped remainder (collapsed-root variant). */
		ungroupedSummary?: string;
		onToggleCollapse: (event: MouseEvent | KeyboardEvent) => void;
		variant: 'external' | 'inline' | 'collapsed-sub' | 'collapsed-root';
		subgroupSummaries?: SubgroupRow[];
		tagHoverRingStyle?: string;
		hideCount?: boolean;
		countOnly?: boolean;
		searchMatchCount?: number;
		searchHighlightRingStyle?: string;
		/** Staleness pill for the entity this container stands for, from `getFreshnessTag`. */
		staleTag?: TagProps | null;
	} = $props();

	let subgroupTotal = $derived(subgroupSummaries.reduce((sum, s) => sum + s.childCount, 0));
	let ungroupedCount = $derived(childCount - subgroupTotal);

	// Tag truncation for inline variant: measure available space, show "+X more" for overflow
	const TAG_GAP = 4;
	const MORE_WIDTH = 50;
	let inlineContainerEl: HTMLDivElement | undefined = $state(undefined);
	let inlineMeasureEl: HTMLDivElement | undefined = $state(undefined);
	// Derived from the label count by default; `calculateVisibleLabels`
	// reassigns this to a smaller value when measured width forces truncation.
	// Reassignment is reset on the next groupLabels change.
	let visibleLabelCount = $derived(groupLabels.length);

	function calculateVisibleLabels() {
		if (!inlineContainerEl || !inlineMeasureEl || groupLabels.length === 0) {
			visibleLabelCount = groupLabels.length;
			return;
		}

		const containerWidth = inlineContainerEl.offsetWidth;
		const tagEls = inlineMeasureEl.querySelectorAll('[data-tag]');
		const tagWidths: number[] = [];
		tagEls.forEach((el) => tagWidths.push((el as HTMLElement).offsetWidth));

		if (tagWidths.length === 0) {
			visibleLabelCount = groupLabels.length;
			return;
		}

		// Space used by non-tag elements (chevron, icon, header text)
		const fixedEls = inlineContainerEl.querySelectorAll('[data-fixed]');
		let fixedWidth = 0;
		fixedEls.forEach((el) => (fixedWidth += (el as HTMLElement).offsetWidth + TAG_GAP));

		let availableForTags = containerWidth - fixedWidth;
		let count = 0;
		let usedWidth = 0;

		for (let i = 0; i < tagWidths.length; i++) {
			const needsMore = i < tagWidths.length - 1;
			const extraWidth =
				(count > 0 ? TAG_GAP : 0) + tagWidths[i] + (needsMore ? TAG_GAP + MORE_WIDTH : 0);

			if (usedWidth + extraWidth <= availableForTags) {
				count++;
				usedWidth += (count > 1 ? TAG_GAP : 0) + tagWidths[i];
			} else {
				break;
			}
		}

		if (count < tagWidths.length && count > 1) {
			const totalWithMore = usedWidth + TAG_GAP + MORE_WIDTH;
			if (totalWithMore > availableForTags) {
				count--;
			}
		}

		visibleLabelCount = Math.max(count, 0);
	}

	let needsTruncation = $derived(
		(variant === 'inline' || variant === 'collapsed-sub') && groupLabels.length > 0
	);

	$effect(() => {
		if (needsTruncation && inlineContainerEl) {
			requestAnimationFrame(() => calculateVisibleLabels());
		}
	});

	onMount(() => {
		if (needsTruncation && inlineContainerEl) {
			const observer = new ResizeObserver(() => calculateVisibleLabels());
			observer.observe(inlineContainerEl);
			return () => observer.disconnect();
		}
	});

	let visibleLabels = $derived(groupLabels.slice(0, visibleLabelCount));
	let hiddenLabelCount = $derived(groupLabels.length - visibleLabelCount);

	/** The tooltip on a "+N more" pill: the names it stands for, one per line. */
	function pillList(pills: GroupPill[]): string {
		return pills.map((pill) => pill.label).join('\n');
	}
</script>

<!-- Hidden measurement container for tag widths (shared by inline + collapsed-sub) -->
{#if needsTruncation}
	<div
		bind:this={inlineMeasureEl}
		class="pointer-events-none invisible absolute flex items-center gap-1"
		aria-hidden="true"
	>
		{#each groupLabels as pill (pill.label)}
			<span data-tag><Tag label={pill.label} color={pill.color} title={pill.title ?? ''} /></span>
		{/each}
	</div>
{/if}

{#if variant === 'external'}
	<!-- External title pill: card above container -->
	<div
		class="nopan nodrag card text-secondary z-100 absolute -top-10 left-0 flex cursor-pointer items-center gap-1 px-2 py-1 shadow-lg backdrop-blur-sm"
		role="button"
		tabindex={-1}
		onclick={onToggleCollapse}
		onkeydown={(e) => {
			if (e.key === 'Enter' || e.key === ' ') onToggleCollapse(e);
		}}
		onmousedown={(e) => e.stopPropagation()}
		onpointerdown={(e) => e.stopPropagation()}
	>
		{#if isCollapsible}
			<span class="caret text-secondary" class:caret-collapsed={isCollapsed}></span>
		{/if}

		{#if iconComponent}
			{@const IconComp = iconComponent}
			<IconComp class={`h-5 w-5 ${colorHelper.icon}`} fill={fillIcon ? 'currentColor' : 'none'} />
		{/if}

		{#if headerText}
			<span data-entity-header class="text-s text-secondary whitespace-nowrap font-medium">
				{headerText}
			</span>
		{/if}

		{#if staleTag}
			<Tag {...staleTag} pill />
		{/if}
	</div>
{:else if variant === 'inline'}
	<!-- Inline header: inside container top padding -->
	<div
		bind:this={inlineContainerEl}
		class="nopan nodrag text-secondary absolute left-2 right-2 top-2 flex items-center gap-1 overflow-hidden rounded-t px-2 py-0.5"
	>
		{#if isCollapsible}
			<span data-fixed class="flex items-center"><span class="caret text-secondary"></span></span>
		{/if}
		{#if logoComponent}
			{@const LogoComp = logoComponent}
			<span data-fixed><LogoComp class="h-4 w-4 flex-shrink-0" /></span>
		{/if}
		{#if headerText}
			<span
				data-fixed
				data-entity-header
				class="text-tertiary flex-shrink-0 whitespace-nowrap text-xs font-medium"
			>
				{headerText}{groupLabels.length > 0 ? ':' : ''}
			</span>
		{/if}
		{#if staleTag}
			<span data-fixed class="flex-shrink-0"><Tag {...staleTag} pill /></span>
		{/if}
		{#each visibleLabels as pill (pill.label)}
			<Tag label={pill.label} color={pill.color} title={pill.title ?? ''} />
		{/each}
		{#if hiddenLabelCount > 0}
			<Tag
				label={common_moreItems({ count: hiddenLabelCount })}
				color="Gray"
				title={pillList(groupLabels.slice(visibleLabelCount))}
			/>
		{/if}
	</div>
{:else if variant === 'collapsed-sub'}
	<!-- Collapsed subcontainer: compact inline with dashed border -->
	<div
		bind:this={inlineContainerEl}
		class="nopan nodrag flex cursor-pointer items-center gap-1 overflow-hidden rounded-lg border border-dashed border-gray-300 px-3 py-2 dark:border-gray-600"
		style="background: var(--color-topology-subgroup-bg); transition: box-shadow 0.15s ease-in-out; {searchHighlightRingStyle}"
		role="button"
		tabindex={-1}
		onclick={onToggleCollapse}
		onkeydown={(e) => {
			if (e.key === 'Enter' || e.key === ' ') onToggleCollapse(e);
		}}
	>
		{#if isCollapsible}
			<span data-fixed class="flex items-center"
				><span class="caret caret-collapsed text-secondary"></span></span
			>
		{/if}
		{#if iconComponent}
			{@const IconComp = iconComponent}
			<span data-fixed
				><IconComp
					class={`h-3.5 w-3.5 flex-shrink-0 ${colorHelper.icon}`}
					fill={fillIcon ? 'currentColor' : 'none'}
				/></span
			>
		{/if}
		{#if logoComponent}
			{@const LogoComp = logoComponent}
			<span data-fixed><LogoComp class="h-4 w-4 flex-shrink-0" /></span>
		{/if}
		{#if headerText}
			<span
				data-fixed
				data-entity-header
				class="text-tertiary whitespace-nowrap text-xs font-medium"
			>
				{headerText}{groupLabels.length > 0 ? ':' : ''}
			</span>
		{/if}
		{#each visibleLabels.slice(0, 2) as pill (pill.label)}
			<Tag label={pill.label} color={pill.color} title={pill.title ?? ''} />
		{/each}
		{#if visibleLabels.length > 2}
			<Tag
				label={common_moreItems({ count: groupLabels.length - 2 })}
				color="Gray"
				title={pillList(groupLabels.slice(2))}
			/>
		{:else if hiddenLabelCount > 0}
			<Tag
				label={common_moreItems({ count: hiddenLabelCount })}
				color="Gray"
				title={pillList(groupLabels.slice(visibleLabelCount))}
			/>
		{/if}
		{#if searchMatchCount > 0}
			<span
				data-fixed
				class="ml-auto flex items-center gap-0.5 whitespace-nowrap rounded-full bg-blue-500/20 px-1.5 py-0.5 text-[10px] leading-none text-blue-400"
			>
				<Search class="h-2.5 w-2.5" />
				{searchMatchCount}
			</span>
		{/if}
		{#if !hideCount}
			<span
				data-fixed
				class="text-tertiary whitespace-nowrap text-xs"
				class:ml-auto={searchMatchCount === 0}
			>
				{#if countOnly}
					({childCount})
				{:else}
					({topology_elementCount({ summary: childSummary })})
				{/if}
			</span>
		{/if}
	</div>
{:else if variant === 'collapsed-root'}
	<!-- Collapsed root container: summary with subcontainer info -->
	<div
		class="rounded-xl text-center text-sm font-semibold shadow-lg"
		style="background: var(--color-topology-node-bg); position: relative; overflow: visible; transition: box-shadow 0.15s ease-in-out; border-top: 2px solid {colorHelper.rgb}; {tagHoverRingStyle}"
	>
		<div class="flex min-w-fit flex-col items-center gap-2 whitespace-nowrap px-6 py-4">
			<!-- The container's name is not repeated here: a collapsed root container is named by
			     its external title pill, which stays visible above the box in both states. -->
			<span class="text-secondary text-base font-medium underline">
				{topology_elementCount({ summary: childSummary })}
			</span>
			{#if searchMatchCount > 0}
				<span
					class="flex items-center gap-1 rounded-full bg-blue-500/20 px-2 py-0.5 text-xs text-blue-400"
				>
					<Search class="h-3 w-3" />
					{topology_searchContainerMatches({ count: String(searchMatchCount) })}
				</span>
			{/if}
			{#if ungroupedCount > 0 && subgroupSummaries.length > 0}
				<span class="text-tertiary text-xs">
					{topology_ungroupedCount({ summary: ungroupedSummary })}
				</span>
			{/if}
			{#each subgroupSummaries as summary, i (i)}
				<div
					class="flex items-center gap-1 whitespace-nowrap rounded-md border border-dashed border-gray-300 px-2 py-1 dark:border-gray-600"
					style="background: var(--color-topology-subgroup-bg);"
				>
					{#if summary.logoComponent}
						{@const GroupLogo = summary.logoComponent}
						<GroupLogo class="h-4 w-4 flex-shrink-0" />
					{/if}
					{#if summary.headerText}
						<span class="text-tertiary text-xs"
							>{summary.headerText}{summary.labels.length > 0 ? ':' : ''}</span
						>
					{/if}
					{#each summary.labels.slice(0, 2) as pill, j (j)}
						<Tag label={pill.label} color={pill.color} title={pill.title ?? ''} />
					{/each}
					{#if summary.labels.length > 2}
						<Tag
							label={common_moreItems({ count: summary.labels.length - 2 })}
							color="Gray"
							title={pillList(summary.labels.slice(2))}
						/>
					{/if}
					{#if !summary.hideCount}
						<span class="text-tertiary text-xs">
							{#if summary.countOnly}
								({summary.childCount})
							{:else}
								({topology_elementCount({ summary: summary.childSummary })})
							{/if}
						</span>
					{/if}
				</div>
			{/each}
		</div>
	</div>
{/if}

<style>
	/*
	 * Collapse caret drawn in CSS rather than as an icon component.
	 *
	 * `ChevronDown`/`ChevronRight` each render an `<svg>` plus a `<path>` — two elements on every
	 * container header, 1,152 of them on the seeded graph, for a shape that is two borders. Blink
	 * accounts for a DOM node and its style and layout objects whether or not anything reads them,
	 * and at this node count that is the cheapest whole category to remove.
	 */
	.caret {
		display: inline-block;
		flex-shrink: 0;
		width: 0.35rem;
		height: 0.35rem;
		border-right: 1.5px solid currentColor;
		border-bottom: 1.5px solid currentColor;
		/*
		 * Rotated +45° the two borders draw a "v" that fills only the lower half of the box
		 * (its point sits ~0.35 of the box's height below centre), so it is lifted by that much
		 * to sit on the text's centre line. Rotated -45° the ">" spans the full height and is
		 * already centred.
		 */
		transform: translateY(-35%) rotate(45deg);
		transition: transform 150ms ease-in-out;
	}
	.caret-collapsed {
		transform: translateY(0) rotate(-45deg);
	}
</style>
