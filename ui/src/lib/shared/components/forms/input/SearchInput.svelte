<!--
	The search box shared by the Cmd+K palette and the topology's Cmd+F search, so both read the
	same: a search icon, the input, optional trailing controls, and a line under it naming what the
	search covers.

	Bound to a TanStack field. The caller keeps its own `$state` copy of the query for reactive
	reads, since `field.state.value` is not tracked by `$derived`; `onInput` is where it updates it.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import { Search } from 'lucide-svelte';

	let {
		field,
		value,
		id,
		placeholder,
		scope,
		inputEl = $bindable(),
		onInput,
		onkeydown,
		trailing
	}: {
		field: AnyFieldApi;
		/** The caller's reactive copy of the query, which the input renders. */
		value: string;
		id: string;
		placeholder: string;
		/** What this search covers, shown under the input. */
		scope: string;
		inputEl?: HTMLInputElement;
		onInput?: (value: string) => void;
		onkeydown?: (event: KeyboardEvent) => void;
		trailing?: Snippet;
	} = $props();

	function handleInput(event: Event) {
		const value = (event.currentTarget as HTMLInputElement).value;
		field.handleChange(value);
		onInput?.(value);
	}
</script>

<div class="space-y-1 px-4 py-3">
	<div class="flex items-center gap-2">
		<Search class="text-tertiary h-4 w-4 flex-shrink-0" />
		<input
			bind:this={inputEl}
			{id}
			{value}
			oninput={handleInput}
			{onkeydown}
			type="text"
			autocomplete="off"
			{placeholder}
			aria-label={placeholder}
			aria-describedby="{id}-scope"
			class="text-primary h-7 w-full border-none bg-transparent text-sm focus:outline-none"
		/>
		{#if trailing}
			{@render trailing()}
		{/if}
	</div>
	<p id="{id}-scope" class="text-tertiary pl-6 text-xs">{scope}</p>
</div>
