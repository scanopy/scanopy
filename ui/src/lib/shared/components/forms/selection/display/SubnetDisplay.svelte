<script lang="ts" module>
	import { isContainerSubnet } from '$lib/features/subnets/queries';

	export type SubnetTagRole = 'subnetType';

	export interface SubnetDisplayContext extends DisplayTagContext<SubnetTagRole> {
		showEntityTagPicker?: boolean;
		tagPickerDisabled?: boolean;
		entityTags?: import('$lib/features/tags/types/base').Tag[];
		showEditableEntityDescription?: boolean;
		entityDescription?: string | null;
		entityDescriptionDisabled?: boolean;
		onEntityDescriptionSave?: (value: string | null) => void;
	}

	export const SubnetDisplay: EntityDisplayComponent<Subnet, SubnetDisplayContext> = {
		getId: (subnet: Subnet) => subnet.id,
		getLabel: (subnet: Subnet) => subnet.name,
		getDescription: (subnet: Subnet) => {
			if (isContainerSubnet(subnet)) return '';
			return subnet.name == subnet.cidr ? '' : subnet.cidr;
		},
		getIcon: (subnet: Subnet) => subnetTypes.getIconComponent(subnet.subnet_type),
		getIconColor: (subnet: Subnet) => subnetTypes.getColorHelper(subnet.subnet_type).icon,
		compactHides: ['subnetType'] satisfies SubnetTagRole[],
		getTags: (subnet: Subnet) => {
			if (!subnetTypes.getMetadata(subnet.subnet_type).show_label) return [];
			return [
				{
					...subnetTypes.getTag(subnet.subnet_type),
					role: 'subnetType' satisfies SubnetTagRole
				}
			];
		},
		getTagPickerProps: (subnet: Subnet, context: SubnetDisplayContext) => {
			if (!context.showEntityTagPicker) return null;
			return {
				selectedTagIds: subnet.tags,
				entityId: subnet.id,
				entityType: 'Subnet' as const,
				availableTags: context.entityTags
			};
		},
		getCategory: () => null
	};
</script>

<script lang="ts">
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { DisplayTagContext, EntityDisplayComponent } from '../types';
	import { subnetTypes } from '$lib/shared/stores/metadata';
	import type { Subnet } from '$lib/features/subnets/types/base';

	interface Props {
		item: Subnet;
		context?: SubnetDisplayContext;
	}

	let { item, context = {} }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={SubnetDisplay} />
