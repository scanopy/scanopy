<script lang="ts" module>
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import type { TypedTypeMetadata, CredentialTypeMetadata } from '$lib/shared/stores/metadata';
	import { getScopeTagProps } from '$lib/features/credentials/types/base';

	export type CredentialTypeOption = TypedTypeMetadata<CredentialTypeMetadata>;

	export const CredentialTypeDisplay: EntityDisplayComponent<CredentialTypeOption, object> = {
		getId: (item) => item.id,
		getLabel: (item) => item.name ?? item.id,
		getDescription: (item) => item.description ?? '',
		getIcon: (item) => credentialTypes.getIconComponent(item.id),
		getIconColor: (item) => credentialTypes.getColorHelper(item.id).icon,
		getCategory: (item) => item.category ?? null,
		getTags: (item) => (item.metadata?.scope_models ?? []).map((s: string) => getScopeTagProps(s))
	};
</script>

<script lang="ts">
	import type { EntityDisplayComponent } from '../types';
	import ListSelectItem from '../ListSelectItem.svelte';

	interface Props {
		item: CredentialTypeOption;
		context?: object;
	}

	let { item, context = {} }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={CredentialTypeDisplay} />
