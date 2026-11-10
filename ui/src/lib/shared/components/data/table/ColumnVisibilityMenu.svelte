<script lang="ts" generics="T">
	import { ChevronDown, ChevronUp, Columns3Cog, GripVertical } from '@lucide/svelte';
	import { tick } from 'svelte';
	import {
		columnSections,
		isMovableColumn,
		movableIds,
		moveColumn,
		moveColumnTo,
		movableGroups,
		moveGroup,
		moveGroupTo,
		type EntityColumn
	} from './columns';
	import {
		common_dragToReorderColumn,
		common_columnPinnedFirst,
		common_columnPinnedLast,
		common_columnHiddenWhileGrouped,
		common_columnSectionPinned,
		common_columns,
		common_moveColumnDown,
		common_moveColumnUp,
		common_resetColumns,
		common_shortcutsPausedInSearch
	} from '$lib/paraglide/messages';
	import { isShiftShortcut, shiftShortcutLabel } from '$lib/shared/utils/shortcuts';
	import { isFreeKeydown, useListShortcuts } from '../controls/listShortcuts';
	import Popover from '../Popover.svelte';
	import { tooltip } from '$lib/shared/actions/tooltip';

	let {
		columns,
		order,
		groupOrder,
		visibility,
		groupedColumnId = null,
		onToggle,
		onReorder,
		onReorderGroups,
		onReset
	}: {
		columns: EntityColumn<T>[];
		/** The full column order, pinned columns included. */
		order: string[];
		/** The headings in the user's order; columns sort by heading first. */
		groupOrder: string[];
		visibility: Record<string, boolean>;
		/** The column the list groups by, which the table leaves out until it is ungrouped. */
		groupedColumnId?: string | null;
		onToggle: (id: string) => void;
		onReorder: (order: string[]) => void;
		onReorderGroups: (groupOrder: string[]) => void;
		onReset: () => void;
	} = $props();

	let open = $state(false);
	let trigger = $state<HTMLButtonElement | undefined>();
	let panelElement = $state<HTMLDivElement | undefined>();

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

	let sections = $derived(columnSections(toggleable));

	/** The columns a column moves among: those of its own section. */
	function peersOf(id: string): EntityColumn<T>[] {
		return sections.find((s) => s.columns.some((c) => c.id === id))?.columns ?? [];
	}

	let draggingId = $state<string | null>(null);
	let dropTarget = $state<{ id: string; position: 'before' | 'after' } | null>(null);

	// Shift+C (columns) opens the panel while this list is on screen; Shift+F already clears filters.
	const COLUMNS_KEY = 'C';
	const columnsShortcut = shiftShortcutLabel(COLUMNS_KEY);
	const listShortcuts = useListShortcuts();

	// Escape, focus leaving the panel and a click outside close it through `Popover`, which also
	// moves focus into the panel when it opens.
	function handleKeydown(event: KeyboardEvent) {
		if (!listShortcuts?.isActive()) return;
		if (!isShiftShortcut(event, COLUMNS_KEY) || !isFreeKeydown(event)) return;
		event.preventDefault();
		open = !open;
	}

	async function step(id: string, delta: -1 | 1) {
		const next = moveColumn(order, peersOf(id), id, delta);
		if (next === order) return;

		onReorder(next);
		await tick();

		// Keep focus on the control that was pressed, or on its twin once the
		// column reaches an end and that control disables.
		const direction = delta < 0 ? 'up' : 'down';
		const opposite = delta < 0 ? 'down' : 'up';
		const pressed = panelElement?.querySelector<HTMLButtonElement>(
			`[data-move="${direction}"][data-column="${id}"]`
		);
		const fallback = panelElement?.querySelector<HTMLButtonElement>(
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
		if (!draggingId || !movableIds(order, peersOf(draggingId)).includes(id)) return;
		event.preventDefault();
		if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';

		const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
		const position = event.clientY < rect.top + rect.height / 2 ? 'before' : 'after';
		dropTarget = { id, position };
	}

	function handleDrop(event: DragEvent) {
		// A dragged heading drops on the block around this row instead (`handleSectionDrop`).
		if (!draggingId) return;
		event.preventDefault();
		if (draggingId && dropTarget) {
			const next = moveColumnTo(
				order,
				peersOf(draggingId),
				draggingId,
				dropTarget.id,
				dropTarget.position
			);
			if (next !== order) onReorder(next);
		}
		clearDrag();
	}

	function clearDrag() {
		draggingId = null;
		dropTarget = null;
		draggingSection = null;
		sectionDropTarget = null;
	}

	/** Headings whose columns move as a block, in menu order. */
	let movableSections = $derived(movableGroups(groupOrder, toggleable));

	let draggingSection = $state<string | null>(null);
	let sectionDropTarget = $state<{ key: string; position: 'before' | 'after' } | null>(null);

	async function stepSection(key: string, delta: -1 | 1) {
		const next = moveGroup(groupOrder, movableSections, key, delta);
		if (next === groupOrder) return;

		onReorderGroups(next);
		await tick();

		// Same focus handling as `step`, keyed by the heading.
		const direction = delta < 0 ? 'up' : 'down';
		const opposite = delta < 0 ? 'down' : 'up';
		const pressed = panelElement?.querySelector<HTMLButtonElement>(
			`[data-move="${direction}"][data-section="${key}"]`
		);
		const fallback = panelElement?.querySelector<HTMLButtonElement>(
			`[data-move="${opposite}"][data-section="${key}"]`
		);
		(pressed && !pressed.disabled ? pressed : fallback)?.focus();
	}

	function handleSectionDragStart(event: DragEvent, key: string) {
		draggingSection = key;
		if (event.dataTransfer) {
			event.dataTransfer.effectAllowed = 'move';
			event.dataTransfer.setData('text/plain', key);
		}
	}

	/** A heading drops before or after a whole heading's block, never inside it. */
	function handleSectionDragOver(event: DragEvent, key: string | null) {
		if (!draggingSection || key === null || !movableSections.includes(key)) return;
		event.preventDefault();
		if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';

		const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
		const position = event.clientY < rect.top + rect.height / 2 ? 'before' : 'after';
		sectionDropTarget = { key, position };
	}

	function handleSectionDrop(event: DragEvent) {
		if (!draggingSection) return;
		event.preventDefault();
		if (sectionDropTarget) {
			const next = moveGroupTo(
				groupOrder,
				draggingSection,
				sectionDropTarget.key,
				sectionDropTarget.position
			);
			if (next !== groupOrder) onReorderGroups(next);
		}
		clearDrag();
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<button
	type="button"
	bind:this={trigger}
	onclick={() => (open = !open)}
	class="summary-control btn-secondary"
	aria-expanded={open}
	aria-haspopup="dialog"
>
	<Columns3Cog class="h-3.5 w-3.5" />
	<span class="max-md:sr-only">{common_columns()}</span>
	{#if listShortcuts}
		{@const paused = listShortcuts.isPaused()}
		<!-- Same key hint as the other controls in the summary row (`PaginationBar`). -->
		<span
			class="font-normal text-tertiary transition-opacity max-md:hidden {paused
				? 'opacity-40'
				: ''}"
			title={paused ? common_shortcutsPausedInSearch() : undefined}>({columnsShortcut})</span
		>
	{/if}
</button>

<Popover
	triggerElement={trigger ?? null}
	isOpen={open}
	role="dialog"
	ariaLabel={common_columns()}
	align="start"
	onClose={() => (open = false)}
>
	<div bind:this={panelElement} class="w-[17rem]">
		<div class="mb-2 flex items-center justify-between">
			<span class="text-sm font-semibold text-primary">{common_columns()}</span>
			<button
				type="button"
				onclick={onReset}
				class="text-xs text-tertiary transition-colors hover:text-secondary"
			>
				{common_resetColumns()}
			</button>
		</div>
		{#each sections as { key, heading, columns: sectionColumns } (key)}
			{@const movable = movableIds(order, sectionColumns)}
			{@const sectionIndex = key === null ? -1 : movableSections.indexOf(key)}
			{@const isSectionTarget =
				key !== null && sectionDropTarget?.key === key && draggingSection !== key}
			<div
				class="rounded border-y-2 border-transparent {draggingSection !== null &&
				draggingSection === key
					? 'opacity-50'
					: ''} {isSectionTarget && sectionDropTarget?.position === 'before'
					? 'border-t-blue-500!'
					: ''} {isSectionTarget && sectionDropTarget?.position === 'after'
					? 'border-b-blue-500!'
					: ''}"
				role="presentation"
				ondragover={(event) => handleSectionDragOver(event, key)}
				ondrop={handleSectionDrop}
			>
				{#if key !== null && heading}
					{@const pinnedReason =
						sectionIndex === -1 ? common_columnSectionPinned({ section: heading }) : undefined}
					<!-- Flush left, with its columns indented beneath it. Dragging the heading, or its
							arrows, moves every column under it. -->
					<div
						class="mt-2 flex items-center gap-1 pr-1 text-xs font-semibold tracking-wide text-tertiary uppercase"
						draggable={sectionIndex !== -1}
						ondragstart={(event) => handleSectionDragStart(event, key)}
						ondragend={clearDrag}
						role="presentation"
					>
						<span
							class={pinnedReason ? 'cursor-not-allowed opacity-30' : 'cursor-grab'}
							title={pinnedReason ?? common_dragToReorderColumn({ column: heading })}
							aria-hidden="true"
						>
							<GripVertical class="h-4 w-4" />
						</span>
						<span class="min-w-0 flex-1 truncate">{heading}</span>
						<span class="flex" data-tooltip={pinnedReason} use:tooltip>
							<button
								type="button"
								data-move="up"
								data-section={key}
								onclick={() => stepSection(key, -1)}
								disabled={!!pinnedReason || sectionIndex <= 0}
								aria-label={common_moveColumnUp({ column: heading })}
								class="rounded-sm p-0.5 transition-colors hover:text-primary disabled:opacity-30"
							>
								<ChevronUp class="h-4 w-4" />
							</button>
							<button
								type="button"
								data-move="down"
								data-section={key}
								onclick={() => stepSection(key, 1)}
								disabled={!!pinnedReason || sectionIndex === movableSections.length - 1}
								aria-label={common_moveColumnDown({ column: heading })}
								class="rounded-sm p-0.5 transition-colors hover:text-primary disabled:opacity-30"
							>
								<ChevronDown class="h-4 w-4" />
							</button>
						</span>
					</div>
				{/if}
				<ul class="space-y-0.5 {key !== null ? 'pl-3' : ''}">
					{#each sectionColumns as column (column.id)}
						{@const canMove = isMovableColumn(column)}
						{@const pinnedReason = canMove
							? undefined
							: column.primary
								? common_columnPinnedFirst({ column: column.label })
								: common_columnPinnedLast({ column: column.label })}
						{@const index = movable.indexOf(column.id)}
						{@const isTarget = dropTarget?.id === column.id && draggingId !== column.id}
						{@const grouped = column.id === groupedColumnId}
						<li
							class="flex items-center gap-1 rounded border-y-2 border-transparent px-1 py-0.5 {draggingId ===
							column.id
								? 'opacity-50'
								: ''} {isTarget && dropTarget?.position === 'before'
								? 'border-t-blue-500!'
								: ''} {isTarget && dropTarget?.position === 'after' ? 'border-b-blue-500!' : ''}"
							draggable={canMove}
							ondragstart={(event) => handleDragStart(event, column.id)}
							ondragover={(event) => handleDragOver(event, column.id)}
							ondrop={handleDrop}
							ondragend={clearDrag}
						>
							<span
								class="text-tertiary {pinnedReason
									? 'cursor-not-allowed opacity-30'
									: 'cursor-grab'}"
								title={pinnedReason ?? common_dragToReorderColumn({ column: column.label })}
								aria-hidden="true"
							>
								<GripVertical class="h-4 w-4" />
							</span>
							<label
								class="flex min-w-0 flex-1 items-center gap-2 {grouped
									? 'cursor-not-allowed opacity-50'
									: 'cursor-pointer'}"
								data-tooltip={grouped ? common_columnHiddenWhileGrouped() : undefined}
								use:tooltip
							>
								<input
									type="checkbox"
									checked={!grouped && visibility[column.id] !== false}
									disabled={grouped}
									onchange={() => onToggle(column.id)}
									class="checkbox-card h-4 w-4 rounded-sm"
								/>
								<span class="truncate text-sm text-secondary" title={column.label}
									>{column.label}</span
								>
							</label>
							<span class="flex" data-tooltip={pinnedReason} use:tooltip>
								<button
									type="button"
									data-move="up"
									data-column={column.id}
									onclick={() => step(column.id, -1)}
									disabled={!!pinnedReason || index <= 0}
									aria-label={common_moveColumnUp({ column: column.label })}
									class="rounded-sm p-0.5 text-tertiary transition-colors hover:text-primary disabled:opacity-30"
								>
									<ChevronUp class="h-4 w-4" />
								</button>
								<button
									type="button"
									data-move="down"
									data-column={column.id}
									onclick={() => step(column.id, 1)}
									disabled={!!pinnedReason || index === movable.length - 1}
									aria-label={common_moveColumnDown({ column: column.label })}
									class="rounded-sm p-0.5 text-tertiary transition-colors hover:text-primary disabled:opacity-30"
								>
									<ChevronDown class="h-4 w-4" />
								</button>
							</span>
						</li>
					{/each}
				</ul>
			</div>
		{/each}
	</div>
</Popover>
