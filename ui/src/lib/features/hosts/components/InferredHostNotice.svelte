<!--
	Why an inferred host is empty, and what fills it in.

	Keyed on `source.type`, the fact the backend stamped at mint time. A host with no ports, no
	services and no answering address looks the same whether a neighbour advertised it or it is
	simply down, so nothing here reads those. The copy is the `entity-sources` fixture entry, the
	same text as the Source chip's tooltip on the Hosts tab.
-->
<script lang="ts">
	import type { components } from '$lib/api/schema';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import { entitySources } from '$lib/shared/stores/metadata';

	let {
		source,
		class: className = ''
	}: {
		source: components['schemas']['EntitySource'];
		class?: string;
	} = $props();
</script>

{#if source.type === 'Inferred'}
	<div class={className}>
		<InlineInfo
			title={entitySources.getName(source.type)}
			body={entitySources.getDescription(source.type)}
		/>
	</div>
{/if}
