<script lang="ts">
	import { ArrowUpCircle } from 'lucide-svelte';
	import {
		triggerUpgrade,
		type PaywallSurface,
		type PaywallGateType
	} from '$lib/features/billing/trigger-upgrade';
	import type { UpgradeFeature } from '$lib/shared/stores/metadata';
	import { common_upgrade, common_upgradePlan } from '$lib/paraglide/messages';
	import { isEmbed } from '$lib/shared/utils/embed';

	let {
		feature,
		surface,
		gate_type = 'plan_required'
	}: {
		feature: UpgradeFeature;
		surface: PaywallSurface;
		gate_type?: PaywallGateType;
	} = $props();
</script>

{#if !isEmbed}
	<button
		title={common_upgradePlan()}
		class="btn-primary inline-flex items-center gap-1.5 border-amber-400 bg-amber-500 hover:border-amber-300 hover:bg-amber-600"
		onclick={() => triggerUpgrade({ feature, source: 'upgrade_button', surface, gate_type })}
	>
		<ArrowUpCircle class="h-4 w-4" />
		<span>{common_upgrade()}</span>
	</button>
{/if}
