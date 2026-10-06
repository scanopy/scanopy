<script lang="ts">
	import type { Snippet } from 'svelte';
	import KbdKey from '$lib/shared/components/feedback/KbdKey.svelte';

	let {
		onclick,
		title,
		icon,
		label = '',
		shortcut,
		reserveShortcutWidth = false,
		active = false,
		collapsed = false,
		disabled = false,
		grouped
	}: {
		onclick: () => void;
		title: string;
		icon: Snippet;
		label?: string;
		shortcut?: string;
		reserveShortcutWidth?: boolean;
		active?: boolean;
		collapsed?: boolean;
		disabled?: boolean;
		grouped?: 'top' | 'middle' | 'bottom';
	} = $props();

	let hovered = $state(false);
	let hasLabel = $derived(label.length > 0);
	let showLabel = $derived(hasLabel && (!collapsed || hovered));
	let hasShortcut = $derived(!!(shortcut || reserveShortcutWidth));

	let roundingClass = $derived(
		grouped === 'top'
			? 'rounded-t'
			: grouped === 'bottom'
				? 'rounded-b'
				: grouped === 'middle'
					? ''
					: 'rounded'
	);

	/* Padding to center content:
	   With shortcut: 58px − 2px borders = 56px; icon(16)+gap(6)+kbd(20)=42px → (56−42)/2 = 7px
	   Icon only: centered via justify-center, uniform padding
	   A wider chip (`Ctrl F` off Apple platforms) grows the button past 58px rather than clipping. */
	let hPad = $derived(shortcut ? 7 : 0);
</script>

<div class="relative" style="min-width: {hasShortcut ? '58px' : '32px'};">
	<button
		class="flex w-full items-center justify-center overflow-hidden text-xs font-medium {roundingClass}
			{disabled
			? '!cursor-not-allowed !border !border-gray-200 !bg-gray-100 !text-gray-400 !opacity-40 !shadow-none dark:!border-gray-700 dark:!bg-gray-800 dark:!text-gray-600'
			: active
				? '!border !border-blue-400 !bg-blue-50 !text-blue-700 hover:!bg-blue-100 dark:!border-blue-500 dark:!bg-blue-900/40 dark:!text-blue-300 dark:hover:!bg-blue-800/50'
				: '!border !border-gray-300 !bg-gray-50 !text-gray-700 hover:!bg-gray-100 dark:!border-gray-600 dark:!bg-gray-700 dark:!text-gray-100 dark:hover:!bg-gray-600'}
		{grouped ? '' : '!shadow-lg'}"
		style="height: 32px; padding: 0 {hPad}px;"
		onclick={disabled ? undefined : onclick}
		aria-disabled={disabled || undefined}
		{title}
		onpointerenter={() => (hovered = true)}
		onpointerleave={() => (hovered = false)}
	>
		<span class="flex h-4 w-4 shrink-0 items-center justify-center overflow-hidden">
			{@render icon()}
		</span>
		{#if shortcut}
			<KbdKey key={shortcut} size="sm" class="ml-1.5 shrink-0 !shadow-none" />
		{/if}
	</button>
	{#if hasLabel}
		<span
			class="pointer-events-none absolute top-0 flex h-[32px] items-center whitespace-nowrap text-xs font-medium transition-all duration-300 ease-in-out
				{disabled
				? 'text-gray-400 dark:text-gray-600'
				: active
					? 'text-blue-700 dark:text-blue-300'
					: 'text-gray-700 dark:text-gray-100'}"
			style="right: 100%; overflow: hidden; max-width: {showLabel
				? '150px'
				: '0px'}; opacity: {showLabel ? 1 : 0}; padding-right: {showLabel ? '6px' : '0px'};"
		>
			{label}
		</span>
	{/if}
</div>
