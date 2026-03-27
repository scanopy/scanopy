<script lang="ts" context="module">
	import { entities, credentialTypes } from '$lib/shared/stores/metadata';
	import { entityRef } from '$lib/shared/components/data/types';
	import type { Credential } from '$lib/features/credentials/types/base';
	import { getCredentialTypeId } from '$lib/features/credentials/types/base';

	export interface NetworkDisplayContext {
		credentials?: Credential[];
	}

	export const NetworkDisplay: EntityDisplayComponent<Network, NetworkDisplayContext> = {
		getId: (network: Network) => network.id,
		getLabel: (network: Network) => network.name,
		getDescription: (network: Network, context: NetworkDisplayContext) => {
			const credIds = network.credential_ids ?? [];
			if (credIds.length === 0) return 'No credentials';
			const creds = context?.credentials ?? [];
			const matched = credIds.map((id) => creds.find((c) => c.id === id)).filter(Boolean);
			if (matched.length > 0) return '';
			return `${credIds.length} credential(s)`;
		},
		getIcon: () => entities.getIconComponent('Network'),
		getIconColor: () => entities.getColorHelper('Network').icon,
		getTags: (network: Network, context: NetworkDisplayContext) => {
			const credIds = network.credential_ids ?? [];
			if (credIds.length === 0) return [];
			const creds = context?.credentials ?? [];
			return credIds
				.map((id) => creds.find((c) => c.id === id))
				.filter(Boolean)
				.map((cred) => ({
					label: cred!.name,
					color: credentialTypes.getColorHelper(getCredentialTypeId(cred!)).color,
					entityRef: entityRef('Credential', cred!.id, cred!)
				}));
		},
		getCategory: () => null
	};
</script>

<script lang="ts">
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { EntityDisplayComponent } from '../types';
	import type { Network } from '$lib/features/networks/types';

	export let item: Network;
	export let context: NetworkDisplayContext = {};
</script>

<ListSelectItem {item} {context} displayComponent={NetworkDisplay} />
