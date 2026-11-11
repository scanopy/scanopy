<script lang="ts">
	import type { Snippet } from 'svelte';
	import { ChevronDown, ChevronRight } from '@lucide/svelte';
	import type { IconComponent } from '#lib/shared/utils/types.ts';
	import { inspectorSections } from '#lib/shared/stores/metadata.ts';
	import { common_collapse, common_expand } from '#lib/paraglide/messages.js';
	import { collapsedInspectorSections, toggleInspectorSection } from './section-collapse';

	/**
	 * One section of the inspector: a collapsible heading that names what the section holds, with
	 * a one-line description of why it is there, then the body.
	 *
	 * `section` reads the heading from the backend's `InspectorSection` metadata. Pass `title`,
	 * `icon`, `iconClass` or `description` to override it (a dynamic title such as "Services Bound
	 * to IP Address"), or to head a section with no `InspectorSection` (edge inspectors).
	 */
	let {
		id,
		section,
		title,
		icon,
		iconClass,
		description,
		count,
		glyph,
		actions,
		children
	}: {
		/** Collapse key; persisted, shared by every node that shows this section. */
		id: string;
		section?: string;
		title?: string;
		icon?: IconComponent;
		iconClass?: string;
		description?: string | null;
		count?: number;
		/** Drawn in place of the icon: the selection's own glyph, for a section about one entity. */
		glyph?: Snippet;
		/** Controls on the heading's right, outside the collapse button (e.g. focus). */
		actions?: Snippet;
		children: Snippet;
	} = $props();

	let resolvedTitle = $derived(title ?? (section ? inspectorSections.getName(section) : ''));
	let resolvedDescription = $derived(
		description !== undefined
			? description
			: section
				? inspectorSections.getDescription(section)
				: null
	);
	let Icon = $derived(icon ?? (section ? inspectorSections.getIconComponent(section) : null));
	let resolvedIconClass = $derived(
		iconClass ?? (section ? inspectorSections.getColorHelper(section).icon : 'text-tertiary')
	);
	let collapsed = $derived($collapsedInspectorSections.has(id));
</script>

<section>
	<div class="flex items-center gap-2">
		<button
			type="button"
			class="flex min-w-0 flex-1 cursor-pointer items-center gap-1.5 text-left"
			aria-expanded={!collapsed}
			aria-label={collapsed ? common_expand() : common_collapse()}
			onclick={() => toggleInspectorSection(id)}
		>
			{#if collapsed}
				<ChevronRight class="h-4 w-4 shrink-0 text-tertiary" />
			{:else}
				<ChevronDown class="h-4 w-4 shrink-0 text-tertiary" />
			{/if}
			{#if glyph}
				{@render glyph()}
			{:else if Icon}
				<Icon class="h-4 w-4 shrink-0 {resolvedIconClass}" />
			{/if}
			<span class="truncate text-sm font-semibold text-primary">{resolvedTitle}</span>
			{#if count !== undefined}
				<span
					class="bg-surface-secondary shrink-0 rounded-full px-1.5 text-xs text-tertiary tabular-nums"
				>
					{count}
				</span>
			{/if}
		</button>
		{#if actions}
			{@render actions()}
		{/if}
	</div>
	{#if !collapsed}
		{#if resolvedDescription}
			<p class="mt-0.5 mb-2 pl-[22px] text-xs text-tertiary">{resolvedDescription}</p>
		{:else}
			<div class="mb-2"></div>
		{/if}
		{@render children()}
	{/if}
</section>
