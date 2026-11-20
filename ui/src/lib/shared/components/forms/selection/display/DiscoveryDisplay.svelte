<script lang="ts" module>
	/** With the daemon's name, a scan is described by the daemon that runs it, as a scan picker
	 *  wants. Without it, by when it was created. */
	export interface DiscoveryDisplayContext {
		daemonName?: string;
	}

	/**
	 * A scan configuration or a run, as the popover on a discovery chip or a scan picker shows it.
	 * A run carries how it ended; a configuration has no outcome, so it shows none.
	 */
	export const DiscoveryDisplay: EntityDisplayComponent<Discovery, DiscoveryDisplayContext> = {
		getId: (discovery) => discovery.id,
		getLabel: (discovery) => discovery.name,
		getDescription: (discovery, context) =>
			context?.daemonName ?? formatTimestamp(discovery.created_at),
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
	import ListSelectItem from '#lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { EntityDisplayComponent } from '../types';
	import type { Discovery } from '#lib/features/discovery/types/base.ts';
	import { entities } from '#lib/shared/stores/metadata.ts';
	import { formatTimestamp } from '#lib/shared/utils/formatting.ts';
	import { runOutcomeTag } from '#lib/features/discovery/utils/outcome.ts';

	interface Props {
		item: Discovery;
		context?: DiscoveryDisplayContext;
	}

	let { item, context = {} }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={DiscoveryDisplay} />
