<!--
	The keyboard hint under a search box. One component so the global search palette and the topology
	search describe their keys identically; both take ↑↓ to move, Enter to open, Esc to close. While
	the box shows a tag completion, it also names Tab.
-->
<script lang="ts">
	import { globalSearch_tabHint, search_navigateHint } from '$lib/paraglide/messages';

	let {
		tabCompletes = false
	}: {
		/** A completion is showing, so Tab accepts it. */
		tabCompletes?: boolean;
	} = $props();
</script>

<p class="text-tertiary border-t px-4 py-2 text-xs" style="border-color: var(--color-border)">
	{search_navigateHint()}{#if tabCompletes}{' · '}{globalSearch_tabHint()}{/if}
</p>
