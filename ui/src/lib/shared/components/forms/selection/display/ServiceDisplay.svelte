<script lang="ts" module>
	import { concepts, serviceCategories, serviceDefinitions } from '$lib/shared/stores/metadata';
	import type { Port } from '$lib/features/hosts/types/base';
	import { services_bindingCount } from '$lib/paraglide/messages';

	export type ServiceTagRole = 'category' | 'virtualization';

	export interface ServiceDisplayContext extends DisplayTagContext<ServiceTagRole> {
		ipAddressId?: string | null;
		ports?: Port[];
		showEntityTagPicker?: boolean;
		tagPickerDisabled?: boolean;
		entityTags?: import('$lib/features/tags/types/base').Tag[];
		allowTagCreate?: boolean;
	}

	export const ServiceDisplay: EntityDisplayComponent<Service, ServiceDisplayContext> = {
		getId: (service: Service) => service.id,
		getLabel: (service: Service) => service.name,
		getDescription: (service: Service, context) => {
			let descriptionItems = [];

			// Show service definition name if different from service name
			const defName = serviceDefinitions.getName(service.service_definition);
			if (defName && defName !== service.name) {
				descriptionItems.push(defName);
			}

			// Filter bindings relevant to the interface(s)
			let bindingsOnInterface = service.bindings.filter((b) =>
				b.ip_address_id
					? context.ipAddressId == b.ip_address_id || context.ipAddressId == null
					: true
			);

			// Show actual port numbers when ports are available in context
			if (context.ports && context.ports.length > 0) {
				const portBindings = bindingsOnInterface.filter((b) => b.type === 'Port');
				let bindingDescriptions: string[] = [];

				if (portBindings.length > 0) {
					for (const binding of portBindings) {
						const port = binding.port_id
							? context.ports.find((p) => p.id === binding.port_id)
							: null;

						if (port) {
							bindingDescriptions.push(formatPort(port));
						}
					}
				}

				if (bindingDescriptions.length > 0) {
					descriptionItems.push(bindingDescriptions.join(', '));
				}
			} else {
				// No ports in context - show binding count
				if (bindingsOnInterface.length > 0) {
					descriptionItems.push(services_bindingCount({ count: bindingsOnInterface.length }));
				}
			}

			return descriptionItems.join(' · ');
		},
		getIcon: (service: Service) => {
			return serviceDefinitions.getIconComponent(service.service_definition);
		},
		getIconColor: (service: Service) =>
			serviceDefinitions.getColorHelper(service.service_definition).icon,
		compactHides: ['category', 'virtualization'] satisfies ServiceTagRole[],
		getTags: (service: Service) => {
			let tags: TagProps[] = [];

			const category = serviceDefinitions.getCategory(service.service_definition);
			if (category) {
				tags.push({
					label: serviceCategories.getName(category),
					color: serviceCategories.getColorString(category),
					role: 'category' satisfies ServiceTagRole
				});
			}

			if (service.virtualization_metadata) {
				tags.push({
					label: service.virtualization_metadata.type,
					color: concepts.getColorHelper('Virtualization').color,
					role: 'virtualization' satisfies ServiceTagRole
				});
			}

			return tags;
		},
		getTagPickerProps: (service: Service, context: ServiceDisplayContext) => {
			if (!context.showEntityTagPicker) return null;
			return {
				selectedTagIds: service.tags,
				entityId: service.id,
				entityType: 'Service' as const,
				availableTags: context.entityTags,
				allowCreate: context.allowTagCreate
			};
		},
		getCategory: () => null
	};
</script>

<script lang="ts">
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { DisplayTagContext, EntityDisplayComponent } from '../types';
	import type { Service } from '$lib/features/services/types/base';
	import type { TagProps } from '$lib/shared/components/data/types';
	import { formatPort } from '$lib/shared/utils/formatting';

	interface Props {
		item: Service;
		context: ServiceDisplayContext;
	}

	let { item, context }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={ServiceDisplay} />
