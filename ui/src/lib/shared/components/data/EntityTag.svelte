<script lang="ts">
	import Tag from './Tag.svelte';
	import Popover from './Popover.svelte';
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { EntityRef } from './types';
	import type { Color } from '$lib/shared/utils/styling';
	import type { IconComponent } from '$lib/shared/utils/types';
	import { entityUIConfig } from '$lib/shared/entity-ui-config';
	import { entityRefTooltip, hasEntityTooltip } from '$lib/shared/entity-tooltip';
	import { dismissTooltip } from '$lib/shared/actions/tooltip';
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
	// The entity's own chip icon unless the caller passes one, so every chip for a host draws its
	// device type the same way.
	let chipIcon = $derived(icon ?? displayComponent?.getChipIcon?.(entityRef.data) ?? null);
	// An entity topology draws shows topology's tooltip, built on hover. The rest, which never
	// appear on the canvas, show their display component in a popover that exists only while open,
	// so a table of chips adds no portals.
	let sharesTooltip = $derived(hasEntityTooltip(entityRef.entityType));
	let chipTooltip = $derived(
		sharesTooltip && !disablePopover ? () => entityRefTooltip(entityRef) : null
	);

	function handleMouseEnter() {
		if (!displayComponent || disablePopover || sharesTooltip) return;
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
		dismissTooltip();
		navigateToEntity(entityRef.entityType, entityRef.entityId, entityRef.data);
	}
</script>

{#if isStaticTags}
	<span class="inline-flex max-w-full shrink-0 items-center gap-1 rounded-full whitespace-nowrap">
		<Tag icon={chipIcon} {color} {disabled} {label} {badge} pill={true} />
	</span>
{:else if disableNavigate}
	<!-- Non-interactive (no click-to-navigate, no hover-brightness) but still tracks
	     mouseenter/leave so the popover can still trigger when !disablePopover.
	     Cursor is inherited from the parent (e.g. pointer when wrapped in a button). -->
	<div
		bind:this={triggerEl}
		role="presentation"
		class="inline-flex max-w-full shrink-0 items-center gap-1 rounded-full whitespace-nowrap"
		onmouseenter={handleMouseEnter}
		onmouseleave={handleMouseLeave}
	>
		<Tag icon={chipIcon} {color} {disabled} {label} {badge} pill={true} title={chipTooltip} />
	</div>
{:else}
	<div
		bind:this={triggerEl}
		class="inline-flex max-w-full shrink-0 cursor-pointer items-center gap-1 rounded-full whitespace-nowrap brightness-100 transition-all hover:brightness-90 dark:hover:brightness-125"
		onmouseenter={handleMouseEnter}
		onmouseleave={handleMouseLeave}
		onclick={handleClick}
		onkeydown={(e) => {
			if (e.key === 'Enter' || e.key === ' ') handleClick(e as unknown as MouseEvent);
		}}
		role="button"
		tabindex="0"
	>
		<Tag icon={chipIcon} {color} {disabled} {label} {badge} pill={true} title={chipTooltip} />
	</div>
{/if}

{#if displayComponent && isHovered && !isStaticTags}
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
