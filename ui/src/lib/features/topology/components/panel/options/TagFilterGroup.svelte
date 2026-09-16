<script lang="ts">
	import type { components } from '$lib/api/schema';
	import { UNTAGGED_SENTINEL, hoveredTag } from '../../../interactions';
	import FilterGroup, { type FilterItem } from './FilterGroup.svelte';
	import { concepts } from '$lib/shared/stores/metadata';
	import { common_untagged } from '$lib/paraglide/messages';

	type TagType = components['schemas']['Tag'];
	type EntityType = components['schemas']['EntityDiscriminants'];

	let {
		label,
		tags,
		hiddenTagIds,
		onToggle,
		entityType,
		hasUntagged = false
	}: {
		label?: string;
		tags: TagType[];
		hiddenTagIds: string[];
		onToggle: (tagId: string) => void;
		// Typed entity discriminant ('Host' | 'Service' | 'Subnet' etc).
		// Callers previously passed lowercase 'host' | 'service' | 'subnet' —
		// callers are updated to pass the typed value.
		entityType: EntityType;
		hasUntagged?: boolean;
	} = $props();

	// Build items list with untagged sentinel first, then real tags
	let items = $derived.by(() => {
		const result: FilterItem[] = [];
		if (hasUntagged) {
			result.push({ value: UNTAGGED_SENTINEL, label: common_untagged(), color: 'Gray' });
		}
		for (const tag of tags) {
			const isApp = tag.is_application ?? false;
			result.push({
				value: tag.id,
				label: tag.name,
				color: tag.color as FilterItem['color'],
				icon: isApp ? concepts.getIconComponent('Application') : undefined,
				isShiny: isApp
			});
		}
		return result;
	});

	function handleHoverStart(value: string, color: string) {
		hoveredTag.set({ tagId: value, color: color as string, entityType });
	}

	function handleHoverEnd() {
		hoveredTag.set(null);
	}
</script>

<FilterGroup
	{items}
	selectedValues={hiddenTagIds}
	mode="exclude"
	{onToggle}
	onHoverStart={handleHoverStart}
	onHoverEnd={handleHoverEnd}
	{label}
/>
