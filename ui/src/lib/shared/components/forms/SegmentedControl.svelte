<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { IconComponent } from '#lib/shared/utils/types.ts';
	import { tooltip } from '#lib/shared/actions/tooltip.ts';
	import Popover from '#lib/shared/components/data/Popover.svelte';

	interface Option {
		value: string;
		label: string;
		icon?: IconComponent;
		tooltip?: string;
		/**
		 * A tooltip that can hold a link: why the option is unavailable and what to
		 * do about it. Opens on hover or tap and stays open while the pointer is
		 * inside it, so its link can be reached. Takes the place of `tooltip`.
		 */
		popover?: Snippet;
		/** Render this one option non-selectable, leaving the rest interactive. */
		disabled?: boolean;
	}

	let {
		options,
		selected,
		onchange,
		size = 'sm',
		iconSize: iconSizeProp,
		disabled = false,
		fullWidth = false
	}: {
		options: Option[];
		selected: string;
		onchange: (value: string) => void;
		size?: 'sm' | 'md';
		iconSize?: 'sm' | 'md' | 'lg';
		disabled?: boolean;
		fullWidth?: boolean;
	} = $props();

	let sizeClasses = $derived(size === 'sm' ? 'px-2 py-1 text-xs' : 'px-3 py-1.5 text-sm');

	// One popover for the control, moved to whichever option is hovered.
	let popoverAnchor = $state<HTMLElement | null>(null);
	let popoverContent = $state<Snippet | null>(null);
	let popoverEntered = false;
	let openTimeout: ReturnType<typeof setTimeout> | undefined;
	let closeTimeout: ReturnType<typeof setTimeout> | undefined;

	function openPopover(anchor: HTMLElement, content: Snippet) {
		clearTimeout(closeTimeout);
		popoverAnchor = anchor;
		popoverContent = content;
	}

	function closePopover() {
		clearTimeout(openTimeout);
		clearTimeout(closeTimeout);
		popoverEntered = false;
		popoverContent = null;
	}

	function handlePointerEnter(e: PointerEvent, option: Option) {
		if (!option.popover || e.pointerType === 'touch') return;
		const anchor = e.currentTarget as HTMLElement;
		const content = option.popover;
		clearTimeout(closeTimeout);
		openTimeout = setTimeout(() => openPopover(anchor, content), 300);
	}

	// A touch pointer leaves as soon as the finger lifts, so a tap-opened popover
	// would close the moment it appeared. It closes on a tap outside instead.
	function handlePointerLeave(e?: PointerEvent) {
		if (e?.pointerType === 'touch') return;
		clearTimeout(openTimeout);
		closeTimeout = setTimeout(() => {
			if (!popoverEntered) closePopover();
		}, 150);
	}

	// A touch has no hover, so a tap opens it. Pointer events still reach the wrapper
	// of a disabled option, where clicks do not.
	function handlePointerDown(e: PointerEvent, option: Option) {
		if (!option.popover || e.pointerType !== 'touch') return;
		openPopover(e.currentTarget as HTMLElement, option.popover);
	}

	const iconSizeMap = { sm: 'h-3.5 w-3.5', md: 'h-4 w-4', lg: 'h-5 w-5' };
	let iconSizeClass = $derived(
		iconSizeProp ? iconSizeMap[iconSizeProp] : size === 'sm' ? 'h-3.5 w-3.5' : 'h-4 w-4'
	);
</script>

<div
	class="rounded-md border border-gray-600"
	class:inline-flex={!fullWidth}
	class:flex={fullWidth}
	class:opacity-50={disabled}
	class:cursor-not-allowed={disabled}
>
	{#each options as option (option.value)}
		{@const optionDisabled = disabled || option.disabled === true}
		<!-- The tooltip sits on the wrapper, not the button: a disabled button fires no
		     mouseenter, so an option explaining why it can't be picked would never show it. -->
		<span
			role="presentation"
			class="inline-flex"
			class:flex-1={fullWidth}
			use:tooltip
			data-tooltip={option.popover ? null : option.tooltip || null}
			onpointerenter={(e) => handlePointerEnter(e, option)}
			onpointerleave={handlePointerLeave}
			onpointerdown={(e) => handlePointerDown(e, option)}
		>
			<button
				type="button"
				disabled={optionDisabled}
				class="{sizeClasses} flex items-center justify-center gap-1 transition-colors {selected ===
				option.value
					? 'bg-blue-600 text-white'
					: 'text-secondary hover:text-primary'} {option.disabled
					? 'cursor-not-allowed opacity-50'
					: ''}"
				class:w-full={fullWidth}
				onclick={() => {
					if (!optionDisabled) onchange(option.value);
				}}
			>
				{#if option.icon}
					{@const Icon = option.icon}
					<Icon class={iconSizeClass} />
				{/if}
				{#if option.label}
					{option.label}
				{/if}
			</button>
		</span>
	{/each}
</div>

<!-- Mounted only where an option has one: the popover portals into the body. -->
{#if options.some((option) => option.popover)}
	<Popover
		triggerElement={popoverAnchor}
		isOpen={popoverContent != null}
		onClose={closePopover}
		onMouseEnter={() => {
			clearTimeout(closeTimeout);
			popoverEntered = true;
		}}
		onMouseLeave={() => {
			popoverEntered = false;
			handlePointerLeave();
		}}
	>
		{#if popoverContent}
			{@render popoverContent()}
		{/if}
	</Popover>
{/if}
