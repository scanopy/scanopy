<!-- Stands in for the tags a row had no room for ("+2 tags"). Hovering, clicking or pressing Enter
     opens a popover with the tags themselves: their colours, tooltips and entity popovers. Once the
     pointer is inside, it stays open until a click outside, Escape or a scroll, so a tag's own
     popover can be reached from it. Static contexts get the tag names as a plain tooltip. -->
<script lang="ts">
	import Popover from '$lib/shared/components/data/Popover.svelte';
	import type { TagProps } from '$lib/shared/components/data/types';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import { common_moreTags, common_oneMoreTag } from '$lib/paraglide/messages';
	import DisplayTag from './DisplayTag.svelte';

	let { tags, interactive }: { tags: TagProps[]; interactive: boolean } = $props();

	let label = $derived(
		tags.length === 1 ? common_oneMoreTag() : common_moreTags({ count: tags.length })
	);
	let names = $derived(
		tags
			.map((tag) => tag.label ?? tag.title ?? '')
			.filter(Boolean)
			.join('\n')
	);

	let triggerEl: HTMLSpanElement | undefined = $state();
	let isOpen = $state(false);
	// Opened by click or keyboard: the popover takes focus, so a keyboard user can reach its tags.
	// Opened by hover it leaves focus alone, so hovering a chip in a picker keeps the search box.
	let takesFocus = $state(false);
	let popoverEntered = false;
	let openTimeout: ReturnType<typeof setTimeout> | undefined;
	let closeTimeout: ReturnType<typeof setTimeout> | undefined;

	function open() {
		clearTimeout(closeTimeout);
		isOpen = true;
	}

	function close() {
		clearTimeout(openTimeout);
		clearTimeout(closeTimeout);
		popoverEntered = false;
		takesFocus = false;
		isOpen = false;
	}

	function handleMouseEnter() {
		clearTimeout(closeTimeout);
		openTimeout = setTimeout(open, 300);
	}

	function handleMouseLeave() {
		clearTimeout(openTimeout);
		closeTimeout = setTimeout(() => {
			if (!popoverEntered) close();
		}, 150);
	}

	function toggle(e: Event) {
		e.stopPropagation();
		e.preventDefault();
		// A click on a chip whose popover hover already opened keeps it open and moves focus in;
		// only a second click closes it.
		if (isOpen && takesFocus) {
			close();
			return;
		}
		takesFocus = true;
		open();
	}
</script>

{#if interactive}
	<span
		bind:this={triggerEl}
		role="button"
		tabindex="0"
		aria-haspopup="dialog"
		aria-expanded={isOpen}
		class="text-tertiary hover:text-secondary cursor-pointer whitespace-nowrap text-xs transition-colors"
		onmouseenter={handleMouseEnter}
		onmouseleave={handleMouseLeave}
		onclick={toggle}
		onkeydown={(e) => {
			if (e.key === 'Enter' || e.key === ' ') toggle(e);
		}}>{label}</span
	>
	<Popover
		triggerElement={triggerEl}
		{isOpen}
		role={takesFocus ? 'dialog' : 'tooltip'}
		ariaLabel={label}
		onClose={close}
		onMouseEnter={() => {
			clearTimeout(closeTimeout);
			popoverEntered = true;
		}}
	>
		<div class="flex flex-wrap items-center gap-1">
			{#each tags as tag, i (`${tag.label}-${i}`)}
				<DisplayTag {tag} {interactive} />
			{/each}
		</div>
	</Popover>
{:else}
	<span use:tooltip data-tooltip={names || null} class="text-tertiary whitespace-nowrap text-xs"
		>{label}</span
	>
{/if}
