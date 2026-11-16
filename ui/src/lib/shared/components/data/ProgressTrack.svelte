<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';

	let {
		progress = 0,
		color = 'bg-blue-500',
		size = 'md',
		class: className = '',
		children,
		...rest
	}: {
		progress?: number;
		color?: string;
		size?: 'sm' | 'md';
		class?: string;
		children?: Snippet;
	} & Omit<HTMLAttributes<HTMLDivElement>, 'class' | 'children'> = $props();

	let heightClass = $derived(size === 'sm' ? 'h-1.5' : 'h-2');
</script>

<div
	{...rest}
	class="{heightClass} overflow-hidden rounded-full bg-gray-200 dark:bg-gray-700 {className}"
>
	{#if children}
		{@render children()}
	{:else}
		<div
			class="h-full rounded-full transition-all {color}"
			style="width: {Math.min(progress, 100)}%"
		></div>
	{/if}
</div>
