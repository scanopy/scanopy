<script lang="ts">
	import scanSettingsFields from '$lib/data/scan-settings.json';
	import CollapsibleCard from '$lib/shared/components/data/CollapsibleCard.svelte';
	import type { Discovery } from '../../types/base';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import {
		discovery_scanSettingsHelp,
		discovery_docsScanSettings,
		discovery_docsScanSettingsLinkText
	} from '$lib/paraglide/messages';

	interface Props {
		formData: Discovery;
		readOnly?: boolean;
	}

	let { formData = $bindable(), readOnly = false }: Props = $props();

	type FieldDef = {
		id: string;
		label: string;
		field_type: string;
		placeholder?: string;
		help_text?: string;
		default_value?: string;
		optional?: boolean;
		category?: string;
	};

	const fields = scanSettingsFields as FieldDef[];

	// Only include performance-related fields (exclude Detection category)
	const performanceFields = fields.filter(
		(f) => f.category !== 'Detection' && f.id !== 'interfaces'
	);

	// Group fields by category
	const categories = [
		...new Set(performanceFields.map((f) => f.category).filter(Boolean))
	] as string[];
	const fieldsByCategory =
		categories.length > 0
			? categories.map((cat) => ({
					name: cat,
					fields: performanceFields.filter((f) => f.category === cat)
				}))
			: [{ name: 'Performance', fields: performanceFields }];

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
			formData.discovery_type = {
				...formData.discovery_type,
				scan_settings: { ...current, [id]: value }
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

	{#each fieldsByCategory as category (category.name)}
		{@const numberFields = category.fields.filter((f) => f.field_type !== 'boolean')}
		{@const booleanFields = category.fields.filter((f) => f.field_type === 'boolean')}
		<CollapsibleCard title={category.name} expanded={true}>
			<div class="space-y-3">
				<div class="grid grid-cols-2 gap-4">
					{#each numberFields as field (field.id)}
						<div class="space-y-2">
							<label for={`scan_${field.id}`} class="text-secondary block text-sm font-medium">
								{field.label}
							</label>
							<input
								id={`scan_${field.id}`}
								type="number"
								value={getScanValue(field.id)}
								oninput={(e) => updateScanSetting(field.id, Number(e.currentTarget.value))}
								placeholder={field.placeholder ?? ''}
								disabled={readOnly}
								class="input-field"
							/>
							{#if field.help_text}
								<p class="text-tertiary text-xs">{field.help_text}</p>
							{/if}
						</div>
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
