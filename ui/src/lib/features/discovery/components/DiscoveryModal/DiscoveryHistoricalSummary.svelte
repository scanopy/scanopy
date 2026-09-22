<script lang="ts">
	import ProgressTrack from '$lib/shared/components/data/ProgressTrack.svelte';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import InfoRow from '$lib/shared/components/data/InfoRow.svelte';
	import InlineSuccess from '$lib/shared/components/feedback/InlineSuccess.svelte';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';
	import InlineDanger from '$lib/shared/components/feedback/InlineDanger.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import type { DiscoveryUpdatePayload } from '../../types/api';
	import { formatDuration, formatTimestamp } from '$lib/shared/utils/formatting';
	import { useSubnetsQuery, getSubnetById } from '$lib/features/subnets/queries';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import scanSettingsFields from '$lib/data/scan-settings.json';
	import type { FieldDefinition } from '$lib/shared/stores/metadata';
	import {
		discovery_runDetails,
		discovery_hostNamingFallback,
		discovery_scanSettings,
		discovery_defaultSettings,
		discovery_bestService,
		discovery_scanModeFull,
		discovery_scanModeLight,
		discovery_subnetsScanned,
		discovery_allInterfacedSubnets,
		discovery_dockerScanDetails,
		discovery_scanMode,
		discovery_scanTuning,
		discovery_selfReportDetails,
		discovery_rescanDetails,
		discovery_portsVerified,
		common_duration,
		common_host,
		common_ipAddresses,
		common_unknownEntity,
		common_finished,
		common_hostId,
		common_ipAddress,
		common_progress,
		common_started
	} from '$lib/paraglide/messages';

	interface Props {
		payload: DiscoveryUpdatePayload;
	}

	let { payload }: Props = $props();

	// TanStack Query for subnets
	const subnetsQuery = useSubnetsQuery();
	let subnetsData = $derived(subnetsQuery.data ?? []);
	// A rescan names its target host by id. The warnings used to be resolved here too; they have
	// their own tab now, and it holds the query for the devices they name.
	let neededHostIds = $derived(
		payload.discovery_type.type === 'Rescan' ? [payload.discovery_type.target_host_id] : []
	);
	const hostsQuery = useHostsByIds(() => neededHostIds);
	let hostsData = $derived(hostsQuery.data ?? []);

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
	const fields = scanSettingsFields as FieldDefinition[];

	// Get non-default scan settings as label/value pairs. A rescan carries the
	// narrower RescanSettings, whose fields are a subset of the same definitions,
	// so it reports the ones it does have rather than showing nothing.
	let nonDefaultSettings = $derived.by(() => {
		const settings =
			payload.discovery_type.type === 'Unified'
				? payload.discovery_type.scan_settings
				: payload.discovery_type.type === 'Rescan'
					? payload.discovery_type.settings
					: null;
		if (!settings) {
			return [];
		}
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

	// Hoisted: narrowing on `payload.discovery_type` doesn't survive into the
	// nested callback, and `ports` is optional in the generated type.
	let rescan = $derived(payload.discovery_type.type === 'Rescan' ? payload.discovery_type : null);
	let rescanHostName = $derived.by(() => {
		if (!rescan) return null;
		const host = hostsData.find((h) => h.id === rescan.target_host_id);
		return host ? hostDisplayName(host) : null;
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
	<!-- Status Banner: the phase, and only the phase. Why a run ended the way it did, and the
	     error that came with it, belong to the Issues tab, which is where a reader goes when
	     something went wrong. -->

	{#if payload.phase === 'Complete'}
		<InlineSuccess title={payload.phase} />
	{:else if payload.phase === 'Failed'}
		<InlineDanger title={payload.phase} />
	{:else if payload.phase === 'Cancelled'}
		<InlineWarning title={payload.phase} />
	{:else}
		<InlineInfo title={payload.phase} />
	{/if}

	<!-- Run Details -->
	<InfoCard title={discovery_runDetails()}>
		{#if payload.progress !== undefined}
			<InfoRow label={common_progress()}>
				<div class="flex items-center gap-2">
					<span>{payload.progress}%</span>
					<ProgressTrack progress={payload.progress} class="w-24" />
				</div>
			</InfoRow>
		{/if}
		{#if duration}
			<InfoRow label={common_duration()}>{duration}</InfoRow>
		{/if}
		{#if payload.started_at}
			<InfoRow label={common_started()}>{formatTimestamp(payload.started_at)}</InfoRow>
		{/if}
		{#if payload.finished_at}
			<InfoRow label={common_finished()}>{formatTimestamp(payload.finished_at)}</InfoRow>
		{/if}
		{#if payload.discovery_type.type === 'Unified' && payload.discovery_type.scan_settings}
			<InfoRow label={discovery_scanMode()}>
				{payload.discovery_type.scan_settings.is_full_scan
					? discovery_scanModeFull()
					: discovery_scanModeLight()}
			</InfoRow>
		{/if}
	</InfoCard>

	<!-- A rescan names its target directly, so there are no subnets to resolve -->
	{#if rescan}
		<InfoCard title={discovery_rescanDetails()}>
			<InfoRow label={common_host()}>
				{rescanHostName ?? common_unknownEntity({ entity: common_host() })}
			</InfoRow>
			<InfoRow label={common_ipAddresses()}>{rescan.ips.join(', ')}</InfoRow>
			<InfoRow label={discovery_portsVerified()}>{(rescan.ports ?? []).length}</InfoRow>
			{#if nonDefaultSettings.length > 0}
				{#each nonDefaultSettings as setting (setting.label)}
					<InfoRow label={setting.label}>{setting.value}</InfoRow>
				{/each}
			{:else}
				<InfoRow label={discovery_scanTuning()}>{discovery_defaultSettings()}</InfoRow>
			{/if}
		</InfoCard>

		<!-- Settings for Unified -->
	{:else if payload.discovery_type.type === 'Unified'}
		<InfoCard title={discovery_scanSettings()}>
			<InfoRow label={discovery_subnetsScanned()}>
				{#if payload.discovery_type.subnet_ids === null}
					{discovery_allInterfacedSubnets()}
				{:else}
					{payload.discovery_type.subnet_ids.map((s) => getSubnetName(s)).join(', ')}
				{/if}
			</InfoRow>
			<InfoRow label={discovery_hostNamingFallback()}>
				{hostNamingLabel}
			</InfoRow>
			{#if nonDefaultSettings.length > 0}
				{#each nonDefaultSettings as setting (setting.label)}
					<InfoRow label={setting.label}>{setting.value}</InfoRow>
				{/each}
			{:else}
				<InfoRow label={discovery_scanTuning()}>{discovery_defaultSettings()}</InfoRow>
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
		<InfoCard title={discovery_dockerScanDetails()}>
			<InfoRow label={common_hostId()} mono>{payload.discovery_type.host_id}</InfoRow>
		</InfoCard>
	{:else if payload.discovery_type.type === 'SelfReport'}
		<InfoCard title={discovery_selfReportDetails()}>
			<InfoRow label={common_hostId()} mono>{payload.discovery_type.host_id}</InfoRow>
		</InfoCard>
	{/if}
</div>
