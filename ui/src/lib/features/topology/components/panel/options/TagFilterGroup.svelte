<script lang="ts">
	import { isApplicationTag, tagGroupLabel, tagIcon } from '#lib/features/tags/groups.ts';
	import type { TagGroup } from '#lib/features/tags/types/base.ts';
	import type { components } from '#lib/api/schema';
	import { UNTAGGED_SENTINEL, hoveredTag } from '../../../interactions';
	import FilterGroup, { type FilterItem } from './FilterGroup.svelte';
	import { common_untagged, topology_hiddenInEveryTopology } from '#lib/paraglide/messages.js';

	type TagType = components['schemas']['Tag'];
	type EntityType = components['schemas']['EntityDiscriminants'];

	let {
		label,
		tags,
		groups = [],
		visibilityTags = [],
		hiddenTagIds,
		onToggle,
		entityType,
		hasUntagged = false,
		applicationGroupId = null
	}: {
		label?: string;
		/** The tags entities of this type hold, in each group's order. */
		tags: TagType[];
		/** The organization's tag groups, in display order; each one heads its own tags. */
		groups?: TagGroup[];
		/** The Visibility group's tags: hidden in every topology, shown here but not toggleable. */
		visibilityTags?: TagType[];
		hiddenTagIds: string[];
		onToggle: (tagId: string) => void;
		// Typed entity discriminant ('Host' | 'Service' | 'Subnet' etc).
		entityType: EntityType;
		hasUntagged?: boolean;
		// The organization's Application tag group, whose tags draw the application icon.
		applicationGroupId?: string | null;
	} = $props();

	function itemFor(tag: TagType): FilterItem {
		return {
			value: tag.id,
			label: tag.name,
			color: tag.color as FilterItem['color'],
			icon: tagIcon(tag, applicationGroupId) ?? undefined,
			isShiny: isApplicationTag(tag, applicationGroupId)
		};
	}

	/**
	 * The untagged choice and tags in no group first, under no subheader; then one section per tag
	 * group, headed by its name, holding its tags in the group's order.
	 */
	let sections = $derived.by(() => {
		const loose: FilterItem[] = [];
		if (hasUntagged) {
			loose.push({ value: UNTAGGED_SENTINEL, label: common_untagged(), color: 'Gray' });
		}
		loose.push(...tags.filter((tag) => !tag.tag_group_id).map(itemFor));

		const grouped = groups
			.filter((group) => group.group_type !== 'Visibility')
			.map((group) => ({
				label: tagGroupLabel(group) ?? group.name,
				items: tags.filter((tag) => tag.tag_group_id === group.id).map(itemFor)
			}))
			.filter((section) => section.items.length > 0);

		return [...(loose.length > 0 ? [{ label: undefined, items: loose }] : []), ...grouped];
	});

	let visibilityGroup = $derived(groups.find((group) => group.group_type === 'Visibility') ?? null);
	let visibilityItems = $derived(
		visibilityTags.map((tag) => ({ ...itemFor(tag), tooltip: topology_hiddenInEveryTopology() }))
	);

	function handleHoverStart(value: string, color: string) {
		hoveredTag.set({ tagId: value, color: color as string, entityType });
	}

	function handleHoverEnd() {
		hoveredTag.set(null);
	}
</script>

<div class="space-y-2">
	{#if label}
		<div class="text-xs text-tertiary">{label}</div>
	{/if}
	{#each sections as section, index (section.label ?? `loose-${index}`)}
		<FilterGroup
			items={section.items}
			selectedValues={hiddenTagIds}
			mode="exclude"
			{onToggle}
			onHoverStart={handleHoverStart}
			onHoverEnd={handleHoverEnd}
			label={section.label}
		/>
	{/each}
	{#if visibilityGroup && visibilityItems.length > 0}
		<FilterGroup
			items={visibilityItems}
			selectedValues={visibilityItems.map((item) => item.value)}
			mode="exclude"
			onToggle={() => {}}
			disabled
			nativeTooltip
			label={tagGroupLabel(visibilityGroup) ?? visibilityGroup.name}
		/>
	{/if}
</div>
