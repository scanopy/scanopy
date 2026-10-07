<!--
	The keyboard hint for a search box. One component so the global search palette and the topology
	search describe their keys identically; both take ↑↓ to move, Enter to open, Esc to close. While
	the box shows a tag completion, it also names Tab.
-->
<script lang="ts">
	import { globalSearch_tabHint, search_navigateHint } from '$lib/paraglide/messages';

	let {
		tabCompletes = false,
		inline = false
	}: {
		/** A completion is showing, so Tab accepts it. */
		tabCompletes?: boolean;
		/** Text only, to sit in a line beside other text; otherwise a bordered footer. */
		inline?: boolean;
	} = $props();
</script>

{#snippet hint()}
	{search_navigateHint()}{#if tabCompletes}{' · '}{globalSearch_tabHint()}{/if}
{/snippet}

{#if inline}
	<span class="text-tertiary text-xs">{@render hint()}</span>
{:else}
	<p class="text-tertiary border-t px-4 py-2 text-xs" style="border-color: var(--color-border)">
		{@render hint()}
	</p>
{/if}
