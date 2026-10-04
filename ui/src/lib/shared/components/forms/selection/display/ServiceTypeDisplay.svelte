<script lang="ts" module>
	import type { ServicedDefinitionMetadata, TypedTypeMetadata } from '$lib/shared/stores/metadata';

	type ServiceType = TypedTypeMetadata<ServicedDefinitionMetadata>;

	export type ServiceTypeTagRole = 'category';

	export const ServiceTypeDisplay: EntityDisplayComponent<
		ServiceType,
		DisplayTagContext<ServiceTypeTagRole>
	> = {
		getId: (serviceType: ServiceType) => serviceType.id,
		getLabel: (serviceType: ServiceType) => serviceType.name ?? '',
		getDescription: (serviceType: ServiceType) => serviceType.description ?? '',
		getIcon: (serviceType: ServiceType) => serviceDefinitions.getIconComponent(serviceType.id),
		getIconColor: (serviceType: ServiceType) =>
			serviceDefinitions.getColorHelper(serviceType.id).icon,
		getTags: (serviceType: ServiceType) => [
			{
				label: serviceType.category ?? '',
				color: serviceType.color ?? undefined,
				role: 'category' satisfies ServiceTypeTagRole
			}
		],
		getCategory: (serviceType: ServiceType) => serviceType.category ?? ''
	};
</script>

<script lang="ts">
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';
	import type { DisplayTagContext, EntityDisplayComponent } from '../types';

	export let item: ServiceType;
	export let context: DisplayTagContext<ServiceTypeTagRole> = {};
</script>

<ListSelectItem {item} {context} displayComponent={ServiceTypeDisplay} />
