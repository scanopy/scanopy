<script lang="ts">
	import Tag from './Tag.svelte';
	import type { EntityRef } from './types';
	import type { Color } from '$lib/shared/utils/styling';
	import type { IconComponent } from '$lib/shared/utils/types';
	import { entityUIConfig } from '$lib/shared/entity-ui-config';
	import { entityRefTooltip } from '$lib/shared/entity-tooltip';
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
		/** When true, hovering the tag shows no tooltip. */
		disablePopover?: boolean;
		/**
		 * When true, the tag doesn't navigate on click and drops the interactive
		 * cursor / hover-brightness treatment. The tooltip is unaffected — use
		 * `disablePopover` for that.
		 */
		disableNavigate?: boolean;
	} = $props();

	let displayComponent = $derived(entityUIConfig[entityRef.entityType]?.displayComponent ?? null);
	// The entity's own chip icon unless the caller passes one, so every chip for a host draws its
	// device type the same way.
	let chipIcon = $derived(icon ?? displayComponent?.getChipIcon?.(entityRef.data) ?? null);
	// Built on hover, in the app's one tooltip, so a table of chips costs nothing until then.
	let chipTooltip = $derived(disablePopover ? null : () => entityRefTooltip(entityRef, label));

	function handleClick(e: MouseEvent) {
		e.stopPropagation();
		e.preventDefault();
		if (disabled || isStaticTags) return;
		dismissTooltip();
		navigateToEntity(entityRef.entityType, entityRef.entityId, entityRef.data);
	}
</script>

{#if isStaticTags}
	<span class="inline-flex max-w-full shrink-0 items-center gap-1 rounded-full whitespace-nowrap">
		<Tag icon={chipIcon} {color} {disabled} {label} {badge} pill={true} />
	</span>
{:else if disableNavigate}
	<!-- Non-interactive (no click-to-navigate, no hover-brightness), and the tooltip still shows.
	     Cursor is inherited from the parent (e.g. pointer when wrapped in a button). -->
	<div
		role="presentation"
		class="inline-flex max-w-full shrink-0 items-center gap-1 rounded-full whitespace-nowrap"
	>
		<Tag icon={chipIcon} {color} {disabled} {label} {badge} pill={true} title={chipTooltip} />
	</div>
{:else}
	<div
		class="inline-flex max-w-full shrink-0 cursor-pointer items-center gap-1 rounded-full whitespace-nowrap brightness-100 transition-all hover:brightness-90 dark:hover:brightness-125"
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
