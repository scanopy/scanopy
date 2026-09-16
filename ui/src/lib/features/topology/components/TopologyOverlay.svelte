<script lang="ts">
	/**
	 * A modal that takes over the topology view rather than the whole window.
	 *
	 * Four things use one: the L2 setup prompt, the filters-emptied-this-view state, the dependency
	 * tutorial and the application wizard. Each is a shroud over the viewer plus a non-dismissable
	 * modal anchored inside `#topology-view-area`, which is the same three parts every time — the
	 * shroud, `GenericModal`'s `anchor="container"`, and the prop quartet that stops the user
	 * closing a state they have to act on.
	 *
	 * `offsetForPanel` insets the modal past the options panel so the panel stays visible beside it.
	 * That matters for the empty states in particular: the panel holds the filters that emptied the
	 * view, so covering it leaves the user with no way out. The panel additionally raises itself
	 * above the shroud — see `TopologyOptionsPanel`'s `raised` prop, which the caller sets.
	 */
	import type { Snippet } from 'svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import { OPTIONS_PANEL_FITVIEW_PADDING_PX } from '../queries';

	let {
		title,
		size = 'sm',
		fixedHeight = false,
		offsetForPanel = false,
		children,
		footer
	}: {
		title: string;
		size?: 'sm' | 'md' | 'lg' | 'xl' | 'full' | 'max';
		fixedHeight?: boolean;
		/** Inset the modal to the right of the options panel so the panel stays reachable. */
		offsetForPanel?: boolean;
		children: Snippet;
		footer?: Snippet;
	} = $props();

	let leftOffset = $derived(offsetForPanel ? OPTIONS_PANEL_FITVIEW_PADDING_PX : 0);
</script>

<!-- Shroud over the topology viewer -->
<div class="absolute inset-0 z-20 bg-black/60 backdrop-blur-sm"></div>

<div class="topology-overlay-anchor" style="--overlay-left-offset: {leftOffset}px;">
	<GenericModal
		{title}
		{size}
		{fixedHeight}
		isOpen={true}
		anchor="container"
		{footer}
		showCloseButton={false}
		preventCloseOnClickOutside={true}
		showBackdrop={false}
	>
		{@render children()}
	</GenericModal>
</div>

<style>
	.topology-overlay-anchor :global(.modal-page) {
		left: var(--overlay-left-offset);
	}
</style>
