<script lang="ts" context="module">
	import { entities } from '$lib/shared/stores/metadata';
	import { toColor } from '$lib/shared/utils/styling';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import type { Discovery } from '$lib/features/discovery/types/base';

	export const HomeDiscoveryDisplay: EntityDisplayComponent<Discovery, Record<string, never>> = {
		getId: (discovery) => discovery.id,
		getLabel: (discovery) => discovery.name,
		getDescription: (discovery) => formatTimestamp(discovery.created_at),
		getIcon: () => entities.getIconComponent('Discovery'),
		getIconColor: () => entities.getColorHelper('Discovery').icon,
		getTags: (discovery) => {
			const phase =
				discovery.run_type.type === 'Historical' && discovery.run_type.results
					? (discovery.run_type.results.phase ?? null)
					: null;

			if (!phase) return [];

			switch (phase) {
				case 'Complete':
					return [{ label: 'Complete', color: toColor('green') }];
				case 'Failed':
					return [{ label: 'Failed', color: toColor('red') }];
				case 'Cancelled':
					return [{ label: 'Cancelled', color: toColor('yellow') }];
				default:
					return [{ label: phase, color: toColor('blue') }];
			}
		}
	};
</script>

<script lang="ts">
	import type { EntityDisplayComponent } from '$lib/shared/components/forms/selection/types';
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';

	export let item: Discovery;
	export let context: Record<string, never> = {} as Record<string, never>;
</script>

<ListSelectItem {item} {context} displayComponent={HomeDiscoveryDisplay} />
