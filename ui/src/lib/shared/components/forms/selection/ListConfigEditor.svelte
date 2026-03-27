<script lang="ts" generics="TItem">
	import Loading from '../../feedback/Loading.svelte';

	// Core data
	export let items: TItem[] = [];

	// Layout configuration
	export let listPanelWidth: string = 'w-2/5';
	export let configPanelWidth: string = 'w-3/5';
	export let loading: boolean = false;

	// Event handlers
	export let onReorder: (fromIndex: number, toIndex: number) => void = () => {};
	export let onChange: (item: TItem, index: number) => void = () => {};
	export let onItemSelect: (item: TItem, index: number) => void = () => {};

	// Deep-link target: when set, auto-selects the matching item
	export let targetEntityId: string | null = null;

	// Internal state
	let selectedIndex: number = -1;

	// Computed values for slot consumers
	$: selectedItem = selectedIndex >= 0 ? items[selectedIndex] : null;

	// Track previous items length to detect when items are added
	let previousItemsLength = 0;
	$: {
		if (items.length > previousItemsLength) {
			// Items were added, select the last one
			selectedIndex = items.length - 1;
		} else if (items.length === 1 && selectedIndex === -1) {
			// Auto-select the first (and only) item when there's exactly one item
			selectedIndex = 0;
		} else if (items.length === 0) {
			// Clear selection when no items
			selectedIndex = -1;
		}
		previousItemsLength = items.length; // eslint-disable-line no-useless-assignment
	}

	// Auto-select deep-linked sub-entity when targetEntityId is set
	$: if (targetEntityId && items.length > 0) {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const index = items.findIndex((item: any) => item.id === targetEntityId);
		if (index >= 0) {
			selectedIndex = index;
			targetEntityId = null;
		}
	}

	// Event handlers
	function handleEdit(item: TItem, index: number) {
		selectedIndex = index;
	}

	function handleItemChange(updatedItem: TItem) {
		if (selectedIndex >= 0 && selectedIndex < items.length) {
			// Don't mutate items directly - let the parent handle updates via callback
			onChange(updatedItem, selectedIndex);
		}
	}

	function handleMoveUp(fromIndex: number, toIndex: number) {
		// Update items first via callback, then update selectedIndex
		// This ensures selectedItem is computed with both changes applied
		onReorder(fromIndex, toIndex);

		// When an item moves up: fromIndex > toIndex
		if (selectedIndex === fromIndex) {
			// The selected item moved up
			selectedIndex = toIndex;
		} else if (selectedIndex >= toIndex && selectedIndex < fromIndex) {
			// Selected item got pushed down by the moving item
			selectedIndex = selectedIndex + 1;
		}
	}

	function handleMoveDown(fromIndex: number, toIndex: number) {
		// Update items first via callback, then update selectedIndex
		// This ensures selectedItem is computed with both changes applied
		onReorder(fromIndex, toIndex);

		// When an item moves down: fromIndex < toIndex
		if (selectedIndex === fromIndex) {
			// The selected item moved down
			selectedIndex = toIndex;
		} else if (selectedIndex > fromIndex && selectedIndex <= toIndex) {
			// Selected item got pushed up by the moving item
			selectedIndex = selectedIndex - 1;
		}
	}
</script>

{#if loading}
	<div class="flex h-full items-center justify-center">
		<Loading />
	</div>
{:else}
	<div class="relative min-h-0 flex-1">
		<div class="absolute inset-0 flex gap-6">
			<!-- Left Panel - List Manager -->
			<div class="{listPanelWidth} min-h-0 overflow-y-auto">
				<div class="p-6">
					<slot
						name="list"
						{items}
						{selectedIndex}
						onEdit={handleEdit}
						onMoveUp={handleMoveUp}
						onMoveDown={handleMoveDown}
						{onItemSelect}
						highlightedIndex={selectedIndex}
						highlightedItem={selectedItem}
					>
						<!-- Default slot content if no list slot provided -->
						<div class="text-danger">No list component provided</div>
					</slot>
				</div>
			</div>

			<!-- Right Panel - Configuration -->
			<div class="{configPanelWidth} min-h-0 overflow-y-auto border-l border-gray-600 p-6">
				<slot name="config" {selectedItem} {selectedIndex} onChange={handleItemChange}>
					<div class="text-tertiary flex h-32 items-center justify-center">
						<p>Select an item to configure</p>
					</div>
				</slot>
			</div>
		</div>
	</div>
{/if}
