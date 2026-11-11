<script lang="ts">
	import { matchReasonLabel, type MatchDetails, type MatchReason } from '#lib/shared/types.ts';
	import { matchConfidences } from '#lib/shared/stores/metadata.ts';
	import { ChevronDown, ChevronRight } from '@lucide/svelte';
	import Tag from '#lib/shared/components/data/Tag.svelte';
	import InfoCard from '#lib/shared/components/data/InfoCard.svelte';
	import { hosts_services_matchDetails } from '#lib/paraglide/messages.js';

	let { details }: { details: MatchDetails } = $props();

	let isExpanded = $state(false);
	let rootChildren = $derived(details.reason.type === 'container' ? details.reason.data[1] : []);
</script>

{#snippet matchReasonNode(reason: MatchReason)}
	{#if reason.type === 'reason'}
		<div class="flex items-start gap-2 py-1 text-sm text-secondary">
			<span class="text-tertiary select-none">•</span>
			<span>{reason.data}</span>
		</div>
	{:else}
		<div class="py-1">
			<span class="text-sm font-medium text-secondary">{reason.data[0]}</span>
			{#if reason.data[1].length > 0}
				<div class="mt-1 ml-2 border-l-2 pl-4" style="border-color: var(--color-border)">
					{#each reason.data[1] as child, index (index)}
						{@render matchReasonNode(child)}
					{/each}
				</div>
			{/if}
		</div>
	{/if}
{/snippet}

<InfoCard title={hosts_services_matchDetails()}>
	<div class="flex items-center justify-between gap-2">
		{#if rootChildren.length > 0}
			<button
				type="button"
				class="flex items-center gap-1 text-left text-sm text-secondary hover:text-primary"
				aria-expanded={isExpanded}
				onclick={() => (isExpanded = !isExpanded)}
			>
				{#if isExpanded}
					<ChevronDown class="h-4 w-4 shrink-0" />
				{:else}
					<ChevronRight class="h-4 w-4 shrink-0" />
				{/if}
				{matchReasonLabel(details.reason)}
			</button>
		{:else}
			<span class="text-sm text-secondary">{matchReasonLabel(details.reason)}</span>
		{/if}
		<Tag {...matchConfidences.getTag(details.confidence)} />
	</div>
	{#if isExpanded && rootChildren.length > 0}
		<div class="ml-2 border-l-2 pl-4" style="border-color: var(--color-border)">
			{#each rootChildren as child, index (index)}
				{@render matchReasonNode(child)}
			{/each}
		</div>
	{/if}
</InfoCard>
