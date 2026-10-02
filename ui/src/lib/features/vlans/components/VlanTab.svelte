<script lang="ts">
	import { lastSeenItems } from '$lib/shared/utils/freshness';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import { defineFields, entityRef } from '$lib/shared/components/data/types';
	import { networkItems } from '$lib/features/networks/columns';
	import { entities, entitySources } from '$lib/shared/stores/metadata';
	import { entitySourceItems } from '$lib/shared/utils/entity-source';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { isUserManagedSubnet, useSubnetsQuery } from '$lib/features/subnets/queries';
	import type { Subnet } from '$lib/features/subnets/types/base';
	import { useVlansQuery } from '../queries';
	import type { Vlan, VlanOrderField } from '../types/base';
	import type { components } from '$lib/api/schema';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import {
		common_created,
		common_description,
		common_lastSeen,
		common_name,
		common_network,
		common_noEntityYet,
		common_source,
		common_subnets,
		common_unknownNetwork,
		common_updated,
		common_vlans,
		daemons_installPromptVlans,
		vlans_emptySubtitle,
		vlans_vlanNumber
	} from '$lib/paraglide/messages';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';

	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];

	// No `$props()`: unlike the sibling tabs this one declares no `TabProps`.
	// It is view-only for every permission level, so `isReadOnly` would have
	// nothing to gate.

	// Organization query for onboarding state
	const organizationQuery = useOrganizationQuery();
	let onboarding = $derived((organizationQuery.data?.onboarding ?? []) as OnboardingOperation[]);

	// Queries
	const vlansQuery = useVlansQuery();
	const networksQuery = useNetworksQuery();
	// Shared full-list subnets cache — used to resolve the hydrated `subnet_ids`
	// on each VLAN into names.
	const subnetsQuery = useSubnetsQuery();

	// Derived data
	let vlansData = $derived(vlansQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);
	let subnetsById = $derived(
		new Map((subnetsQuery.data ?? []).filter(isUserManagedSubnet).map((s) => [s.id, s]))
	);
	let isLoading = $derived(vlansQuery.isPending);

	function getSubnets(vlan: Vlan): Subnet[] {
		return (vlan.subnet_ids ?? []).map((id) => subnetsById.get(id)).filter((s): s is Subnet => !!s);
	}

	function getSubnetNames(vlan: Vlan): string[] {
		return getSubnets(vlan).map((s) => s.name);
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('Vlan', {});
	}

	// Define field configuration for the DataTableControls
	// Uses defineFields to ensure all VlanOrderField values are covered
	let vlanFields = $derived(
		defineFields<Vlan, VlanOrderField>(
			{
				// Identity fields: grouping by one would render a header per VLAN.
				vlan_number: {
					label: vlans_vlanNumber(),
					type: 'string',
					searchable: true,
					groupable: false
				},
				name: {
					label: common_name(),
					type: 'string',
					searchable: true,
					groupable: false,
					display: { primary: true, width: 220 }
				},
				created_at: { label: common_created(), type: 'date', display: { hiddenByDefault: true } },
				updated_at: { label: common_updated(), type: 'date', display: { hiddenByDefault: true } }
			},
			[
				{ key: 'description', label: common_description(), type: 'string', searchable: true },
				{
					key: 'source',
					label: common_source(),
					type: 'string',
					filterable: true,
					groupable: true,
					getValue: (vlan) => (vlan.source ? entitySources.getName(vlan.source.type) : null),
					display: {
						hiddenByDefault: true,
						getItems: (vlan) => (vlan.source ? entitySourceItems(vlan.source) : [])
					}
				},
				{
					key: 'network_id',
					label: common_network(),
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					getValue: (item) =>
						networksData.find((n) => n.id == item.network_id)?.name || common_unknownNetwork(),
					display: { getItems: (item) => networkItems(item.network_id, networksData) }
				},
				{
					key: 'subnet_ids',
					label: common_subnets(),
					type: 'array',
					searchable: true,
					filterable: true,
					getValue: getSubnetNames,
					display: {
						getItems: (vlan) =>
							getSubnets(vlan).map((subnet) => ({
								id: subnet.id,
								label: subnet.name,
								color: entities.getColorHelper('Subnet').color,
								entityRef: entityRef('Subnet', subnet.id, subnet)
							}))
					}
				},
				{
					// Not in VlanOrderField, so display-only with client-side sorting.
					key: 'last_seen_at',
					label: common_lastSeen(),
					type: 'date',
					sortable: true,
					display: { recency: true, getItems: lastSeenItems(() => networksData, 'Vlan') }
				}
			]
		)
	);
</script>

<div class="space-y-6">
	<!-- Header: no actions — VLANs are discovery-populated and view-only -->
	<TabHeader title={common_vlans()} />

	{#if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title={daemons_installPromptVlans()} />
	{:else if isLoading}
		<!-- Loading state -->
		<Loading />
	{:else if vlansData.length === 0}
		<!-- Empty state: no CTA, there is nothing to create -->
		<EmptyState
			title={common_noEntityYet({ entity: common_vlans() })}
			subtitle={vlans_emptySubtitle()}
		/>
	{:else}
		<DataControls
			items={vlansData}
			fields={vlanFields}
			storageKey="scanopy-vlans-table-state"
			getItemId={(item) => item.id}
			getIcon={() => ({
				icon: entities.getIconComponent('Vlan'),
				color: entities.getColorHelper('Vlan').icon
			})}
			onCsvExport={handleCsvExport}
			entityLabel={common_vlans()}
		></DataControls>
	{/if}
</div>
