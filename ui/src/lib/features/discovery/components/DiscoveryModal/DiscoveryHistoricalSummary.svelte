<script lang="ts">
	import ProgressTrack from '$lib/shared/components/data/ProgressTrack.svelte';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import InfoRow from '$lib/shared/components/data/InfoRow.svelte';
	import InlineSuccess from '$lib/shared/components/feedback/InlineSuccess.svelte';
	import InlineDanger from '$lib/shared/components/feedback/InlineDanger.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import type { DiscoveryUpdatePayload } from '../../types/api';
	import { formatDuration, formatTimestamp } from '$lib/shared/utils/formatting';
	import { useSubnetsQuery, getSubnetById } from '$lib/features/subnets/queries';
	import scanSettingsFields from '$lib/data/scan-settings.json';
	import {
		discovery_runDetails,
		discovery_dockerScanning,
		discovery_hostNamingFallback,
		discovery_scanSettings,
		discovery_defaultSettings,
		discovery_bestService,
		discovery_scanModeFull,
		discovery_scanModeLight,
		discovery_subnetsScanned,
		discovery_allInterfacedSubnets,
		common_ipAddress,
		common_enabled,
		common_disabled
	} from '$lib/paraglide/messages';

	interface Props {
		payload: DiscoveryUpdatePayload;
	}

	let { payload }: Props = $props();

	// TanStack Query for subnets
	const subnetsQuery = useSubnetsQuery();
	let subnetsData = $derived(subnetsQuery.data ?? []);

	let duration = $derived(
		payload.started_at && payload.finished_at
			? formatDuration(payload.started_at, payload.finished_at)
			: null
	);

	// Helper to get subnet name by ID
	function getSubnetName(subnetId: string): string {
		const subnet = getSubnetById(subnetsData, subnetId);
		return subnet?.name || 'Unknown Subnet';
	}

	// Scan settings field metadata for label lookup
	interface FieldDef {
		id: string;
		label: string;
		default_value: string;
		field_type: string;
	}

	const fields = scanSettingsFields as FieldDef[];

	// Get non-default scan settings as label/value pairs
	let nonDefaultSettings = $derived.by(() => {
		if (payload.discovery_type.type !== 'Unified' || !payload.discovery_type.scan_settings) {
			return [];
		}
		const settings = payload.discovery_type.scan_settings;
		const result: { label: string; value: string }[] = [];
		for (const field of fields) {
			const val = settings[field.id as keyof typeof settings];
			if (val !== undefined && val !== null && String(val) !== field.default_value) {
				result.push({
					label: field.label,
					value: field.field_type === 'boolean' ? (val ? 'Yes' : 'No') : String(val)
				});
			}
		}
		return result;
	});

	let hostNamingLabel = $derived(
		payload.discovery_type.type === 'Unified'
			? payload.discovery_type.host_naming_fallback === 'Ip'
				? common_ipAddress()
				: discovery_bestService()
			: ''
	);
</script>

<div class="space-y-4">
	<!-- Status Banner -->
	{#if payload.phase === 'Complete'}
		<InlineSuccess title={payload.phase} />
	{:else if payload.phase === 'Failed'}
		<InlineDanger title={payload.phase} body={payload.error ?? null} />
	{:else if payload.phase === 'Cancelled'}
		<InlineWarning title={payload.phase} />
	{:else}
		<InlineInfo title={payload.phase} />
	{/if}

	<!-- Run Details -->
	<InfoCard title={discovery_runDetails()}>
		{#if payload.progress !== undefined}
			<InfoRow label="Progress">
				<div class="flex items-center gap-2">
					<span>{payload.progress}%</span>
					<ProgressTrack progress={payload.progress} class="w-24" />
				</div>
			</InfoRow>
		{/if}
		{#if duration}
			<InfoRow label="Duration">{duration}</InfoRow>
		{/if}
		{#if payload.started_at}
			<InfoRow label="Started">{formatTimestamp(payload.started_at)}</InfoRow>
		{/if}
		{#if payload.finished_at}
			<InfoRow label="Finished">{formatTimestamp(payload.finished_at)}</InfoRow>
		{/if}
		{#if payload.discovery_type.type === 'Unified' && payload.discovery_type.scan_settings}
			<InfoRow label="Scan Mode">
				{payload.discovery_type.scan_settings.is_full_scan
					? discovery_scanModeFull()
					: discovery_scanModeLight()}
			</InfoRow>
		{/if}
	</InfoCard>

	<!-- Settings for Unified -->
	{#if payload.discovery_type.type === 'Unified'}
		<InfoCard title={discovery_scanSettings()}>
			<InfoRow label={discovery_subnetsScanned()}>
				{#if payload.discovery_type.subnet_ids === null}
					{discovery_allInterfacedSubnets()}
				{:else}
					{payload.discovery_type.subnet_ids.map((s) => getSubnetName(s)).join(', ')}
				{/if}
			</InfoRow>
			<InfoRow label={discovery_dockerScanning()}>
				{payload.discovery_type.scan_local_docker_socket ? common_enabled() : common_disabled()}
			</InfoRow>
			<InfoRow label={discovery_hostNamingFallback()}>
				{hostNamingLabel}
			</InfoRow>
			{#if nonDefaultSettings.length > 0}
				{#each nonDefaultSettings as setting (setting.label)}
					<InfoRow label={setting.label}>{setting.value}</InfoRow>
				{/each}
			{:else}
				<InfoRow label="Scan Tuning">{discovery_defaultSettings()}</InfoRow>
			{/if}
		</InfoCard>

		<!-- Settings for Network -->
	{:else if payload.discovery_type.type === 'Network'}
		<InfoCard title={discovery_scanSettings()}>
			<InfoRow label={discovery_subnetsScanned()}>
				{#if payload.discovery_type.subnet_ids === null}
					{discovery_allInterfacedSubnets()}
				{:else}
					{payload.discovery_type.subnet_ids.map((s) => getSubnetName(s)).join(', ')}
				{/if}
			</InfoRow>
		</InfoCard>

		<!-- Docker/SelfReport host_id card -->
	{:else if payload.discovery_type.type === 'Docker'}
		<InfoCard title="Docker Scan Details">
			<InfoRow label="Host ID" mono>{payload.discovery_type.host_id}</InfoRow>
		</InfoCard>
	{:else if payload.discovery_type.type === 'SelfReport'}
		<InfoCard title="Self Report Details">
			<InfoRow label="Host ID" mono>{payload.discovery_type.host_id}</InfoRow>
		</InfoCard>
	{/if}
</div>
