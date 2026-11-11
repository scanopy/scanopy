<script lang="ts">
	import { showViewSwitcherHint } from '../queries';
	import { topology_viewSwitcherHint } from '#lib/paraglide/messages.js';
	import { onMount } from 'svelte';

	let { anchor }: { anchor: HTMLElement } = $props();

	let portalContainer: HTMLDivElement | null = $state(null);
	let top = $state(0);
	let left = $state(0);
	let ready = $state(false);

	onMount(() => {
		portalContainer = document.createElement('div');
		portalContainer.style.position = 'absolute';
		portalContainer.style.top = '0';
		portalContainer.style.left = '0';
		portalContainer.style.width = '0';
		portalContainer.style.height = '0';
		document.body.appendChild(portalContainer);

		setTimeout(() => {
			if (!anchor) return;
			const rect = anchor.getBoundingClientRect();
			const hintWidth = 256;
			top = rect.bottom + 8;
			left = rect.left + rect.width / 2 - hintWidth / 2;
			ready = true;
		}, 100);

		function handleClick() {
			dismiss();
		}
		document.addEventListener('click', handleClick, { once: true, capture: true });

		return () => {
			portalContainer?.remove();
			document.removeEventListener('click', handleClick, { capture: true });
		};
	});

	function portal(node: HTMLElement) {
		portalContainer?.appendChild(node);
		return {
			destroy() {
				node.remove();
			}
		};
	}

	function dismiss() {
		showViewSwitcherHint.set(false);
	}
</script>

{#if ready && portalContainer}
	<div use:portal class="fixed z-9999 w-64" style="top: {top}px; left: {left}px;">
		<div class="hint-callout relative rounded-lg px-4 py-3">
			<div class="hint-arrow absolute -top-2 left-1/2 -translate-x-1/2"></div>
			<p class="text-xs text-primary">
				{topology_viewSwitcherHint()}
			</p>
		</div>
	</div>
{/if}

<style>
	.hint-callout {
		background: var(--color-bg-elevated, #1e2030);
		border: 1.5px solid rgb(244, 63, 94);
		box-shadow:
			0 0 16px rgba(244, 63, 94, 0.25),
			0 4px 12px rgba(0, 0, 0, 0.4);
	}

	.hint-arrow {
		width: 0;
		height: 0;
		border-left: 8px solid transparent;
		border-right: 8px solid transparent;
		border-bottom: 8px solid rgb(244, 63, 94);
	}
</style>
