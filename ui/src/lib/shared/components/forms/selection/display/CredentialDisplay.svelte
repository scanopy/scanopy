<script lang="ts" module>
	/** A non-null `disabledReason` renders the option disabled with that tooltip. */
	export type CredentialDisplayContext = { disabledReason?: string | null };

	export const CredentialDisplay: EntityDisplayComponent<Credential, CredentialDisplayContext> = {
		getId: (credential) => credential.id,
		getDisabled: (_credential, context) => !!context?.disabledReason,
		getDisabledReason: (_credential, context) => context?.disabledReason ?? null,
		getLabel: (credential) => credential.name,
		getDescription: (credential) => getCredentialDescription(credential),
		getIcon: (credential) => {
			const typeId = credential.credential_type.type;
			return credentialTypes.getIconComponent(typeId);
		},
		getIconColor: (credential) => {
			const typeId = credential.credential_type.type;
			return credentialTypes.getColorHelper(typeId).icon;
		},
		getTags: (credential) => {
			const typeId = credential.credential_type.type;
			return [credentialTypes.getTag(typeId)];
		},
		getCategory: (credential) => {
			const typeId = credential.credential_type.type;
			return credentialTypes.getItem(typeId)?.category ?? null;
		}
	};
</script>

<script lang="ts">
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { EntityDisplayComponent } from '../types';
	import { type Credential, getCredentialDescription } from '$lib/features/credentials/types/base';
	import { credentialTypes } from '$lib/shared/stores/metadata';

	interface Props {
		item: Credential;
		context?: CredentialDisplayContext;
	}

	let { item, context = {} }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={CredentialDisplay} />
