<script lang="ts">
	import scanSettingsFields from '#lib/data/scan-settings.json';
	import type { FieldDefinition } from '#lib/shared/stores/metadata.ts';
	import type { Discovery } from '../../types/base';
	import type { Daemon } from '#lib/features/daemons/types/base.ts';
	import { useSubnetsQuery } from '#lib/features/subnets/queries.ts';
	import DocsHint from '#lib/shared/components/feedback/DocsHint.svelte';
	import InlineWarning from '#lib/shared/components/feedback/InlineWarning.svelte';
	import { formatDurationHuman } from '#lib/shared/utils/formatting.ts';
	import { translateFieldDefinitions } from '#lib/i18n/metadata.ts';
	import {
		discovery_docsScanSettings,
		discovery_docsScanSettingsLinkText,
		discovery_arpScanCutoffWarning,
		discovery_arpScanCutoffWarningSlow
	} from '#lib/paraglide/messages.js';

	interface Props {
		formData: Discovery;
		daemon: Daemon | null;
		readOnly?: boolean;
	}

	let { formData = $bindable(), daemon = null, readOnly = false }: Props = $props();

	const subnetsQuery = useSubnetsQuery();
	let subnetsData = $derived(subnetsQuery.data ?? []);

	// Labels/placeholders/help text resolved via meta_* i18n keys with fixture fallback
	const fields = translateFieldDefinitions(
		'scan_settings',
		null,
		scanSettingsFields as FieldDefinition[]
	);

	// Only include performance-related fields (exclude the Detection group)
	const performanceFields = fields.filter((f) => f.group !== 'Detection' && f.id !== 'interfaces');

	function getCidrPrefix(cidr: string): number | null {
		const parts = cidr.split('/');
		if (parts.length !== 2) return null;
		const prefix = parseInt(parts[1], 10);
		return isNaN(prefix) ? null : prefix;
	}

	// Read cutoff directly from formData to avoid reactivity gaps
	let arpScanCutoff = $derived.by(() => {
		if (formData.discovery_type.type !== 'Unified') return 15;
		const val = formData.discovery_type.scan_settings?.arp_scan_cutoff;
		return typeof val === 'number' && !isNaN(val) ? val : 15;
	});

	// Find interfaced subnets that would be truncated by the cutoff
	let truncatedInterfacedSubnets = $derived.by(() => {
		if (!daemon) return [];
		const interfacedIds = daemon.interfaced_subnet_ids;
		return subnetsData
			.filter((s) => interfacedIds.includes(s.id))
			.filter((s) => {
				// Skip loopback subnets — they're never ARP scanned
				if (s.cidr.startsWith('127.')) return false;
				const prefix = getCidrPrefix(s.cidr);
				return prefix !== null && prefix < arpScanCutoff;
			})
			.map((s) => {
				const prefix = getCidrPrefix(s.cidr)!;
				const ipCount = Math.pow(2, 32 - prefix);
				const name = s.name !== s.cidr ? `${s.name} (${s.cidr})` : s.cidr;
				return { name, ipCount, prefix };
			});
	});

	let showCutoffWarning = $derived(truncatedInterfacedSubnets.length > 0);

	// scan_settings lives inside discovery_type for Unified variant
	function getScanSettings() {
		if (formData.discovery_type.type === 'Unified') {
			return formData.discovery_type.scan_settings ?? {};
		}
		return {};
	}

	let scanValues = $derived({
		scan_rate_pps: getScanSettings().scan_rate_pps ?? '',
		arp_rate_pps: getScanSettings().arp_rate_pps ?? '',
		arp_retries: getScanSettings().arp_retries ?? '',
		arp_scan_cutoff: getScanSettings().arp_scan_cutoff ?? '',
		port_scan_batch_size: getScanSettings().port_scan_batch_size ?? '',
		max_discovery_duration: getScanSettings().max_discovery_duration ?? '',
		use_npcap_arp: getScanSettings().use_npcap_arp ?? false
	});

	function getScanValue(id: string): string | boolean | number {
		return (scanValues as Record<string, string | boolean | number>)[id] ?? '';
	}

	function updateScanSetting(id: string, value: string | boolean | number) {
		if (formData.discovery_type.type !== 'Unified') return;
		const current = formData.discovery_type.scan_settings ?? {};
		if (typeof value === 'number' && isNaN(value)) {
			formData.discovery_type = {
				...formData.discovery_type,
				scan_settings: { ...current, [id]: null }
			};
		} else {
			let clamped = value;
			if (id === 'arp_scan_cutoff' && typeof value === 'number') {
				clamped = Math.max(0, Math.min(32, value));
			}
			formData.discovery_type = {
				...formData.discovery_type,
				scan_settings: { ...current, [id]: clamped }
			};
		}
	}
</script>

{#snippet numberField(id: string)}
	{@const field = performanceFields.find((f) => f.id === id)}
	{#if field}
		<div class="space-y-2">
			<label for={`scan_${field.id}`} class="block text-sm font-medium text-secondary">
				{field.label}
			</label>
			<input
				id={`scan_${field.id}`}
				type="number"
				value={getScanValue(field.id)}
				oninput={(e) => {
					let val = Number(e.currentTarget.value);
					if (field.id === 'arp_scan_cutoff' && !isNaN(val)) {
						val = Math.max(0, Math.min(32, val));
						e.currentTarget.value = String(val);
					}
					updateScanSetting(field.id, val);
				}}
				placeholder={field.placeholder ?? ''}
				disabled={readOnly}
				min={field.id === 'arp_scan_cutoff' ? 0 : undefined}
				max={field.id === 'arp_scan_cutoff' ? 32 : undefined}
				class="input-field"
			/>
			{#if field.help_text}
				<p class="text-xs text-tertiary">{field.help_text}</p>
			{/if}
		</div>
	{/if}
{/snippet}

{#snippet booleanField(id: string, besideInput = false)}
	{@const field = performanceFields.find((f) => f.id === id)}
	{#if field}
		<div class="flex flex-col gap-1 {besideInput ? 'pt-7' : ''}">
			<label
				for={`scan_${field.id}`}
				class="flex cursor-pointer items-center gap-2 text-sm font-medium text-secondary {besideInput
					? 'min-h-9'
					: ''}"
			>
				<input
					type="checkbox"
					id={`scan_${field.id}`}
					checked={!!getScanValue(field.id)}
					disabled={readOnly}
					onchange={(e) => updateScanSetting(field.id, e.currentTarget.checked)}
					class="checkbox-card h-4 w-4 focus:ring-1 focus:ring-blue-500 disabled:cursor-not-allowed disabled:opacity-50"
				/>
				<div>{field.label}</div>
			</label>
			{#if field.help_text}
				<p class="text-xs text-tertiary">{field.help_text}</p>
			{/if}
		</div>
	{/if}
{/snippet}

<div class="space-y-4">
	<div class="flex flex-wrap items-baseline gap-x-1">
		<DocsHint
			text={discovery_docsScanSettings()}
			href="https://scanopy.net/docs/using-scanopy/discovery/"
			linkText={discovery_docsScanSettingsLinkText()}
		/>
	</div>

	<div class="grid grid-cols-1 items-start gap-4 sm:grid-cols-2">
		{@render numberField('scan_rate_pps')}
		{@render numberField('port_scan_batch_size')}
	</div>

	<div class="grid grid-cols-1 items-start gap-4 sm:grid-cols-3">
		{@render numberField('arp_rate_pps')}
		{@render numberField('arp_scan_cutoff')}
		{@render numberField('arp_retries')}
	</div>

	<div class="grid grid-cols-1 items-start gap-4 sm:grid-cols-2">
		{@render numberField('max_discovery_duration')}
		{@render booleanField('use_npcap_arp', true)}
	</div>

	{#if showCutoffWarning}
		{@const maxIps = Math.pow(2, 32 - arpScanCutoff)}
		{@const secondsAt50pps = maxIps / 50}
		{@const subnetNames = truncatedInterfacedSubnets.map((s) => s.name).join(', ')}
		<InlineWarning
			title={secondsAt50pps >= 3600
				? discovery_arpScanCutoffWarningSlow({
						cutoff: String(arpScanCutoff),
						ipCount: maxIps.toLocaleString(),
						timeEstimate: formatDurationHuman(secondsAt50pps),
						subnets: subnetNames
					})
				: discovery_arpScanCutoffWarning({
						cutoff: String(arpScanCutoff),
						ipCount: maxIps.toLocaleString(),
						subnets: subnetNames
					})}
		/>
	{/if}
</div>
