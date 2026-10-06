<!--
	A page's title row, the same on every page. It is the height of the sidebar's logo row and sits
	8px above what follows, as the logo row does the sidebar search, so a page's toolbar lines up
	with global search. A subtitle is a hover note on an (i), and `aside` is muted context after the
	title, set at the title's size and weight (Home's organization name); neither adds a second line.
-->
<script lang="ts">
	import { Info } from 'lucide-svelte';
	import { tooltip } from '$lib/shared/actions/tooltip';

	let {
		title,
		subtitle = null,
		aside = null
	}: { title: string; subtitle?: string | null; aside?: string | null } = $props();
</script>

<div class="mb-2 flex h-[38px] min-w-0 items-center gap-2">
	<h1 class="text-primary shrink-0 text-xl font-bold">{title}</h1>
	{#if subtitle}
		<span
			class="text-tertiary hover:text-secondary inline-flex cursor-help transition-colors"
			use:tooltip
			data-tooltip={subtitle}
			role="note"
			aria-label={subtitle}
		>
			<Info class="h-4 w-4" aria-hidden="true" />
		</span>
	{/if}
	{#if aside}
		<span aria-hidden="true" class="text-tertiary text-xl font-bold">·</span>
		<span class="text-tertiary truncate text-xl font-bold">{aside}</span>
	{/if}
</div>
