<script lang="ts">
	import { matchReasonLabel, type MatchDetails } from '$lib/shared/types';
	import { matchConfidences } from '$lib/shared/stores/metadata';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { hosts_services_matchDetails, services_matchConfidence } from '$lib/paraglide/messages';

	let { details }: { details: MatchDetails } = $props();
</script>

<div class="flex items-center justify-between">
	<div class="text-secondary flex items-center gap-2">
		<h3 class="text-sm font-semibold">{hosts_services_matchDetails()}</h3>
		<span class="text-tertiary text-xs">({matchReasonLabel(details.reason)})</span>
	</div>
	<Tag {...matchConfidences.getTag(details.confidence, services_matchConfidence())} />
</div>
