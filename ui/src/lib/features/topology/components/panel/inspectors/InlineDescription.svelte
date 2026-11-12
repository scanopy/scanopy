<script lang="ts">
	import { Pencil } from '@lucide/svelte';
	import { inspector_editDescription } from '#lib/paraglide/messages.js';

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
		class="w-full resize-none rounded-sm border border-gray-600 bg-transparent px-2 py-1 text-xs text-secondary focus:ring-1 focus:ring-blue-500 focus:outline-hidden"
		bind:value={draft}
		maxlength={maxLength}
		rows={2}
		onblur={save}
		onkeydown={handleKeydown}
		use:focus
	></textarea>
{:else if editable && !value}
	<button
		class="flex items-center gap-1 text-xs text-tertiary italic hover:text-secondary"
		onclick={startEdit}
	>
		<Pencil class="h-3 w-3" />
		<span>{inspector_editDescription()}</span>
	</button>
{:else if value}
	{#if editable}
		<button
			class="flex w-full cursor-text items-center gap-1 text-left text-xs text-secondary hover:opacity-80"
			onclick={startEdit}
		>
			<span class="flex-1 text-left">{value}</span>
			<Pencil class="h-3 w-3 shrink-0 opacity-50" />
		</button>
	{:else}
		<p class="text-xs text-secondary">{value}</p>
	{/if}
{/if}
