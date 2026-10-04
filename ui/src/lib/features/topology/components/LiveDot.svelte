<script lang="ts">
	/**
	 * A filled dot sized to sit in an icon slot, coloured by `currentColor`.
	 *
	 * Icon slots (ListSelectItem) pass only a `class` of size and text colour, so
	 * a Lucide icon can't be filled from there, and Lucide's `Dot` draws a 2px
	 * circle at that size. This centres a half-size dot in the slot instead.
	 */
	let { class: className = '' }: { class?: string } = $props();
</script>

<span class="inline-flex items-center justify-center {className}" aria-hidden="true">
	<span class="h-2.5 w-2.5 rounded-full bg-current"></span>
</span>
