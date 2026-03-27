<script lang="ts">
	import { Pencil } from 'lucide-svelte';

	function focus(node: HTMLElement) {
		node.focus();
	}

	let {
		value = null,
		editable = false,
		maxLength = 500,
		onSave
	}: {
		value: string | null;
		editable: boolean;
		maxLength?: number;
		onSave: (value: string | null) => void;
	} = $props();

	let editing = $state(false);
	let draft = $state('');

	function startEdit() {
		if (!editable) return;
		draft = value ?? '';
		editing = true;
	}

	function save() {
		editing = false;
		const trimmed = draft.trim();
		const newValue = trimmed || null;
		if (newValue !== (value ?? null)) {
			onSave(newValue);
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			save();
		}
	}
</script>

{#if editing}
	<textarea
		class="text-secondary w-full resize-none rounded border border-gray-600 bg-transparent px-2 py-1 text-xs focus:outline-none focus:ring-1 focus:ring-blue-500"
		bind:value={draft}
		maxlength={maxLength}
		rows={2}
		onblur={save}
		onkeydown={handleKeydown}
		use:focus
	></textarea>
{:else if editable && !value}
	<button
		class="text-tertiary hover:text-secondary flex items-center gap-1 text-xs italic"
		onclick={startEdit}
	>
		<Pencil class="h-3 w-3" />
		<span>Edit description...</span>
	</button>
{:else if value}
	{#if editable}
		<button
			class="text-secondary flex w-full cursor-text items-center gap-1 text-left text-xs hover:opacity-80"
			onclick={startEdit}
		>
			<span class="flex-1 text-left">{value}</span>
			<Pencil class="h-3 w-3 shrink-0 opacity-50" />
		</button>
	{:else}
		<p class="text-secondary text-xs">{value}</p>
	{/if}
{/if}
