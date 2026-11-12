<script lang="ts">
	import type { Snippet } from 'svelte';
	import { ChevronDown, ChevronRight } from '@lucide/svelte';
	import type { IconComponent } from '#lib/shared/utils/types.ts';
	import { common_collapse, common_expand } from '#lib/paraglide/messages.js';
	import { collapsedInspectorSections, toggleInspectorSection } from './section-collapse';

	/**
	 * A group inside an inspector section, such as a guest's Network Identities inside its
	 * Services: a smaller collapsible heading and an indented body under a left rule, so it reads
	 * as part of the section above it rather than as a sibling section.
	 */
	let {
		id,
		title,
		icon,
		iconClass = 'text-tertiary',
		description,
		count,
		children
	}: {
		/** Collapse key; persisted. Keep it stable across nodes (e.g. by service definition). */
		id: string;
		title: string;
		icon?: IconComponent | null;
		iconClass?: string;
		description?: string | null;
		count?: number;
		children: Snippet;
	} = $props();

	let collapsed = $derived($collapsedInspectorSections.has(id));
</script>

<div>
	<button
		type="button"
		class="flex w-full min-w-0 cursor-pointer items-center gap-1.5 text-left"
		aria-expanded={!collapsed}
		aria-label={collapsed ? common_expand() : common_collapse()}
		onclick={() => toggleInspectorSection(id)}
	>
		{#if collapsed}
			<ChevronRight class="h-3.5 w-3.5 shrink-0 text-tertiary" />
		{:else}
			<ChevronDown class="h-3.5 w-3.5 shrink-0 text-tertiary" />
		{/if}
		{#if icon}
			{@const Icon = icon}
			<Icon class="h-4 w-4 shrink-0 {iconClass}" />
		{/if}
		<span class="truncate text-sm font-medium text-secondary">{title}</span>
		{#if count !== undefined}
			<span class="shrink-0 text-xs text-tertiary tabular-nums">{count}</span>
		{/if}
	</button>
	{#if !collapsed}
		<div class="border-border mt-1 ml-[7px] space-y-1 border-l pl-3">
			{#if description}
				<p class="text-xs text-tertiary">{description}</p>
			{/if}
			{@render children()}
		</div>
	{/if}
</div>
