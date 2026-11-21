<script lang="ts" module>
	const SEPARATOR = '⁣';

	/** A stand-in for a slot inside a translated message: pass it as the message's parameter, and
	 *  `MessageWithSlots` renders `part(key)` where the translation placed it. */
	export function slot(key: string): string {
		return `${SEPARATOR}${key}${SEPARATOR}`;
	}
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		text,
		part
	}: {
		/** A message rendered with `slot(key)` for each parameter that becomes markup. */
		text: string;
		/** Renders the markup for one slot key, such as an entity chip. */
		part: Snippet<[string]>;
	} = $props();

	// Splitting on the separator puts the text between slots at even indices and slot keys at odd ones.
	let pieces = $derived(text.split(SEPARATOR));
</script>

{#each pieces as piece, i (i)}{#if i % 2 === 1}{@render part(piece)}{:else}{piece}{/if}{/each}
