<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import type { TopologyEditState } from '$lib/features/topology/state';
	import type { ElementRenderContext } from '$lib/features/topology/resolvers';
	import { useTagsQuery, type EntityDiscriminants } from '$lib/features/tags/queries';
	import { concepts } from '$lib/shared/stores/metadata';
	import {
		common_ungrouped,
		tags_inheritedFromHost,
		tags_inheritedOverrideHint,
		common_overrides,
		tags_fromHost
	} from '$lib/paraglide/messages';
	import InspectorSection from '../shared/InspectorSection.svelte';

	/* eslint-disable @typescript-eslint/no-unused-vars -- component contract props */
	let {
		node,
		topology,
		editState,
		elementContext
	}: {
		node: Node;
		topology: RenderableTopology;
		editState: TopologyEditState;
		elementContext?: ElementRenderContext;
	} = $props();
	/* eslint-enable @typescript-eslint/no-unused-vars */

	let entityId = $derived.by((): string | undefined => {
		if (elementContext?.services.length) return elementContext.services[0].id;
		return undefined;
	});

	let entityType = $derived('Service' as EntityDiscriminants);

	let selectedTagIds = $derived.by((): string[] => {
		if (elementContext?.services.length) return elementContext.services[0].tags;
		return [];
	});

	// Merge topology entity_tags with tags query cache for newly created tags
	const tagsQuery = useTagsQuery();
	let entityTags = $derived.by(() => {
		const topoTags = topology?.entity_tags ?? [];
		const cachedTags = tagsQuery.data ?? [];
		const topoIds = new Set(topoTags.map((t) => t.id));
		return [...topoTags, ...cachedTags.filter((t) => !topoIds.has(t.id))];
	});

	// App tags
	let appTags = $derived(entityTags.filter((t) => t.is_application));
	let appTagIds = $derived(new Set(appTags.map((t) => t.id)));

	// Selected app tags: direct first, then inherited from host
	let selectedAppTagIds = $derived.by(() => {
		const direct = selectedTagIds.filter((id) => appTagIds.has(id));
		if (direct.length > 0) return direct;
		if (elementContext?.host) {
			return elementContext.host.tags.filter((id) => appTagIds.has(id));
		}
		return [];
	});
	let hasAppTag = $derived(selectedAppTagIds.length > 0);

	// Inherited vs direct vs override
	let isAppInherited = $derived(hasAppTag && !selectedTagIds.some((id) => appTagIds.has(id)));

	let hostAppTagId = $derived.by(() => {
		if (!elementContext?.host) return null;
		return elementContext.host.tags.find((id) => appTagIds.has(id)) ?? null;
	});

	let isAppOverride = $derived(
		!isAppInherited && hostAppTagId !== null && !selectedAppTagIds.includes(hostAppTagId)
	);

	let hostAppTag = $derived(hostAppTagId ? appTags.find((t) => t.id === hostAppTagId) : null);

	let currentAppTag = $derived(
		hasAppTag ? (appTags.find((t) => selectedAppTagIds.includes(t.id)) ?? null) : null
	);

	let appAvailableTags = $derived(
		hasAppTag ? appTags.filter((t) => selectedAppTagIds.includes(t.id)) : appTags
	);

	// Bindable dropdown state and local dismissal for the Ungrouped pseudotag.
	// Clicking X on the pseudotag hides it and opens the picker. If the user closes
	// the dropdown without picking a tag and the service is still ungrouped, the
	// pseudotag returns. No flash-guard needed here — TagPickerInline runs in entity
	// mode and awaits the bulk-add mutation before closing, so `hasAppTag` has
	// already transitioned by the time `open` flips back to false.
	let pickerOpen = $state(false);
	let ungroupedDismissed = $state(false);
	let showUngroupedPseudo = $derived(!hasAppTag && !ungroupedDismissed);

	$effect(() => {
		if (!pickerOpen && ungroupedDismissed && !hasAppTag) {
			ungroupedDismissed = false;
		}
	});
</script>

{#if entityId}
	<InspectorSection id="Application" section="Application">
		<div class="card card-static space-y-2 p-2">
			{#if hasAppTag && currentAppTag}
				<div class="flex flex-wrap items-center gap-1">
					<Tag
						label={currentAppTag.name}
						color={currentAppTag.color}
						icon={concepts.getIconComponent('Application')}
						isShiny={true}
					/>
					{#if isAppInherited}
						<span class="text-tertiary text-xs">{tags_inheritedFromHost()}</span>
					{:else if isAppOverride && hostAppTag}
						<span class="text-tertiary text-xs">{common_overrides()}</span>
						<Tag
							label={hostAppTag.name}
							color={hostAppTag.color}
							icon={concepts.getIconComponent('Application')}
							isShiny={true}
						/>
						<span class="text-tertiary text-xs">{tags_fromHost()}</span>
					{/if}
				</div>
				{#if isAppInherited}
					<p class="text-tertiary text-xs">{tags_inheritedOverrideHint()}</p>
				{/if}
			{/if}
			<div class="flex flex-wrap items-center gap-1.5">
				{#if showUngroupedPseudo}
					<Tag
						label={common_ungrouped()}
						color="Gray"
						icon={concepts.getIconComponent('Application')}
						isShiny={true}
						pill={true}
						removable={editState.isEditable}
						onRemove={() => {
							ungroupedDismissed = true;
							pickerOpen = true;
						}}
					/>
				{/if}
				<TagPickerInline
					bind:open={pickerOpen}
					selectedTagIds={isAppInherited ? [] : selectedAppTagIds}
					{entityId}
					{entityType}
					disabled={!editState.isEditable}
					availableTags={isAppInherited ? appTags : appAvailableTags}
					allowCreate={!hasAppTag || isAppInherited}
					createAsApplication={true}
					hideAddButton={showUngroupedPseudo}
				/>
			</div>
		</div>
	</InspectorSection>
{/if}
