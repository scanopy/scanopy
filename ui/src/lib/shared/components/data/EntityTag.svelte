<script lang="ts">
	import Tag from './Tag.svelte';
	import Popover from './Popover.svelte';
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { EntityRef } from './types';
	import type { Color } from '$lib/shared/utils/styling';
	import type { IconComponent } from '$lib/shared/utils/types';
	import { entityUIConfig } from '$lib/shared/entity-ui-config';
	import { navigateToEntity } from '$lib/shared/stores/modal-registry';
	import { getContext } from 'svelte';

	const isStaticTags = getContext<boolean>('staticTags') ?? false;

	let {
		entityRef,
		label,
		icon = null,
		color = 'Gray',
		disabled = false,
		badge = '',
		disablePopover = false,
		disableNavigate = false
	}: {
		entityRef: EntityRef;
		/** Omit to render the icon alone, as `Tag` does. */
		label?: string;
		icon?: IconComponent | null;
		color?: Color;
		disabled?: boolean;
		badge?: string;
		disablePopover?: boolean;
		/**
		 * When true, the tag doesn't navigate on click and drops the interactive
		 * cursor / hover-brightness treatment. Popover behavior is unaffected — use
		 * `disablePopover` for that.
		 */
		disableNavigate?: boolean;
	} = $props();

	let triggerEl: HTMLDivElement | undefined = $state();
	let isHovered = $state(false);
	let popoverHovered = $state(false);
	let hoverTimeout: ReturnType<typeof setTimeout> | undefined;
	let leaveTimeout: ReturnType<typeof setTimeout> | undefined;

	let config = $derived(entityUIConfig[entityRef.entityType]);
	let displayComponent = $derived(config?.displayComponent ?? null);

	function handleMouseEnter() {
		if (!displayComponent || disablePopover) return;
		clearTimeout(leaveTimeout);
		hoverTimeout = setTimeout(() => {
			isHovered = true;
		}, 300);
	}

	function handleMouseLeave() {
		clearTimeout(hoverTimeout);
		leaveTimeout = setTimeout(() => {
			if (!popoverHovered) {
				isHovered = false;
			}
		}, 150);
	}

	function handlePopoverEnter() {
		clearTimeout(leaveTimeout);
		popoverHovered = true;
	}

	function handlePopoverLeave() {
		popoverHovered = false;
		isHovered = false;
	}

	function handleClick(e: MouseEvent) {
		e.stopPropagation();
		e.preventDefault();
		if (disabled || isStaticTags) return;
		isHovered = false;
		navigateToEntity(entityRef.entityType, entityRef.entityId, entityRef.data);
	}
</script>

{#if isStaticTags}
	<span
		class="inline-flex max-w-full flex-shrink-0 items-center gap-1 whitespace-nowrap rounded-full"
	>
		<Tag {icon} {color} {disabled} {label} {badge} pill={true} />
	</span>
{:else if disableNavigate}
	<!-- Non-interactive (no click-to-navigate, no hover-brightness) but still tracks
	     mouseenter/leave so the popover can still trigger when !disablePopover.
	     Cursor is inherited from the parent (e.g. pointer when wrapped in a button). -->
	<div
		bind:this={triggerEl}
		role="presentation"
		class="inline-flex max-w-full flex-shrink-0 items-center gap-1 whitespace-nowrap rounded-full"
		onmouseenter={handleMouseEnter}
		onmouseleave={handleMouseLeave}
	>
		<Tag {icon} {color} {disabled} {label} {badge} pill={true} />
	</div>
{:else}
	<div
		bind:this={triggerEl}
		class="inline-flex max-w-full flex-shrink-0 cursor-pointer items-center gap-1 whitespace-nowrap rounded-full brightness-100 transition-all hover:brightness-90 dark:hover:brightness-125"
		onmouseenter={handleMouseEnter}
		onmouseleave={handleMouseLeave}
		onclick={handleClick}
		onkeydown={(e) => {
			if (e.key === 'Enter' || e.key === ' ') handleClick(e as unknown as MouseEvent);
		}}
		role="button"
		tabindex="0"
	>
		<Tag {icon} {color} {disabled} {label} {badge} pill={true} />
	</div>
{/if}

{#if displayComponent && !isStaticTags}
	<Popover
		triggerElement={triggerEl}
		isOpen={isHovered}
		onClose={() => {
			popoverHovered = false;
			isHovered = false;
		}}
		onMouseEnter={handlePopoverEnter}
		onMouseLeave={handlePopoverLeave}
		maxWidth="calc(100vw - 16px)"
	>
		<!-- Shown only while hovered, so it widens to show every tag rather than hiding any. -->
		<ListSelectItem
			item={entityRef.data}
			context={entityRef.context ?? {}}
			{displayComponent}
			showAllTags
		/>
	</Popover>
{/if}
