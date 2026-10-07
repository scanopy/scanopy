<script lang="ts">
	import type { Interface } from '$lib/features/hosts/types/base';
	import ListConfigEditor from '$lib/shared/components/forms/selection/ListConfigEditor.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import EntityConfigEmpty from '$lib/shared/components/forms/EntityConfigEmpty.svelte';
	import { InterfaceDisplay } from '$lib/shared/components/forms/selection/display/InterfaceDisplay.svelte';
	import InterfaceConfigPanel from './InterfaceConfigPanel.svelte';
	import {
		hosts_interfaces_emptySubtitle,
		hosts_noInterfaces,
		hosts_interfaces_selectToView,
		hosts_interfaces_subtitle,
		common_interfaces
	} from '$lib/paraglide/messages';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';

	interface Props {
		interfaces: Interface[];
		targetEntityId?: string | null;
	}

	let { interfaces, targetEntityId = $bindable(null) }: Props = $props();

	// Sorted by if_index, with ports that never had one read sorting last.
	let sortedInterfaces = $derived(
		[...interfaces].sort((a, b) => (a.if_index ?? Infinity) - (b.if_index ?? Infinity))
	);
</script>

<div class="flex min-h-0 flex-1 flex-col">
	<ListConfigEditor items={sortedInterfaces} bind:targetEntityId>
		<svelte:fragment slot="list" let:items let:onEdit let:highlightedIndex>
			<ListManager
				label={common_interfaces({ count: items.length })}
				helpText={hosts_interfaces_subtitle()}
				emptyMessage={hosts_noInterfaces()}
				{items}
				itemClickAction="edit"
				allowReorder={false}
				allowAddFromOptions={false}
				allowItemRemove={() => false}
				options={[] as Interface[]}
				itemDisplayComponent={InterfaceDisplay}
				optionDisplayComponent={InterfaceDisplay}
				{onEdit}
				{highlightedIndex}
			/>
		</svelte:fragment>

		<svelte:fragment slot="config" let:selectedItem>
			{#if selectedItem}
				<InterfaceConfigPanel iface={selectedItem} />
			{:else if interfaces.length === 0}
				<EntityConfigEmpty
					title={hosts_noInterfaces()}
					subtitle={hosts_interfaces_emptySubtitle()}
				/>
			{:else}
				<EntityConfigEmpty
					title={hosts_noInterfaces()}
					subtitle={hosts_interfaces_selectToView()}
				/>
			{/if}
		</svelte:fragment>
	</ListConfigEditor>

	<EntityMetadataSection entities={interfaces} />
</div>
