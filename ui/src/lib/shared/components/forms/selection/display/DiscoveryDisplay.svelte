<script lang="ts" module>
	/**
	 * A scan configuration or a run, as the popover on a discovery chip shows it. A run carries
	 * how it ended; a configuration has no outcome, so it shows none.
	 */
	export const DiscoveryDisplay: EntityDisplayComponent<Discovery, object> = {
		getId: (discovery) => discovery.id,
		getLabel: (discovery) => discovery.name,
		getDescription: (discovery) => formatTimestamp(discovery.created_at),
		getIcon: () => entities.getIconComponent('Discovery'),
		getIconColor: () => entities.getColorHelper('Discovery').icon,
		getTags: (discovery) => {
			const tag = runOutcomeTag(
				discovery.run_type.type === 'Historical' ? discovery.run_type.results : null,
				{ includeCompleted: true }
			);
			return tag ? [tag] : [];
		},
		getCategory: () => null
	};
</script>

<script lang="ts">
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { EntityDisplayComponent } from '../types';
	import type { Discovery } from '$lib/features/discovery/types/base';
	import { entities } from '$lib/shared/stores/metadata';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import { runOutcomeTag } from '$lib/features/discovery/utils/outcome';

	interface Props {
		item: Discovery;
		context?: object;
	}

	let { item, context = {} }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={DiscoveryDisplay} />
