<script lang="ts">
	/**
	 * L2 Physical has no neighbour data to draw. A setup prompt, not a filter state — a view
	 * emptied by the user's own filters gets `ViewFiltersEmptyState` instead, which names them.
	 */
	import TopologyOverlay from './TopologyOverlay.svelte';
	import { openModal } from '$lib/shared/stores/modal-registry';
	import { Cable } from 'lucide-svelte';
	import {
		topology_l2EmptyTitle,
		topology_l2EmptyDescription,
		topology_l2EmptySnmpHint,
		home_nudges_snmpAction
	} from '$lib/paraglide/messages';

	let {
		hasSnmpCredential = false
	}: {
		hasSnmpCredential?: boolean;
	} = $props();
</script>

<TopologyOverlay title={topology_l2EmptyTitle()} size="sm" offsetForPanel>
	<div class="flex flex-col items-center gap-4 p-6 text-center">
		<div class="rounded-full bg-emerald-500/10 p-3">
			<Cable class="h-8 w-8 text-emerald-500" />
		</div>
		<p class="text-secondary text-sm">
			{topology_l2EmptyDescription()}
		</p>
		{#if !hasSnmpCredential}
			<div class="border-primary/10 w-full border-t pt-3">
				<p class="text-secondary/70 mb-2 text-xs">
					{topology_l2EmptySnmpHint()}
				</p>
				<button class="btn btn-sm btn-primary" onclick={() => openModal('credential-editor')}>
					{home_nudges_snmpAction()}
				</button>
			</div>
		{/if}
	</div>
</TopologyOverlay>
