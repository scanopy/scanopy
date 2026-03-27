<script lang="ts" module>
	export const CredentialDisplay: EntityDisplayComponent<Credential, object> = {
		getId: (credential) => credential.id,
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
			return [
				{
					label: credentialTypes.getName(typeId),
					color: credentialTypes.getColorHelper(typeId).color
				}
			];
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
		context?: object;
	}

	let { item, context = {} }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={CredentialDisplay} />
