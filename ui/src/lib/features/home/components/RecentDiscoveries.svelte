<script lang="ts">
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { toColor } from '$lib/shared/utils/styling';
	import { entities } from '$lib/shared/stores/metadata';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import type { components } from '$lib/api/schema';

	type Discovery = components['schemas']['Discovery'];

	let { discoveries }: { discoveries: Discovery[] } = $props();

	const DiscoveryIcon = entities.getIconComponent('Discovery');

	function getPhaseFromRunType(discovery: Discovery) {
		if (discovery.run_type.type === 'Historical' && discovery.run_type.results) {
			return discovery.run_type.results.phase ?? null;
		}
		return null;
	}

	function getPhaseTag(phase: string | null) {
		if (!phase) return null;
		switch (phase) {
			case 'Complete':
				return { label: 'Complete', color: toColor('green') };
			case 'Failed':
				return { label: 'Failed', color: toColor('red') };
			case 'Cancelled':
				return { label: 'Cancelled', color: toColor('yellow') };
			default:
				return { label: phase, color: toColor('blue') };
		}
	}
</script>

<section>
	<h3 class="text-primary mb-3 text-base font-semibold">Recent Discoveries</h3>
	{#if discoveries.length === 0}
		<p class="text-tertiary text-sm">No discovery results yet.</p>
	{:else}
		<div class="space-y-2">
			{#each discoveries as discovery (discovery.id)}
				{@const phase = getPhaseFromRunType(discovery)}
				{@const phaseTag = getPhaseTag(phase)}
				<div
					class="flex items-center justify-between rounded-lg border border-gray-700 bg-gray-800/50 px-4 py-3"
				>
					<div class="flex items-center gap-3">
						<DiscoveryIcon
							class="h-4 w-4 flex-shrink-0"
							style="color: {entities.getColorHelper('Discovery').icon}"
						/>
						<span class="text-primary text-sm font-medium">{discovery.name}</span>
					</div>
					<div class="flex items-center gap-3">
						<span class="text-tertiary text-xs">{formatTimestamp(discovery.created_at)}</span>
						{#if phaseTag}
							<Tag label={phaseTag.label} color={phaseTag.color} />
						{/if}
					</div>
				</div>
			{/each}
		</div>
	{/if}
</section>
