<script lang="ts">
	import { Search } from '@lucide/svelte';
	import type { Snippet } from 'svelte';
	import type { IconComponent as IconComponentType } from '#lib/shared/utils/types.ts';
	import KbdKey from '#lib/shared/components/feedback/KbdKey.svelte';
	import PaidCrown from '#lib/shared/components/PaidCrown.svelte';
	import { isOnVisiblePage } from '#lib/shared/stores/modal-registry.ts';
	import { isFreeKeydown, isPlainKey } from '#lib/shared/components/data/controls/listShortcuts.ts';
	import { triggerUpgrade, type UpgradeGate } from '#lib/features/billing/trigger-upgrade.ts';
	import { common_availableOnPaidPlans } from '#lib/paraglide/messages.js';

	let {
		IconComponent = Search,
		title = '',
		subtitle = '',
		onClick = () => {},
		cta = '',
		upgrade = null,
		children
	}: {
		IconComponent?: IconComponentType;
		title?: string;
		subtitle?: string;
		onClick?: () => void;
		cta?: string;
		/** The plan lacks what the CTA does: it reads the same with a crown and opens the plans. */
		upgrade?: UpgradeGate | null;
		children?: Snippet;
	} = $props();

	// C presses the CTA, as it presses Create on a list: an empty page's CTA is its first item.
	const SHORTCUT = 'C';
	let root: HTMLDivElement | undefined = $state();
	let hasCta = $derived(!children && cta.length > 0);

	function press() {
		if (upgrade) triggerUpgrade({ ...upgrade, source: 'empty_state' });
		else onClick();
	}

	function handleKeydown(event: KeyboardEvent) {
		if (!hasCta || !isPlainKey(event, SHORTCUT) || !isFreeKeydown(event)) return;
		if (!isOnVisiblePage(root)) return;
		event.preventDefault();
		press();
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div bind:this={root} class="py-12 text-center">
	<IconComponent size={16} class="mx-auto mb-4 h-16 w-16 text-secondary" />
	<p class="mb-4 text-lg text-secondary">{title}</p>
	<p class="mb-6 text-tertiary">{subtitle}</p>
	{#if children}
		{@render children()}
	{:else if hasCta}
		<button
			onclick={press}
			class="btn-primary gap-2"
			title={upgrade ? common_availableOnPaidPlans() : undefined}
		>
			{#if upgrade}
				<PaidCrown onPrimary />
			{/if}
			{cta}
			<KbdKey key={SHORTCUT} size="xs" tone="inherit" />
		</button>
	{/if}
</div>
