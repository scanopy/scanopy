<script lang="ts">
	import type { Interface } from '#lib/features/hosts/types/base.ts';
	import {
		getAdminStatusLabels,
		getOperStatusLabels
	} from '#lib/features/credentials/types/base.ts';
	import CollapsibleCard from '#lib/shared/components/data/CollapsibleCard.svelte';
	import InfoRow from '#lib/shared/components/data/InfoRow.svelte';
	import Tag from '#lib/shared/components/data/Tag.svelte';
	import { entities, ifOperStatuses } from '#lib/shared/stores/metadata.ts';
	import {
		collectionLookup,
		entityFieldLabel,
		entityFields,
		formatFieldValue,
		resolveField,
		type EntityField
	} from '#lib/shared/entity-fields/index.ts';
	import InterfaceRelationRows, {
		type InterfaceRelations,
		type VlanInfo
	} from './InterfaceRelationRows.svelte';

	// The card's registry rows are read off the interface itself, so no related entity is needed.
	const NO_RELATIONS = collectionLookup({});
	import type { Color } from '#lib/shared/utils/styling.ts';
	import {
		common_details,
		common_status,
		common_unknown,
		hosts_interfaces_adminStatus,
		hosts_interfaces_nativeVlan,
		hosts_interfaces_operStatus
	} from '#lib/paraglide/messages.js';

	interface Props extends InterfaceRelations {
		iface: Interface;
		nativeVlan?: VlanInfo | null;
		showStatus?: boolean;
	}

	let {
		iface,
		linkedIpAddress = null,
		linkedSubnet = null,
		neighbours = [],
		nativeVlan = null,
		taggedVlans = [],
		showStatus = true
	}: Props = $props();

	// A status nothing read is unknown, the same as one this build has no label for.
	let adminStatusLabel = $derived(
		(iface.admin_status && getAdminStatusLabels()[iface.admin_status]) || common_unknown()
	);
	let operStatusLabel = $derived(
		(iface.oper_status && getOperStatusLabels()[iface.oper_status]) || common_unknown()
	);

	// The status's own colour, the one the L2 "By status" chip and the port's dot use.
	let operStatusColor: Color = $derived(ifOperStatuses.getColorString(iface.oper_status ?? null));

	// The plain facts come from the entity field registry: every single-valued Interface field read
	// off the interface itself, except what this card draws in its own way (the name heads it, the
	// status card shows both statuses, and the linked IP, VLANs and neighbours get rows below).
	const DRAWN_ELSEWHERE = new Set(['display_name', 'oper_status', 'admin_status']);
	const detailFields = entityFields('Interface').filter(
		(field) => field.path.length === 0 && !field.many && !DRAWN_ELSEWHERE.has(field.id)
	);
	function detailText(field: EntityField): string {
		return resolveField(field, iface, NO_RELATIONS)
			.map((value) => formatFieldValue(field, value))
			.filter(Boolean)
			.join(', ');
	}

	// The MACs the port's forwarding table learned: the devices behind it, including ones no scan
	// reached (each of which is an unconfirmed host until someone confirms it).
	const learnedField = entityFields('Interface').find((field) => field.id === 'fdb_macs');
	let learnedMacs = $derived(
		learnedField
			? resolveField(learnedField, iface, NO_RELATIONS).map((value) => String(value))
			: []
	);

	let statusExpanded = $state(true);
	let detailsExpanded = $state(true);
</script>

{#if showStatus}
	<CollapsibleCard title={common_status()} bind:expanded={statusExpanded}>
		<InfoRow label={hosts_interfaces_adminStatus()}>{adminStatusLabel}</InfoRow>
		<InfoRow label={hosts_interfaces_operStatus()}>
			<Tag label={operStatusLabel} color={operStatusColor} />
		</InfoRow>
	</CollapsibleCard>
{/if}

<CollapsibleCard title={common_details()} bind:expanded={detailsExpanded}>
	{#each detailFields as field (field.id)}
		<InfoRow label={entityFieldLabel('Interface', field)} mono={field.format.type === 'Identifier'}>
			{detailText(field) || '-'}
		</InfoRow>
	{/each}

	{#if nativeVlan}
		<InfoRow label={hosts_interfaces_nativeVlan()}>
			<Tag
				label="VLAN {nativeVlan.vlan_number} ({nativeVlan.name})"
				color={entities.getColorHelper('Vlan').color}
			/>
		</InfoRow>
	{/if}

	{#if learnedField && learnedMacs.length > 0}
		<InfoRow label={entityFieldLabel('Interface', learnedField)}>
			<div class="flex flex-wrap gap-1">
				{#each learnedMacs as mac (mac)}
					<Tag label={mac} color={null} />
				{/each}
			</div>
		</InfoRow>
	{/if}

	<InterfaceRelationRows {linkedIpAddress} {linkedSubnet} {neighbours} {taggedVlans} />
</CollapsibleCard>
