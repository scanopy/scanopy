<!--
	The keyboard hint under a search box. One component so the Cmd+K palette and the topology
	search describe their keys identically; both take ↑↓ to move, Enter to open, Esc to close.
-->
<script lang="ts">
	import { search_navigateHint } from '$lib/paraglide/messages';
</script>

<p class="text-tertiary border-t px-4 py-2 text-xs" style="border-color: var(--color-border)">
	{search_navigateHint()}
</p>
