<script lang="ts" context="module">
	import type { PortTypeMetadata, TypedTypeMetadata } from '#lib/shared/stores/metadata.ts';

	type PortType = TypedTypeMetadata<PortTypeMetadata>;

	export const PortTypeDisplay: EntityDisplayComponent<PortType, object> = {
		getId: (portType: PortType) => portType.id,
		getLabel: (portType: PortType) =>
			`${portType.metadata.number}/${portType.metadata.protocol.toLowerCase()} - ${portType.name}`,
		getDescription: (portType: PortType) => portType.description ?? '',
		getIcon: (portType: PortType) => createIconComponent(portType.icon ?? null),
		getIconColor: () => entities.getColorHelper('Port').icon,
		getTags: () => [],
		getCategory: (portType: PortType) => portType.category ?? ''
	};
</script>

<script lang="ts">
	import ListSelectItem from '#lib/shared/components/forms/selection/ListSelectItem.svelte';
	import { entities } from '#lib/shared/stores/metadata.ts';
	import { createIconComponent } from '#lib/shared/utils/styling.ts';
	import type { EntityDisplayComponent } from '../types';

	export let item: PortType;
	export let context = {};
</script>

<ListSelectItem {item} {context} displayComponent={PortTypeDisplay} />
