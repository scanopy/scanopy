<script lang="ts">
	import scanSettingsFields from '$lib/data/scan-settings.json';
	import type { FieldDefinition } from '$lib/shared/stores/metadata';
	import CollapsibleCard from '$lib/shared/components/data/CollapsibleCard.svelte';
	import type { Discovery } from '../../types/base';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import { formatDurationHuman } from '$lib/shared/utils/formatting';
	import { translateFieldDefinitions } from '$lib/i18n/metadata';
	import {
		discovery_scanSettingsHelp,
		discovery_docsScanSettings,
		discovery_docsScanSettingsLinkText,
		discovery_arpScanCutoffWarning,
		discovery_arpScanCutoffWarningSlow
	} from '$lib/paraglide/messages';

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

	// Section the form by each field's declared group
	const groupNames = [
		...new Set(performanceFields.map((f) => f.group).filter(Boolean))
	] as string[];
	const fieldsByGroup =
		groupNames.length > 0
			? groupNames.map((name) => ({
					name,
					fields: performanceFields.filter((f) => f.group === name)
				}))
			: [{ name: 'Performance', fields: performanceFields }];

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

<div class="space-y-4">
	<p class="text-tertiary text-sm">{discovery_scanSettingsHelp()}</p>
	<DocsHint
		text={discovery_docsScanSettings()}
		href="https://scanopy.net/docs/using-scanopy/discovery/"
		linkText={discovery_docsScanSettingsLinkText()}
	/>

	{#each fieldsByGroup as group (group.name)}
		{@const numberFields = group.fields.filter((f) => f.field_type === 'number')}
		{@const booleanFields = group.fields.filter((f) => f.field_type === 'boolean')}
		<CollapsibleCard title={group.name} expanded={true}>
			<div class="space-y-3">
				<div class="grid grid-cols-2 items-center gap-4">
					{#each numberFields as field (field.id)}
						<div class="space-y-2">
							<label for={`scan_${field.id}`} class="text-secondary block text-sm font-medium">
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
								<p class="text-tertiary text-xs">{field.help_text}</p>
							{/if}
						</div>
						{#if field.id === 'arp_scan_cutoff' && showCutoffWarning}
							{@const maxIps = Math.pow(2, 32 - arpScanCutoff)}
							{@const secondsAt50pps = maxIps / 50}
							{@const subnetNames = truncatedInterfacedSubnets.map((s) => s.name).join(', ')}
							<div class="col-span-2">
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
							</div>
						{/if}
					{/each}
				</div>

				{#if booleanFields.length > 0}
					<div class="grid grid-cols-2 gap-4 pt-2">
						{#each booleanFields as field (field.id)}
							<div class="flex flex-col gap-1">
								<label
									for={`scan_${field.id}`}
									class="text-secondary flex cursor-pointer items-center gap-2 text-sm font-medium"
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
									<p class="text-tertiary text-xs">{field.help_text}</p>
								{/if}
							</div>
						{/each}
					</div>
				{/if}
			</div>
		</CollapsibleCard>
	{/each}
</div>
