<script lang="ts">
	import Tag from './Tag.svelte';
	import { attributeSourceTag, type AttributeSource } from '#lib/shared/utils/attribute-source.ts';
	import { common_noSourceRecorded } from '#lib/paraglide/messages.js';

	/**
	 * Where one value came from. Pass `null` for a field that records no provenance (a host's
	 * hostname, an address): the tag then says so rather than inventing a source.
	 */
	let { source }: { source: AttributeSource | null | undefined } = $props();

	let tag = $derived(source ? attributeSourceTag(source) : null);
</script>

{#if tag}
	<Tag label={tag.label} color={tag.color} title={tag.title} />
{:else}
	<span class="text-xs text-secondary italic">{common_noSourceRecorded()}</span>
{/if}
