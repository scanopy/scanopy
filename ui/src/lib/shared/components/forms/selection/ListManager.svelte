<script lang="ts" generics="T, V, OC, IC, W = unknown, SOC = unknown">
	import { noneOf } from '$lib/shared/utils/embed-i18n';
	import { ArrowUp, ArrowDown, Trash2, Plus, Edit, Square, CheckSquare } from 'lucide-svelte';
	import RichSelect from './RichSelect.svelte';
	import SegmentedControl from '../SegmentedControl.svelte';
	import ListSelectItem from './ListSelectItem.svelte';
	import type { EntityDisplayComponent } from './types';
	import type { Snippet } from 'svelte';

	interface Props {
		// Global
		label: string;
		helpText?: string;
		helpSnippet?: Snippet;
		headerSnippet?: Snippet;
		placeholder?: string;
		required?: boolean;
		allowReorder?: boolean;
		allowAddFromOptions?: boolean;
		allowCreateNew?: boolean;
		allowSelection?: boolean;
		disableCreateNewButton?: boolean;
		createNewLabel?: string;
		highlightedIndex?: number;
		emptyMessage?: string;
		error?: string;

		// Options (dropdown)
		options?: V[];
		optionDisplayComponent: EntityDisplayComponent<V, OC>;
		getOptionContext?: (option: V, index: number) => OC;
		showSearch?: boolean;

		// Secondary options (dual-mode dropdown)
		secondaryOptions?: W[];
		secondaryOptionDisplayComponent?: EntityDisplayComponent<W, SOC>;
		secondaryPlaceholder?: string;
		primaryOptionsLabel?: string;
		secondaryOptionsLabel?: string;
		onAddSecondary?: (selectOptionId: string) => void;
		getSecondaryOptionContext?: (option: W, index: number) => SOC;

		// Items
		items?: T[];
		itemDisplayComponent: EntityDisplayComponent<T, IC>;
		getItemContext?: (item: T, index: number) => IC;

		// Item interaction
		allowDuplicates?: boolean;
		itemClickAction?: 'edit' | 'select' | null;
		allowItemEdit?: (item: T) => boolean;
		allowItemRemove?: (item: T) => boolean;
		allowItemReorder?: (item: T) => boolean;
		stickyHeader?: boolean;
		reorderDisabledTooltip?: string;
		isItemEditing?: (item: T, index: number) => boolean;
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		editIcon?: (item: T, index: number) => any;
		editButtonClass?: (item: T, index: number) => string;
		selectedItems?: T[];

		// Interaction handlers
		onCreateNew?: (() => void) | null;
		onEdit?: (item: T, index: number) => void;
		onAdd?: (selectOptionId: string) => void;
		onMoveUp?: (fromIndex: number, toIndex: number) => void;
		onMoveDown?: (fromIndex: number, toIndex: number) => void;
		onRemove?: (index: number) => void;
		onClick?: (item: T, index: number) => void;
		onItemUpdate?: (item: T, index: number, updates: Partial<T>) => void;

		// Snippets (slots)
		itemSnippet?: Snippet<[{ item: T; index: number }]>;
		itemExpandedSnippet?: Snippet<[{ item: T; index: number }]>;
	}

	let {
		// Global
		label,
		helpText = '',
		helpSnippet,
		headerSnippet,
		placeholder = 'Select an item to add',
		required = false,
		allowReorder = true,
		allowAddFromOptions = true,
		allowCreateNew = false,
		allowSelection = false,
		disableCreateNewButton = false,
		createNewLabel = 'Create New',
		highlightedIndex = -1,
		emptyMessage = '',
		error = '',

		// Options (dropdown)
		options = [] as V[],
		optionDisplayComponent,
		getOptionContext = () => ({}) as OC,
		showSearch = false,

		// Secondary options (dual-mode dropdown)
		secondaryOptions = undefined,
		secondaryOptionDisplayComponent = undefined,
		secondaryPlaceholder = '',
		primaryOptionsLabel = '',
		secondaryOptionsLabel = '',
		onAddSecondary = () => {},
		getSecondaryOptionContext = () => ({}) as SOC,

		// Items
		items = [] as T[],
		itemDisplayComponent,
		getItemContext = () => ({}) as IC,
		selectedItems = $bindable([]),

		// Item interaction
		allowDuplicates = false,
		itemClickAction = null,
		allowItemEdit = () => true,
		allowItemRemove = () => true,
		allowItemReorder = () => true,
		stickyHeader = false,
		reorderDisabledTooltip = undefined,
		isItemEditing = () => false,
		editIcon = undefined,
		editButtonClass = undefined,

		// Interaction handlers
		onCreateNew = null,
		onEdit = () => {},
		onAdd = () => {},
		onMoveUp = () => {},
		onMoveDown = () => {},
		onRemove = () => {},
		onClick = () => {},
		onItemUpdate = () => {},

		itemSnippet,
		itemExpandedSnippet
	}: Props = $props();

	// Internal state
	let selectedOptionId = $state('');
	let editingIndex = $state<number | null>(null);
	let optionMode = $state<'primary' | 'secondary'>('primary');

	let hasDualMode = $derived(
		!!secondaryOptionDisplayComponent &&
			!!secondaryOptionsLabel &&
			(secondaryOptions?.length ?? 0) > 0
	);

	// Auto-switch mode if current mode's options become empty
	$effect(() => {
		if (optionMode === 'secondary' && (secondaryOptions?.length ?? 0) === 0) {
			optionMode = 'primary';
		} else if (
			optionMode === 'primary' &&
			options.length === 0 &&
			(secondaryOptions?.length ?? 0) > 0
		) {
			optionMode = 'secondary';
		}
	});

	let computedEmptyMessage = $derived(emptyMessage || noneOf(label));

	function addItem() {
		if (selectedOptionId) {
			// Check for duplicates only if allowDuplicates is false
			if (!allowDuplicates) {
				const isDuplicate = items.some((item) => {
					const itemId = itemDisplayComponent.getId(item);
					return itemId === selectedOptionId;
				});

				if (isDuplicate) {
					return; // Don't add duplicates
				}
			}

			// Call the parent's onAdd callback with the option ID
			onAdd(selectedOptionId);
			selectedOptionId = '';
		}
	}

	function removeItem(index: number) {
		// Reset editing index if we're removing the item being edited
		if (editingIndex === index) {
			editingIndex = null;
		} else if (editingIndex !== null && editingIndex > index) {
			// Adjust editing index if it's after the removed item
			editingIndex = editingIndex - 1;
		}
		onRemove(index);
	}

	function moveItemUp(index: number) {
		if (index > 0 && allowReorder) {
			onMoveUp(index, index - 1);
		}
	}

	function moveItemDown(index: number) {
		if (index < items.length - 1 && allowReorder) {
			onMoveDown(index, index + 1);
		}
	}

	function handleDropdownSelectChange(value: string) {
		selectedOptionId = value;
		if (value) {
			addItem();
		}
	}

	function handleSecondaryDropdownSelectChange(value: string) {
		if (value) {
			onAddSecondary(value);
		}
	}

	function isItemSelected(item: T): boolean {
		const itemId = itemDisplayComponent.getId(item);
		return selectedItems.some((selected) => itemDisplayComponent.getId(selected) === itemId);
	}

	function toggleItemSelection(item: T) {
		const itemId = itemDisplayComponent.getId(item);
		const isCurrentlySelected = isItemSelected(item);

		if (isCurrentlySelected) {
			selectedItems = selectedItems.filter((s) => itemDisplayComponent.getId(s) !== itemId);
		} else {
			selectedItems = [...selectedItems, item];
		}
	}

	function selectAll() {
		selectedItems = [...items];
	}

	function selectNone() {
		selectedItems = [];
	}
</script>

<div class={stickyHeader ? 'flex min-h-0 flex-1 flex-col' : ''}>
	<div class={`mb-2 flex items-start justify-between gap-4${stickyHeader ? ' flex-shrink-0' : ''}`}>
		<div class="min-w-0 flex-1">
			<div class="text-secondary flex items-center gap-2 text-sm font-medium">
				<span>
					{label}
					{#if required}<span class="text-danger">*</span>{/if}
				</span>
				{#if headerSnippet}
					{@render headerSnippet()}
				{/if}
			</div>
			{#if helpSnippet}
				<div class="text-tertiary mt-1 text-sm">
					{@render helpSnippet()}
				</div>
			{:else if helpText}
				<p class="text-tertiary mt-1 text-sm">
					{helpText}
				</p>
			{/if}
		</div>

		{#if allowSelection && items.length > 0}
			{@const anySelected = selectedItems.length > 0}
			<button
				onclick={anySelected ? selectNone : selectAll}
				class="btn-secondary flex items-center gap-2"
				type="button"
				title={anySelected ? 'Deselect all' : 'Select all'}
			>
				{#if anySelected}
					<Square class="h-4 w-4" />
				{:else}
					<CheckSquare class="h-4 w-4" />
				{/if}
				{anySelected ? 'None' : 'All'}
			</button>
		{/if}

		{#if allowCreateNew && onCreateNew}
			<button
				type="button"
				disabled={disableCreateNewButton}
				onclick={() => onCreateNew()}
				class="btn-primary"
			>
				<Plus size={16} />
				{createNewLabel}
			</button>
		{/if}
	</div>

	<!-- Add Item Section with RichSelect -->
	{#if allowAddFromOptions}
		<div class="mb-3 mt-4 space-y-2">
			{#if hasDualMode}
				<SegmentedControl
					options={[
						{ value: 'primary', label: primaryOptionsLabel },
						{ value: 'secondary', label: secondaryOptionsLabel }
					]}
					selected={optionMode}
					onchange={(v) => (optionMode = v as 'primary' | 'secondary')}
					size="sm"
					fullWidth={true}
				/>
			{/if}
			<div class="flex gap-2">
				<div class="flex-1">
					{#if optionMode === 'primary' || !secondaryOptionDisplayComponent}
						<RichSelect
							selectedValue={selectedOptionId}
							{showSearch}
							{options}
							{placeholder}
							onSelect={handleDropdownSelectChange}
							displayComponent={optionDisplayComponent}
							{getOptionContext}
						/>
					{:else if secondaryOptionDisplayComponent}
						<RichSelect
							selectedValue=""
							{showSearch}
							options={secondaryOptions ?? []}
							placeholder={secondaryPlaceholder || placeholder}
							onSelect={handleSecondaryDropdownSelectChange}
							displayComponent={secondaryOptionDisplayComponent}
							getOptionContext={getSecondaryOptionContext}
						/>
					{/if}
				</div>
			</div>
		</div>
	{/if}

	<!-- Current Items -->
	{#if items.length > 0}
		<div class={`mb-3 space-y-2 p-0.5${stickyHeader ? ' min-h-0 flex-1 overflow-y-auto' : ''}`}>
			{#each items as item, index (itemDisplayComponent.getId(item))}
				{@const isHighlighted = highlightedIndex === index}

				<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
				<div
					class="
						card flex flex-wrap items-center gap-3 rounded-lg border p-3 transition-all
						{isHighlighted ? 'card-focused' : isItemSelected(item) ? 'card-selected' : ''}"
					onclick={() => {
						onClick(item, index);
						if (allowSelection && itemClickAction == 'select') {
							toggleItemSelection(item);
						} else if (allowItemEdit(item)) {
							if (!itemSnippet && itemDisplayComponent.supportsInlineEdit) {
								// Toggle inline editing for this item
								editingIndex = editingIndex === index ? null : index;
							} else {
								onEdit(item, index);
							}
						}
					}}
					tabindex={allowItemEdit(item) || allowSelection ? 0 : -1}
					role={allowSelection ? 'checkbox' : allowItemEdit(item) ? 'button' : undefined}
					aria-checked={allowSelection ? isItemSelected(item) : undefined}
				>
					<!-- Selection checkbox -->
					{#if allowSelection && itemClickAction != 'select'}
						<div class="flex-shrink-0">
							<input
								type="checkbox"
								checked={isItemSelected(item)}
								onclick={() => toggleItemSelection(item)}
								class="checkbox-card h-4 w-4"
							/>
						</div>
					{/if}

					<!-- Use slot if provided, otherwise check for inline editing -->
					<div class="min-w-0 flex-1 overflow-hidden">
						{#if itemSnippet}
							{@render itemSnippet({ item, index })}
						{:else}
							{@const context = getItemContext(item, index)}
							{#if editingIndex === index && itemDisplayComponent.supportsInlineEdit && itemDisplayComponent.InlineEditorComponent}
								{@const InlineEditor = itemDisplayComponent.InlineEditorComponent}
								{@const ctx = context as Record<string, unknown>}
								<InlineEditor
									binding={item}
									onUpdate={(updates: Partial<T>) => onItemUpdate(item, index, updates)}
									service={ctx.service}
									host={ctx.host}
									services={ctx.services}
								/>
							{:else}
								<ListSelectItem {item} {context} displayComponent={itemDisplayComponent} />
							{/if}
						{/if}
					</div>

					<!-- Action Buttons -->
					<div class="flex items-center gap-1">
						{#if allowItemEdit(item) && itemClickAction != 'edit'}
							{@const EditIconComponent = editIcon ? editIcon(item, index) : Edit}
							{@const btnClass = editButtonClass ? editButtonClass(item, index) : 'btn-icon'}
							<button
								type="button"
								onclick={(e) => {
									e.stopPropagation();
									if (itemDisplayComponent.supportsInlineEdit) {
										editingIndex = editingIndex === index ? null : index;
									} else {
										onEdit(item, index);
									}
								}}
								class={btnClass}
								title="Edit"
							>
								<EditIconComponent size={16} />
							</button>
						{/if}

						{#if !isItemEditing(item, index) && allowReorder && allowItemReorder(item)}
							<button
								type="button"
								onclick={(e) => {
									e.stopPropagation();
									moveItemUp(index);
								}}
								disabled={index === 0 ||
									!allowItemReorder(items[index - 1]) ||
									!!reorderDisabledTooltip}
								class="btn-icon"
								title={reorderDisabledTooltip ?? 'Move up'}
							>
								<ArrowUp size={16} />
							</button>

							<button
								type="button"
								onclick={(e) => {
									e.stopPropagation();
									moveItemDown(index);
								}}
								disabled={index === items.length - 1 ||
									!allowItemReorder(items[index + 1]) ||
									!!reorderDisabledTooltip}
								class="btn-icon"
								title={reorderDisabledTooltip ?? 'Move down'}
							>
								<ArrowDown size={16} />
							</button>
						{/if}

						{#if !isItemEditing(item, index) && allowItemRemove(item)}
							<button
								type="button"
								onclick={(e) => {
									e.stopPropagation();
									removeItem(index);
								}}
								class="btn-icon-danger"
								title="Remove"
							>
								<Trash2 size={16} />
							</button>
						{/if}
					</div>

					<!-- Expanded content panel — full card width, below the header row -->
					{#if itemExpandedSnippet}
						{@render itemExpandedSnippet({ item, index })}
					{/if}
				</div>
			{/each}
		</div>
	{:else if computedEmptyMessage}
		<div
			class="text-secondary rounded-lg border border-dashed border-gray-400 py-4 text-center text-sm dark:border-gray-500"
		>
			{computedEmptyMessage}
		</div>
	{/if}

	<!-- Error Message -->
	{#if error}
		<div class="text-danger mt-2 text-sm">
			{error}
		</div>
	{/if}
</div>
