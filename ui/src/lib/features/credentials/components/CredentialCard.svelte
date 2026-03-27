<script lang="ts">
	import { Edit, Trash2 } from 'lucide-svelte';
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import type { Credential } from '../types/base';
	import { getCredentialTypeId, getScopeTagProps } from '../types/base';
	import { entities } from '$lib/shared/stores/metadata';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { entityRef } from '$lib/shared/components/data/types';
	import type { Color } from '$lib/shared/utils/styling';
	import type { CardFieldItem } from '$lib/shared/components/data/types';
	import { permissions } from '$lib/shared/stores/metadata';
	import type { Network } from '$lib/features/networks/types';
	import type { Host } from '$lib/features/hosts/types/base';
	import {
		common_delete,
		common_edit,
		common_hosts,
		common_networks,
		common_notAssigned,
		common_scope,
		common_tags,
		common_notApplicable
	} from '$lib/paraglide/messages';

	let {
		credential,
		assignedNetworks = [],
		assignedHosts = [],
		onEdit = () => {},
		onDelete = () => {},
		viewMode,
		selected,
		onSelectionChange = () => {}
	}: {
		credential: Credential;
		assignedNetworks?: Network[];
		assignedHosts?: Host[];
		onEdit?: (credential: Credential) => void;
		onDelete?: (credential: Credential) => void;
		viewMode: 'card' | 'list';
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	let typeId = $derived(getCredentialTypeId(credential));
	let scopeModels = $derived(credentialTypes.getMetadata(typeId)?.scope_models ?? []);

	let canManage = $derived(
		(currentUser && permissions.getMetadata(currentUser.permissions).manage_org_entities) || false
	);

	let cardData = $derived({
		title: credential.name,
		iconColor: credentialTypes.getColorHelper(typeId).icon,
		Icon: credentialTypes.getIconComponent(typeId),
		fields: [
			{
				label: 'Type',
				value: [
					{
						id: 'type',
						label: credentialTypes.getName(typeId),
						color: credentialTypes.getColorHelper(typeId).color
					}
				]
			},
			{
				label: common_scope(),
				value: scopeModels.map((s) => {
					const props = getScopeTagProps(s);
					return { id: s, ...props } as CardFieldItem;
				})
			},
			{
				label: common_networks(),
				value: scopeModels.includes('Broadcast')
					? assignedNetworks.length > 0
						? assignedNetworks.map((n) => ({
								id: n.id,
								label: n.name,
								color: entities.getColorHelper('Network').color as Color,
								entityRef: entityRef('Network', n.id, n)
							}))
						: common_notAssigned()
					: common_notApplicable()
			},
			{
				label: common_hosts(),
				value: scopeModels.includes('PerHost')
					? assignedHosts.length > 0
						? assignedHosts.map((h) => ({
								id: h.id,
								label: h.name ?? h.id,
								color: entities.getColorHelper('Host').color as Color,
								entityRef: entityRef('Host', h.id, h)
							}))
						: common_notAssigned()
					: common_notApplicable()
			},
			{
				label: common_tags(),
				snippet: tagsSnippet
			}
		],
		actions: [
			...(canManage
				? [
						{
							label: common_delete(),
							icon: Trash2,
							class: 'btn-icon-danger',
							onClick: () => onDelete(credential)
						},
						{
							label: common_edit(),
							icon: Edit,
							onClick: () => onEdit(credential)
						}
					]
				: [])
		]
	});
</script>

{#snippet tagsSnippet()}
	<div class="flex items-center gap-2">
		<span class="text-secondary text-sm">{common_tags()}:</span>
		<TagPickerInline
			selectedTagIds={credential.tags}
			entityId={credential.id}
			entityType="Credential"
		/>
	</div>
{/snippet}

<GenericCard {...cardData} {viewMode} {selected} {onSelectionChange} selectable={canManage} />
