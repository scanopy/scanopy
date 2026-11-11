<!--
	What to do about a host Scanopy knows only from another device's report: one an LLDP/CDP
	neighbour advertised, or a MAC a switch learned on one of its ports.

	The same layout as resolving an assumed subnet range (`ConfirmEntityModal`), so an unconfirmed
	host and an unconfirmed subnet are settled the same way. The actions hand off to the flows that
	already own them: the host editor for labelling, the merge modal for a merge.
-->
<script lang="ts">
	import { Check, EyeOff, Replace } from '@lucide/svelte';

	import ConfirmEntityModal, {
		type ConfirmAction
	} from '#lib/shared/components/confirmation/ConfirmEntityModal.svelte';
	import CopyableValue from '#lib/shared/components/data/CopyableValue.svelte';
	import { unconfirmedReason, unconfirmedVia } from '#lib/shared/utils/confirmation.ts';
	import {
		entityFieldLabel,
		entityFields,
		type EntityType
	} from '#lib/shared/entity-fields/index.ts';
	import type { Host } from '../types/base';
	import type { UnconfirmedEvidence } from '../unconfirmed-evidence';
	import {
		hosts_confirmAndLabel,
		hosts_confirmAndLabelDetail,
		common_dismiss,
		hosts_dismissUnconfirmedDetail,
		hosts_mergeUnconfirmed,
		hosts_mergeUnconfirmedDetail,
		hosts_resolveUnconfirmed
	} from '#lib/paraglide/messages.js';

	interface Props {
		host: Host | null;
		/** What the modal shows for `host`, built by `unconfirmedEvidence`. */
		known: UnconfirmedEvidence | null;
		isOpen?: boolean;
		/** Confirms the host, then opens the host editor on it. Owned by the caller. */
		onConfirmAndLabel: (host: Host) => Promise<void> | void;
		/** Opens the merge modal with this host as the one merged away. */
		onMerge: (host: Host) => void;
		onDismiss: (host: Host) => Promise<void> | void;
		onClose: () => void;
	}

	let {
		host,
		known,
		isOpen = false,
		onConfirmAndLabel,
		onMerge,
		onDismiss,
		onClose
	}: Props = $props();

	let via = $derived(host ? unconfirmedVia(host) : null);

	/** A registry field's label, so each identifier is named as the rest of the app names it. */
	function label(type: EntityType, id: string): string {
		const field = entityFields(type).find((f) => f.id === id);
		return field ? entityFieldLabel(type, field) : id;
	}

	/** The identifiers the host is known by, each under its field's name and copyable. */
	let identifiers = $derived.by(() => {
		if (!known) return [];
		const rows: { label: string; values: string[] }[] = [];
		if (known.chassisId) {
			rows.push({ label: label('Host', 'chassis_id'), values: [known.chassisId] });
		}
		if (known.sysName) {
			rows.push({ label: label('Host', 'sys_name'), values: [known.sysName] });
		}
		if (known.macs.length > 0) {
			rows.push({ label: label('Interface', 'mac_address'), values: known.macs });
		}
		return rows;
	});

	let actions = $derived.by<ConfirmAction[]>(() => {
		if (!host) return [];
		const current = host;
		return [
			{
				id: 'confirm',
				title: hosts_confirmAndLabel(),
				description: hosts_confirmAndLabelDetail(),
				icon: Check,
				color: 'Green',
				run: () => void onConfirmAndLabel(current)
			},
			{
				id: 'merge',
				title: hosts_mergeUnconfirmed(),
				description: hosts_mergeUnconfirmedDetail(),
				icon: Replace,
				color: 'Violet',
				run: () => onMerge(current)
			},
			{
				id: 'dismiss',
				title: common_dismiss(),
				description: hosts_dismissUnconfirmedDetail(),
				icon: EyeOff,
				color: 'Gray',
				run: () => void onDismiss(current)
			}
		];
	});
</script>

<ConfirmEntityModal
	name="unconfirmed-host"
	entityId={host?.id}
	title={hosts_resolveUnconfirmed()}
	intro={via ? unconfirmedReason(via) : ''}
	{actions}
	{isOpen}
	{onClose}
	reportedByHostIds={known?.reportedByHostIds ?? []}
	sourceDiscoveryId={known?.sourceDiscoveryId ?? null}
>
	{#snippet evidence()}
		{#each identifiers as row (row.label)}
			<div class="flex flex-wrap items-center gap-2">
				<span class="text-secondary">{row.label}</span>
				{#each row.values as value (value)}
					<CopyableValue {value} variant="inline" />
				{/each}
			</div>
		{/each}
	{/snippet}
</ConfirmEntityModal>
