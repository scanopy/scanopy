<!--
	The badge on a sidebar item, and on a heading that repeats one: a dot, or a pill with `count`.
	Both are one height, so a badge with a count and one without match. `class` places it, such as
	on the corner of an icon.
-->
<script lang="ts">
	let {
		color,
		count,
		class: className = ''
	}: {
		/** A CSS color, such as a color helper's `rgb`. */
		color: string;
		count?: number;
		class?: string;
	} = $props();
</script>

{#if count === undefined}
	<span class="inline-block h-3.5 w-3.5 rounded-full {className}" style="background-color: {color}"
	></span>
{:else}
	<span
		class="inline-block h-3.5 min-w-3.5 rounded-full px-1 text-center text-[9px] leading-3.5 font-semibold text-white {className}"
		style="background-color: {color}"
	>
		{count}
	</span>
{/if}
