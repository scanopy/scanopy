<script lang="ts" module>
	import { entities } from '#lib/shared/stores/metadata.ts';
	import type { EntityTypeMetadata, TypedTypeMetadata } from '#lib/shared/stores/metadata.ts';

	export type EntityTypeOption = TypedTypeMetadata<EntityTypeMetadata>;

	/** An entity type as a row: its plural name, icon and color, from the entity metadata. */
	export const EntityTypeDisplay: EntityDisplayComponent<EntityTypeOption, object> = {
		getId: (item) => item.id,
		getLabel: (item) => item.metadata?.entity_name_plural ?? entities.getName(item.id),
		getIcon: (item) => entities.getIconComponent(item.id),
		getIconColor: (item) => entities.getColorHelper(item.id).icon,
		getCategory: () => null
	};
</script>

<script lang="ts">
	import type { EntityDisplayComponent } from '../types';
	import ListSelectItem from '../ListSelectItem.svelte';

	interface Props {
		item: EntityTypeOption;
		context?: object;
	}

	let { item, context = {} }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={EntityTypeDisplay} />
