<script lang="ts" generics="T">
	import { ChevronDown, ChevronUp, Columns3Cog, GripVertical } from 'lucide-svelte';
	import { tick } from 'svelte';
	import {
		isMovableColumn,
		movableIds,
		moveColumn,
		moveColumnTo,
		type EntityColumn
	} from './columns';
	import {
		common_dragToReorderColumn,
		common_fields,
		common_moveColumnDown,
		common_moveColumnUp,
		common_resetFields
	} from '$lib/paraglide/messages';

	let {
		columns,
		order,
		visibility,
		onToggle,
		onReorder,
		onReset
	}: {
		columns: EntityColumn<T>[];
		/** The full column order, pinned columns included. */
		order: string[];
		visibility: Record<string, boolean>;
		onToggle: (id: string) => void;
		onReorder: (order: string[]) => void;
		onReset: () => void;
	} = $props();

	let open = $state(false);
	let menuElement = $state<HTMLDivElement | undefined>();

	/**
	 * The primary column stays put: it carries the row's identity and its
	 * checkbox, so hiding it would leave rows with nothing to identify them by.
	 * Listed in the user's order, so the menu reads the way the table does.
	 */
	let toggleable = $derived.by(() => {
		const byId = new Map(columns.map((c) => [c.id, c]));
		return order
			.map((id) => byId.get(id))
			.filter((c): c is EntityColumn<T> => c !== undefined && !c.primary);
	});

	let movable = $derived(movableIds(order, columns));

	let draggingId = $state<string | null>(null);
	let dropTarget = $state<{ id: string; position: 'before' | 'after' } | null>(null);

	/**
	 * Set while a move re-sorts the rows. Moving a focused row in the DOM blurs
	 * it with no `relatedTarget`, which would otherwise read as focus leaving the
	 * menu and close it under the keyboard user.
	 */
	let reordering = false;

	function handleFocusOut(event: FocusEvent) {
		if (reordering) return;
		const next = event.relatedTarget as Node | null;
		if (next && menuElement?.contains(next)) return;
		open = false;
	}

	async function step(id: string, delta: -1 | 1) {
		const next = moveColumn(order, columns, id, delta);
		if (next === order) return;

		reordering = true;
		onReorder(next);
		await tick();
		reordering = false;

		// Keep focus on the control that was pressed, or on its twin once the
		// column reaches an end and that control disables.
		const direction = delta < 0 ? 'up' : 'down';
		const opposite = delta < 0 ? 'down' : 'up';
		const pressed = menuElement?.querySelector<HTMLButtonElement>(
			`[data-move="${direction}"][data-column="${id}"]`
		);
		const fallback = menuElement?.querySelector<HTMLButtonElement>(
			`[data-move="${opposite}"][data-column="${id}"]`
		);
		(pressed && !pressed.disabled ? pressed : fallback)?.focus();
	}

	function handleDragStart(event: DragEvent, id: string) {
		draggingId = id;
		if (event.dataTransfer) {
			event.dataTransfer.effectAllowed = 'move';
			event.dataTransfer.setData('text/plain', id);
		}
	}

	function handleDragOver(event: DragEvent, id: string) {
		if (!draggingId || !movable.includes(id)) return;
		event.preventDefault();
		if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';

		const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
		const position = event.clientY < rect.top + rect.height / 2 ? 'before' : 'after';
		dropTarget = { id, position };
	}

	function handleDrop(event: DragEvent) {
		event.preventDefault();
		if (draggingId && dropTarget) {
			const next = moveColumnTo(order, columns, draggingId, dropTarget.id, dropTarget.position);
			if (next !== order) onReorder(next);
		}
		clearDrag();
	}

	function clearDrag() {
		draggingId = null;
		dropTarget = null;
	}
</script>

<div bind:this={menuElement} class="relative" onfocusout={handleFocusOut} role="presentation">
	<button
		type="button"
		onclick={() => (open = !open)}
		class="btn-secondary toolbar-control"
		aria-expanded={open}
		aria-haspopup="true"
		title={common_fields()}
	>
		<Columns3Cog class="h-5 w-5" />
	</button>

	{#if open}
		<!--
			Focusable but out of the tab order, so a mousedown on something that is not
			focusable — a drag grip, row padding — moves focus here instead of nowhere.
			Nowhere reads to `handleFocusOut` as leaving the menu, and closed it before
			a drag could start.
		-->
		<div
			tabindex="-1"
			class="card absolute right-0 z-30 mt-1 max-h-96 w-72 overflow-y-auto !rounded-lg !p-3 shadow-lg outline-none"
		>
			<div class="mb-2 flex items-center justify-between">
				<span class="text-primary text-sm font-semibold">{common_fields()}</span>
				<button
					type="button"
					onclick={onReset}
					class="text-tertiary hover:text-secondary text-xs transition-colors"
				>
					{common_resetFields()}
				</button>
			</div>
			<ul class="space-y-0.5">
				{#each toggleable as column (column.id)}
					{@const canMove = isMovableColumn(column)}
					{@const index = movable.indexOf(column.id)}
					{@const isTarget = dropTarget?.id === column.id && draggingId !== column.id}
					<li
						class="flex items-center gap-1 rounded border-y-2 border-transparent px-1 py-0.5 {draggingId ===
						column.id
							? 'opacity-50'
							: ''} {isTarget && dropTarget?.position === 'before'
							? '!border-t-blue-500'
							: ''} {isTarget && dropTarget?.position === 'after' ? '!border-b-blue-500' : ''}"
						draggable={canMove}
						ondragstart={(event) => handleDragStart(event, column.id)}
						ondragover={(event) => handleDragOver(event, column.id)}
						ondrop={handleDrop}
						ondragend={clearDrag}
					>
						{#if canMove}
							<span
								class="text-tertiary cursor-grab"
								title={common_dragToReorderColumn({ column: column.label })}
								aria-hidden="true"
							>
								<GripVertical class="h-4 w-4" />
							</span>
						{:else}
							<span class="w-4" aria-hidden="true"></span>
						{/if}
						<label class="flex min-w-0 flex-1 cursor-pointer items-center gap-2">
							<input
								type="checkbox"
								checked={visibility[column.id] !== false}
								onchange={() => onToggle(column.id)}
								class="checkbox-card h-4 w-4 rounded"
							/>
							<span class="text-secondary truncate text-sm" title={column.label}
								>{column.label}</span
							>
						</label>
						{#if canMove}
							<button
								type="button"
								data-move="up"
								data-column={column.id}
								onclick={() => step(column.id, -1)}
								disabled={index <= 0}
								aria-label={common_moveColumnUp({ column: column.label })}
								class="text-tertiary hover:text-primary rounded p-0.5 transition-colors disabled:opacity-30"
							>
								<ChevronUp class="h-4 w-4" />
							</button>
							<button
								type="button"
								data-move="down"
								data-column={column.id}
								onclick={() => step(column.id, 1)}
								disabled={index === movable.length - 1}
								aria-label={common_moveColumnDown({ column: column.label })}
								class="text-tertiary hover:text-primary rounded p-0.5 transition-colors disabled:opacity-30"
							>
								<ChevronDown class="h-4 w-4" />
							</button>
						{/if}
					</li>
				{/each}
			</ul>
		</div>
	{/if}
</div>
