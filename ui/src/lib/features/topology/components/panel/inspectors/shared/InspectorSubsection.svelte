<script lang="ts">
	import type { Snippet } from 'svelte';
	import { ChevronDown, ChevronRight } from 'lucide-svelte';
	import type { IconComponent } from '$lib/shared/utils/types';
	import { common_collapse, common_expand } from '$lib/paraglide/messages';
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
			<ChevronRight class="text-tertiary h-3.5 w-3.5 flex-shrink-0" />
		{:else}
			<ChevronDown class="text-tertiary h-3.5 w-3.5 flex-shrink-0" />
		{/if}
		{#if icon}
			{@const Icon = icon}
			<Icon class="h-4 w-4 flex-shrink-0 {iconClass}" />
		{/if}
		<span class="text-secondary truncate text-sm font-medium">{title}</span>
		{#if count !== undefined}
			<span class="text-tertiary flex-shrink-0 text-xs tabular-nums">{count}</span>
		{/if}
	</button>
	{#if !collapsed}
		<div class="border-border ml-[7px] mt-1 space-y-1 border-l pl-3">
			{#if description}
				<p class="text-tertiary text-xs">{description}</p>
			{/if}
			{@render children()}
		</div>
	{/if}
</div>
