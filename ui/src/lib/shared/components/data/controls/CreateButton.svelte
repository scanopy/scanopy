<!--
	A list page's main action, last in its toolbar: Create, or the page's own word for it (Invite).
	C presses it while its list is on screen; the key shows in the button and dims while the table
	search has focus. Outside a list (no `DataControls` around it) it has no shortcut.

	When the plan lacks what it creates (`upgrade`), it reads the same with a crown beside the label,
	and pressing it opens the plans instead (`triggerUpgrade`).
-->
<script lang="ts">
	import KbdKey from '#lib/shared/components/feedback/KbdKey.svelte';
	import PaidCrown from '#lib/shared/components/PaidCrown.svelte';
	import { triggerUpgrade, type UpgradeGate } from '#lib/features/billing/trigger-upgrade.ts';
	import {
		common_availableOnPaidPlans,
		common_create,
		common_shortcutsPausedInSearch
	} from '#lib/paraglide/messages.js';
	import { isFreeKeydown, isPlainKey, useListShortcuts } from './listShortcuts';

	let {
		onclick,
		label = common_create(),
		upgrade = null
	}: {
		onclick: () => void;
		label?: string;
		/** The plan lacks what this creates: the gate the click opens the plans for. */
		upgrade?: UpgradeGate | null;
	} = $props();

	const SHORTCUT = 'C';
	const shortcuts = useListShortcuts();
	let paused = $derived(shortcuts?.isPaused() ?? false);

	function press() {
		if (upgrade) triggerUpgrade({ ...upgrade, source: 'create_button' });
		else onclick();
	}

	function handleKeydown(event: KeyboardEvent) {
		if (!shortcuts?.isActive() || !isPlainKey(event, SHORTCUT) || !isFreeKeydown(event)) return;
		event.preventDefault();
		press();
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<button
	class="btn-primary toolbar-control flex items-center gap-2"
	title={upgrade ? common_availableOnPaidPlans() : undefined}
	onclick={press}
>
	{#if upgrade}
		<PaidCrown onPrimary />
	{/if}
	{label}
	{#if shortcuts}
		<span
			class="inline-flex transition-opacity max-md:hidden {paused ? 'opacity-40' : ''}"
			title={paused ? common_shortcutsPausedInSearch() : undefined}
		>
			<KbdKey key={SHORTCUT} size="xs" tone="inherit" />
		</span>
	{/if}
</button>
