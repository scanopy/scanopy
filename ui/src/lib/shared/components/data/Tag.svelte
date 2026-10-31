<script lang="ts">
	import { X } from '@lucide/svelte';
	import { createColorHelper, type Color } from '$lib/shared/utils/styling';
	import { common_remove, common_unknown } from '$lib/paraglide/messages';
	import type { IconComponent } from '$lib/shared/utils/types';
	import { tooltip, type TooltipOptions } from '$lib/shared/actions/tooltip';
	import type { Snippet } from 'svelte';

	let {
		icon = null,
		color = 'Gray',
		disabled = false,
		label = null,
		badge = '',
		href = '',
		pill = false,
		title = '',
		removable = false,
		isShiny = false,
		upgrade = false,
		nativeTooltip = false,
		faded = false,
		onclick = undefined,
		onmouseenter = undefined,
		onmouseleave = undefined,
		onRemove,
		leading
	}: {
		icon?: IconComponent | null;
		color?: Color | null;
		disabled?: boolean;
		label?: string | null;
		badge?: string;
		href?: string;
		pill?: boolean;
		/**
		 * Hover text, or structured tooltip content, or a function that builds it on hover. A native
		 * tooltip shows text only.
		 */
		title?: string | TooltipOptions['content'];
		removable?: boolean;
		isShiny?: boolean;
		/** A paid feature's badge (`upgradeTagProps`): an amber chip around its crown, instead of `color`. */
		upgrade?: boolean;
		nativeTooltip?: boolean;
		faded?: boolean;
		onclick?: ((e: MouseEvent) => void) | undefined;
		onmouseenter?: ((e: MouseEvent) => void) | undefined;
		onmouseleave?: ((e: MouseEvent) => void) | undefined;
		onRemove?: () => void;
		/** Drawn inside the tag ahead of its icon, filling its left end: a container title's collapse
		 *  button, so the title stays the same chip as any other reference to its entity. */
		leading?: Snippet;
	} = $props();

	let interactive = $derived(!!href || !!onclick);
	let titleText = $derived(typeof title === 'string' ? title : '');
	let tooltipContent = $derived(
		nativeTooltip || !title || typeof title === 'string' ? null : title
	);
	let fadedClasses = $derived(
		faded ? 'opacity-50 grayscale hover:opacity-75 hover:grayscale-50 dark:opacity-40' : ''
	);

	// An icon carries the meaning on its own, so a tag with one is not unknown for lacking text.
	// Without either there is nothing to show, which is what `common_unknown` is for.
	let isUnknown = $derived((!label && !icon) || (!color && !upgrade));
	// Null collapses the text element entirely rather than rendering an empty span.
	let displayLabel = $derived(label ?? (icon ? null : common_unknown()));
	let colorHelper = $derived(color ? createColorHelper(color) : null);
	let bgColor = $derived(
		upgrade
			? 'border border-amber-300 bg-amber-100 dark:border-amber-400/45 dark:bg-amber-500/15'
			: (colorHelper?.bg ?? '')
	);
	let textColor = $derived(upgrade ? '' : (colorHelper?.text ?? ''));

	/**
	 * Names the full label on hover, but only while the chip is too narrow to
	 * show it. A table column narrower than the label truncates it.
	 */
	function titleWhenTruncated(node: HTMLElement, label: string) {
		let current = label;
		const onEnter = () => {
			node.title = node.scrollWidth > node.clientWidth ? current : '';
		};
		node.addEventListener('mouseenter', onEnter);
		return {
			update(next: string) {
				current = next;
			},
			destroy() {
				node.removeEventListener('mouseenter', onEnter);
			}
		};
	}

	let unknownClasses = 'bg-gray-200 text-gray-600 dark:bg-gray-600 dark:text-gray-300';
</script>

{#snippet content()}
	<span
		class="inline-flex max-w-full min-w-0 items-center gap-1 {pill
			? 'rounded-full'
			: 'rounded-sm'} {leading
			? 'pr-2'
			: displayLabel
				? 'px-2'
				: 'px-1.5'} py-0.5 text-xs font-medium
		{isUnknown ? unknownClasses : disabled ? 'bg-gray-700/30 text-tertiary' : `${bgColor} ${textColor}`}
		{isShiny ? 'tag-shiny' : ''}"
	>
		{@render leading?.()}
		{#if icon}
			{@const Icon = icon}
			<Icon size={16} class="{textColor} shrink-0" />
		{/if}

		{#if displayLabel}
			<span class="truncate" use:titleWhenTruncated={displayLabel}>{displayLabel}</span>
		{/if}
		{#if badge.length > 0}
			<span class="shrink-0 {textColor}">{badge}</span>
		{/if}
		{#if removable && onRemove}
			<!-- Tab reaches it, and Delete or Backspace removes the tag like a click. -->
			<button
				type="button"
				data-tag-remove
				aria-label={common_remove()}
				onclick={onRemove}
				onkeydown={(e) => {
					if (e.key === 'Delete' || e.key === 'Backspace') {
						e.preventDefault();
						onRemove();
					}
				}}
				class="rounded-full p-0.5 transition-colors hover:bg-white/20 focus:outline-hidden focus-visible:ring-2 focus-visible:ring-blue-500"
			>
				<X class="h-3 w-3" />
			</button>
		{/if}
	</span>
{/snippet}

<!-- eslint-disable svelte/no-navigation-without-resolve -->
{#if href}
	<a
		{href}
		target="_blank"
		rel="noopener noreferrer"
		use:tooltip={{ content: tooltipContent }}
		data-tooltip={nativeTooltip ? null : titleText || null}
		title={nativeTooltip ? titleText || undefined : undefined}
		class="inline-flex max-w-full shrink-0 items-center gap-1 rounded-sm whitespace-nowrap brightness-100 transition-all hover:brightness-90 dark:hover:brightness-125 {fadedClasses}"
		onclick={(e) => e.stopPropagation()}
	>
		{@render content()}
	</a>
{:else if interactive}
	<button
		type="button"
		use:tooltip={{ content: tooltipContent }}
		data-tooltip={nativeTooltip ? null : titleText || null}
		title={nativeTooltip ? titleText || undefined : undefined}
		class="inline-flex max-w-full shrink-0 cursor-pointer appearance-none items-center gap-1 rounded-sm whitespace-nowrap brightness-100 transition-all hover:brightness-90 dark:hover:brightness-125 {fadedClasses}"
		{onclick}
		{onmouseenter}
		{onmouseleave}
		{disabled}
	>
		{@render content()}
	</button>
{:else}
	<div
		use:tooltip={{ content: tooltipContent }}
		data-tooltip={nativeTooltip ? null : titleText || null}
		title={nativeTooltip ? titleText || undefined : undefined}
		class="inline-flex max-w-full shrink-0 items-center gap-1 rounded-sm whitespace-nowrap {fadedClasses}"
	>
		{@render content()}
	</div>
{/if}

<style>
	:global(.tag-shiny) {
		position: relative;
		overflow: hidden;
	}

	:global(.tag-shiny)::after {
		content: '';
		position: absolute;
		inset: 0;
		background: linear-gradient(
			120deg,
			transparent 0%,
			transparent 30%,
			rgba(255, 255, 255, 0.08) 45%,
			rgba(255, 255, 255, 0.12) 50%,
			rgba(255, 255, 255, 0.08) 55%,
			transparent 70%,
			transparent 100%
		);
		transform: translateX(-100%);
		pointer-events: none;
	}

	:global(.tag-shiny:hover)::after {
		animation: tag-sheen-hover 0.6s ease-out forwards;
	}

	@keyframes tag-sheen {
		from {
			transform: translateX(-100%);
		}
		to {
			transform: translateX(100%);
		}
	}

	@keyframes tag-sheen-hover {
		from {
			transform: translateX(-100%);
			opacity: 0.4;
		}
		to {
			transform: translateX(100%);
			opacity: 0.4;
		}
	}
</style>
