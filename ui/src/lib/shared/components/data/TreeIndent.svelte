<script lang="ts">
	/**
	 * The indent and connector before a row nested in a tree grouping. Nothing at depth 0.
	 *
	 * Decorative: the nesting is also stated in words where it matters (a subnet's "Within" item),
	 * so screen readers skip it.
	 */
	let { depth }: { depth: number } = $props();
</script>

{#if depth > 0}
	<span
		class="text-tertiary shrink-0 select-none"
		style="padding-left: {(depth - 1) * 16}px"
		aria-hidden="true">└</span
	>
{/if}
