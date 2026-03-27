<script lang="ts">
	import type { IfEntry } from '$lib/features/hosts/types/base';
	import ListConfigEditor from '$lib/shared/components/forms/selection/ListConfigEditor.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import EntityConfigEmpty from '$lib/shared/components/forms/EntityConfigEmpty.svelte';
	import { IfEntryDisplay } from '$lib/shared/components/forms/selection/display/IfEntryDisplay.svelte';
	import IfEntryConfigPanel from './IfEntryConfigPanel.svelte';
	import {
		hosts_ifEntries_emptySubtitle,
		hosts_ifEntries_emptyTitle,
		hosts_ifEntries_noInterfaces,
		hosts_ifEntries_selectToView,
		hosts_ifEntries_subtitle,
		hosts_ifEntries_title
	} from '$lib/paraglide/messages';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';

	interface Props {
		ifEntries: IfEntry[];
		targetEntityId?: string | null;
	}

	let { ifEntries, targetEntityId = $bindable(null) }: Props = $props();

	// Sort if_entries by if_index
	let sortedIfEntries = $derived([...ifEntries].sort((a, b) => a.if_index - b.if_index));
</script>

<div class="flex min-h-0 flex-1 flex-col">
	<ListConfigEditor items={sortedIfEntries} bind:targetEntityId>
		<svelte:fragment slot="list" let:items let:onEdit let:highlightedIndex>
			<ListManager
				label={hosts_ifEntries_title({ count: items.length })}
				helpText={hosts_ifEntries_subtitle()}
				emptyMessage={hosts_ifEntries_noInterfaces()}
				{items}
				itemClickAction="edit"
				allowReorder={false}
				allowAddFromOptions={false}
				allowItemRemove={() => false}
				options={[] as IfEntry[]}
				itemDisplayComponent={IfEntryDisplay}
				optionDisplayComponent={IfEntryDisplay}
				{onEdit}
				{highlightedIndex}
			/>
		</svelte:fragment>

		<svelte:fragment slot="config" let:selectedItem>
			{#if selectedItem}
				<IfEntryConfigPanel ifEntry={selectedItem} />
			{:else if ifEntries.length === 0}
				<EntityConfigEmpty
					title={hosts_ifEntries_emptyTitle()}
					subtitle={hosts_ifEntries_emptySubtitle()}
				/>
			{:else}
				<EntityConfigEmpty
					title={hosts_ifEntries_noInterfaces()}
					subtitle={hosts_ifEntries_selectToView()}
				/>
			{/if}
		</svelte:fragment>
	</ListConfigEditor>

	<EntityMetadataSection entities={ifEntries} />
</div>
