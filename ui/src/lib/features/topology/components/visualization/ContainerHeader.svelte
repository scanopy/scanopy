<script lang="ts" module>
	/** Flow units the External title pill sits above its container's top edge. */
	export const EXTERNAL_TITLE_OFFSET = 40;
</script>

<script lang="ts">
	import { onMount } from 'svelte';
	import { Search } from '@lucide/svelte';
	// ChevronDown / ChevronRight replaced by the CSS `.caret` below — see the note on that rule.
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import HiddenTagsChip from '$lib/shared/components/forms/selection/HiddenTagsChip.svelte';
	import { HIDDEN_TAGS_CHIP_WIDTH } from '$lib/shared/components/forms/selection/display-tags';
	import type { ColorStyle, Color } from '$lib/shared/utils/styling';
	import type { IconComponent } from '$lib/shared/utils/types';
	import type { TagProps } from '$lib/shared/components/data/types';
	import type { components } from '$lib/api/schema';
	import DeviceGlyph from '$lib/features/hosts/components/DeviceGlyph.svelte';
	import SoftwareGlyph from './SoftwareGlyph.svelte';
	import {
		common_collapse,
		common_expand,
		topology_elementCount,
		topology_ungroupedCount,
		topology_searchContainerMatches
	} from '$lib/paraglide/messages';
	import {
		tooltip as tooltipAction,
		type TooltipContent,
		type TooltipOptions
	} from '$lib/shared/actions/tooltip';

	/** A group container's pill: a category or tag the group collects, with its tooltip. */
	export type GroupPill = { label: string; color: Color; title?: string | TooltipContent };

	export type SubgroupRow = {
		/** Service definition whose logo marks the subgroup, drawn round as software. */
		logoDefinition: string | null;
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
		logoDefinition = null,
		deviceType = null,
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
		staleTag = null,
		tooltip = null,
		entityColor = null,
		chipIcon = null
	}: {
		isCollapsed: boolean;
		isCollapsible: boolean;
		headerText: string;
		iconComponent: IconComponent | null;
		/** Service definition whose logo marks the container, drawn round as software. */
		logoDefinition: string | null;
		/**
		 * The device type a container standing for a device draws, as the same square tile as its icon card,
		 * in place of its icon. Expanded or collapsed, the tile holds the header's identity slot.
		 */
		deviceType?: components['schemas']['DeviceType'] | null;
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
		/** Hover text for the container's name: the entity it stands for, or what the group holds. */
		tooltip?: TooltipOptions['content'];
		/** The colour of the entity a subcontainer stands for, which draws its title as that entity's pill. Null for a grouping. */
		entityColor?: Color | null;
		/** The chip icon of the entity the container stands for, as `EntityTag` draws it. */
		chipIcon?: IconComponent | null;
	} = $props();

	let subgroupTotal = $derived(subgroupSummaries.reduce((sum, s) => sum + s.childCount, 0));
	let ungroupedCount = $derived(childCount - subgroupTotal);

	// Tag truncation for inline variant: measure available space, show the (i) icon for overflow
	const TAG_GAP = 4;
	const MORE_WIDTH = HIDDEN_TAGS_CHIP_WIDTH;
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
			// Recount on the next frame, not inside the delivery: showing or hiding a label resizes
			// the observed container, which inside the callback raises the ResizeObserver loop error.
			let frame = 0;
			const observer = new ResizeObserver(() => {
				cancelAnimationFrame(frame);
				frame = requestAnimationFrame(calculateVisibleLabels);
			});
			observer.observe(inlineContainerEl);
			return () => {
				observer.disconnect();
				cancelAnimationFrame(frame);
			};
		}
	});

	let visibleLabels = $derived(groupLabels.slice(0, visibleLabelCount));
	let hiddenLabelCount = $derived(groupLabels.length - visibleLabelCount);
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

<!-- The chevron collapses; the rest of the title chip selects the container (through the flow's node
     click, which only selects a container when the click lands in `data-container-title`). -->
{#snippet collapseToggle(hitArea: string)}
	<button
		type="button"
		class="flex shrink-0 items-center self-stretch transition-colors hover:bg-black/5 dark:hover:bg-white/10 {hitArea}"
		aria-expanded={!isCollapsed}
		aria-label={isCollapsed ? common_expand() : common_collapse()}
		onclick={onToggleCollapse}
		onpointerdown={(e) => e.stopPropagation()}
	>
		<span class="caret" class:caret-collapsed={isCollapsed}></span>
	</button>
{/snippet}

<!-- The status of the entity the container stands for (Unconfirmed, Stale) as an icon badge after
     its title, outside the title's pill. The tooltip says which and why. -->
{#snippet statusBadge()}
	{#if staleTag}
		<span data-fixed class="flex shrink-0"><Tag {...staleTag} label={undefined} /></span>
	{/if}
{/snippet}

{#snippet pillToggle()}
	{@render collapseToggle('-my-0.5 rounded-l-full py-0.5 pr-0.5 pl-2')}
{/snippet}

<!-- The title of a container standing for an entity (a subnet, a host, a VLAN, a hypervisor): the
     same chip as any other reference to that entity (`Tag`, its chip icon and colour), with the
     two things a container adds: the chevron filling its rounded left end, and a click that opens
     the inspector. -->
{#snippet entityPill(suffix: string)}
	<!-- The tooltip goes to the Tag itself: a hover finds the nearest tooltip source, which is the
	     Tag's own, so one set on this wrapper would never show. -->
	<span data-fixed data-container-title class="flex min-w-0 shrink cursor-pointer">
		<Tag
			pill
			color={entityColor}
			icon={chipIcon}
			label={`${headerText}${suffix}`}
			title={tooltip}
			leading={isCollapsible ? pillToggle : undefined}
		/>
	</span>
{/snippet}

<!-- A subcontainer's title, which selects the subcontainer: an entity's chip, or for a grouping (a
     stack, a tag) a plain title. `withIcon` draws a grouping's type icon, which a collapsed box
     shows in place of its children. -->
{#snippet subcontainerTitle(hasLabels: boolean, withIcon = false)}
	{#if entityColor}
		{@render entityPill(hasLabels ? ':' : '')}
	{:else}
		<span
			data-fixed
			data-container-title
			class="flex min-w-0 shrink cursor-pointer items-center gap-1 text-xs font-medium text-tertiary"
			use:tooltipAction={{ content: tooltip }}
		>
			{#if isCollapsible}
				{@render collapseToggle('rounded-sm px-0.5 text-secondary')}
			{/if}
			{#if deviceType}
				<DeviceGlyph {deviceType} />
			{:else if withIcon && iconComponent}
				{@const IconComp = iconComponent}
				<IconComp
					class={`h-3.5 w-3.5 shrink-0 ${colorHelper.icon}`}
					fill={fillIcon ? 'currentColor' : 'none'}
				/>
			{/if}
			{#if logoDefinition}
				<SoftwareGlyph definition={logoDefinition} framed={false} />
			{/if}
			{#if headerText}
				<span data-entity-header class="min-w-0 truncate whitespace-nowrap">
					{headerText}{hasLabels ? ':' : ''}
				</span>
			{/if}
		</span>
	{/if}
	{@render statusBadge()}
{/snippet}

{#if variant === 'external' && entityColor}
	<!-- External title above a container standing for an entity: its chip, then its status badge,
	     sitting just above the box's top edge whatever the chip's height. The pointer handlers only
	     keep a press on the title from panning the canvas. -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="nopan nodrag absolute bottom-full left-0 z-100 mb-2 flex max-w-full items-center gap-1"
		onmousedown={(e) => e.stopPropagation()}
		onpointerdown={(e) => e.stopPropagation()}
	>
		{@render entityPill('')}
		{@render statusBadge()}
	</div>
{:else if variant === 'external'}
	<!-- External title pill above a grouping container. Its pointer handlers only keep a press on
	     the pill from panning the canvas. -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		data-container-title
		class="nopan nodrag card absolute left-0 z-100 flex max-w-full cursor-pointer items-center gap-1 py-1 pr-2 text-secondary shadow-lg backdrop-blur-xs {isCollapsible
			? 'pl-0'
			: 'pl-2'}"
		style:top="-{EXTERNAL_TITLE_OFFSET}px"
		use:tooltipAction={{ content: tooltip }}
		onmousedown={(e) => e.stopPropagation()}
		onpointerdown={(e) => e.stopPropagation()}
	>
		{#if isCollapsible}
			<!-- Fills the pill's left end, so the whole rounded end collapses it. -->
			{@render collapseToggle('-my-1 rounded-l-[inherit] py-1 pr-1 pl-2 text-secondary')}
		{/if}

		{#if deviceType}
			<DeviceGlyph {deviceType} />
		{:else if logoDefinition}
			<SoftwareGlyph definition={logoDefinition} framed={false} />
		{:else if iconComponent}
			{@const IconComp = iconComponent}
			<IconComp class={`h-5 w-5 ${colorHelper.icon}`} fill={fillIcon ? 'currentColor' : 'none'} />
		{/if}

		{#if headerText}
			<span data-entity-header class="text-s min-w-0 truncate font-medium text-secondary">
				{headerText}
			</span>
		{/if}
	</div>
{:else if variant === 'inline'}
	<!-- Inline header: the title inside the container's top padding, then the group's labels. Above
	     the container's body, which is drawn after it and would otherwise take its clicks. -->
	<div
		bind:this={inlineContainerEl}
		class="nopan nodrag absolute top-2 right-2 left-2 z-10 flex items-center gap-1 overflow-hidden py-0.5 text-secondary"
	>
		{#if isCollapsible || deviceType || logoDefinition || headerText}
			{@render subcontainerTitle(groupLabels.length > 0)}
		{/if}
		{#each visibleLabels as pill (pill.label)}
			<Tag label={pill.label} color={pill.color} title={pill.title ?? ''} />
		{/each}
		{#if hiddenLabelCount > 0}
			<HiddenTagsChip tags={groupLabels.slice(visibleLabelCount)} interactive />
		{/if}
	</div>
{:else if variant === 'collapsed-sub'}
	<!-- Collapsed subcontainer: compact inline with dashed border, its title chip leading -->
	<div
		bind:this={inlineContainerEl}
		class="nopan nodrag flex items-center gap-1 overflow-hidden rounded-lg border border-dashed border-gray-300 px-3 py-2 dark:border-gray-600"
		style="background: var(--color-topology-subgroup-bg); transition: box-shadow 0.15s ease-in-out; {searchHighlightRingStyle}"
	>
		{@render subcontainerTitle(groupLabels.length > 0, true)}
		{#each visibleLabels.slice(0, 2) as pill (pill.label)}
			<Tag label={pill.label} color={pill.color} title={pill.title ?? ''} />
		{/each}
		{#if visibleLabels.length > 2}
			<HiddenTagsChip tags={groupLabels.slice(2)} interactive />
		{:else if hiddenLabelCount > 0}
			<HiddenTagsChip tags={groupLabels.slice(visibleLabelCount)} interactive />
		{/if}
		{#if searchMatchCount > 0}
			<span
				data-fixed
				class="ml-auto flex items-center gap-0.5 rounded-full bg-blue-500/20 px-1.5 py-0.5 text-[10px] leading-none whitespace-nowrap text-blue-400"
			>
				<Search class="h-2.5 w-2.5" />
				{searchMatchCount}
			</span>
		{/if}
		{#if !hideCount}
			<span
				data-fixed
				class="text-xs whitespace-nowrap text-tertiary"
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
		<div class="flex min-w-fit flex-col items-center gap-2 px-6 py-4 whitespace-nowrap">
			<!-- The container's name is not repeated here: a collapsed root container is named by
			     its external title pill, which stays visible above the box in both states. -->
			<span
				class="text-base font-medium text-secondary underline"
				use:tooltipAction={{ content: tooltip }}
			>
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
				<span class="text-xs text-tertiary">
					{topology_ungroupedCount({ summary: ungroupedSummary })}
				</span>
			{/if}
			{#each subgroupSummaries as summary, i (i)}
				<div
					class="flex items-center gap-1 rounded-md border border-dashed border-gray-300 px-2 py-1 whitespace-nowrap dark:border-gray-600"
					style="background: var(--color-topology-subgroup-bg);"
				>
					{#if summary.logoDefinition}
						<SoftwareGlyph definition={summary.logoDefinition} framed={false} />
					{/if}
					{#if summary.headerText}
						<span class="text-xs text-tertiary"
							>{summary.headerText}{summary.labels.length > 0 ? ':' : ''}</span
						>
					{/if}
					{#each summary.labels.slice(0, 2) as pill, j (j)}
						<Tag label={pill.label} color={pill.color} title={pill.title ?? ''} />
					{/each}
					{#if summary.labels.length > 2}
						<HiddenTagsChip tags={summary.labels.slice(2)} interactive />
					{/if}
					{#if !summary.hideCount}
						<span class="text-xs text-tertiary">
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
