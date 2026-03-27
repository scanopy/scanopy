<script lang="ts">
	import { Edit, Trash2 } from 'lucide-svelte';
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import { entities, permissions, credentialTypes, subnetTypes } from '$lib/shared/stores/metadata';
	import type { Network } from '../types';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import { entityRef } from '$lib/shared/components/data/types';
	import {
		common_daemons,
		common_delete,
		common_edit,
		common_subnets,
		common_tags
	} from '$lib/paraglide/messages';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { getCredentialTypeId } from '$lib/features/credentials/types/base';
	import { uuidv4Sentinel } from '$lib/shared/utils/formatting';
	import { toColor } from '$lib/shared/utils/styling';
	import { useHostsQuery } from '$lib/features/hosts/queries';

	interface Props {
		network: Network;
		onDelete?: (network: Network) => void;
		onEdit?: (network: Network) => void;
		viewMode: 'card' | 'list';
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	}

	let {
		network,
		onDelete = () => {},
		onEdit = () => {},
		viewMode,
		selected,
		onSelectionChange = () => {}
	}: Props = $props();

	// TanStack Query hooks
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	const daemonsQuery = useDaemonsQuery();
	const subnetsQuery = useSubnetsQuery();
	const hostsQuery = useHostsQuery({ limit: 0 });

	// Derived data from queries
	let daemonsData = $derived(daemonsQuery.data ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);
	let hostsData = $derived(hostsQuery.data?.items ?? []);

	let networkDaemons = $derived(daemonsData.filter((d) => d.network_id == network.id));
	let networkSubnets = $derived(
		subnetsData.filter(
			(s) =>
				s.network_id == network.id && !subnetTypes.getMetadata(s.subnet_type).hide_from_subnet_list
		)
	);

	// Credentials query
	const credentialsQuery = useCredentialsQuery();
	let credentialsData = $derived(credentialsQuery.data ?? []);
	let networkCredentials = $derived(
		(network.credential_ids ?? [])
			.map((id) => credentialsData.find((c) => c.id === id))
			.filter(Boolean)
	);

	let canManageNetworks = $derived(
		(currentUser && permissions.getMetadata(currentUser.permissions).manage_org_entities) || false
	);

	// Build card data
	let cardData = $derived({
		title: network.name,
		iconColor: entities.getColorHelper('Network').icon,
		Icon: entities.getIconComponent('Network'),
		fields: [
			{
				label: common_daemons(),
				value: networkDaemons.map((d) => ({
					id: d.id,
					label: d.name,
					color: entities.getColorHelper('Daemon').color,
					entityRef: entityRef('Daemon', d.id, d, { hosts: hostsData, subnets: subnetsData })
				}))
			},
			{
				label: 'Credentials',
				value:
					networkCredentials.length > 0
						? networkCredentials.map((cred) => ({
								id: cred!.id,
								label: cred!.name,
								color: credentialTypes.getColorHelper(getCredentialTypeId(cred!)).color,
								entityRef: entityRef('Credential', cred!.id, cred!)
							}))
						: [
								{
									id: uuidv4Sentinel,
									label: 'None',
									color: toColor('Gray')
								}
							]
			},
			{
				label: common_subnets(),
				value: networkSubnets.map((s) => ({
					id: s.id,
					label: s.name,
					color: entities.getColorHelper('Subnet').color,
					entityRef: entityRef('Subnet', s.id, s)
				}))
			},
			{ label: common_tags(), snippet: tagsSnippet }
		],

		actions: [
			...(canManageNetworks
				? [
						{
							label: common_delete(),
							icon: Trash2,
							class: 'btn-icon-danger',
							onClick: () => onDelete(network)
						},
						{
							label: common_edit(),
							icon: Edit,
							onClick: () => onEdit(network)
						}
					]
				: [])
		]
	});
</script>

{#snippet tagsSnippet()}
	<div class="flex items-center gap-2">
		<span class="text-secondary text-sm">{common_tags()}:</span>
		<TagPickerInline selectedTagIds={network.tags} entityId={network.id} entityType="Network" />
	</div>
{/snippet}

<GenericCard
	{...cardData}
	{viewMode}
	{selected}
	{onSelectionChange}
	selectable={canManageNetworks}
/>
