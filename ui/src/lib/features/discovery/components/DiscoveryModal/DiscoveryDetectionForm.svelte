<script lang="ts">
	import scanSettingsFields from '$lib/data/scan-settings.json';
	import type { Discovery } from '../../types/base';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';
	import {
		discovery_forceFullScan,
		discovery_forceFullScanHelp,
		discovery_fullPortScan,
		discovery_scanModeIntervalExplainer
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
	const detectionFields = fields.filter((f) => f.category === 'Detection');
	// full_scan_interval is grouped with force_full_scan in its own card
	const booleanFields = detectionFields.filter(
		(f) => f.field_type === 'boolean' && f.id !== 'full_scan_interval'
	);
	const fullScanIntervalField = detectionFields.find((f) => f.id === 'full_scan_interval');

	let rawSocketServiceNames = $derived(
		(serviceDefinitions.getItems() ?? [])
			.filter((s) => s.metadata?.has_raw_socket_endpoint)
			.map((s) => s.name)
			.join(', ')
	);

	function getHelpText(field: FieldDef): string {
		if (field.id === 'probe_raw_socket_ports' && rawSocketServiceNames) {
			return `${field.help_text} Required to detect: ${rawSocketServiceNames}`;
		}
		return field.help_text ?? '';
	}

	function getScanSettings() {
		if (formData.discovery_type.type === 'Unified') {
			return formData.discovery_type.scan_settings ?? {};
		}
		return {};
	}

	let scanValues = $derived({
		probe_raw_socket_ports: getScanSettings().probe_raw_socket_ports ?? false,
		full_scan_interval: getScanSettings().full_scan_interval ?? ''
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
	{#each booleanFields as field (field.id)}
		<div class="card">
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
				{#if getHelpText(field)}
					<p class="text-tertiary text-xs">{getHelpText(field)}</p>
				{/if}
			</div>
		</div>
	{/each}

	{#if fullScanIntervalField}
		<div class="card space-y-3">
			<h4 class="text-secondary text-sm font-medium">{discovery_fullPortScan()}</h4>
			<div class="space-y-2">
				<label for="scan_full_scan_interval" class="text-secondary block text-sm font-medium">
					{fullScanIntervalField.label}
				</label>
				<input
					id="scan_full_scan_interval"
					type="number"
					value={getScanValue('full_scan_interval')}
					oninput={(e) => updateScanSetting('full_scan_interval', Number(e.currentTarget.value))}
					placeholder={fullScanIntervalField.placeholder ?? ''}
					disabled={readOnly}
					class="input-field"
				/>
				{#if fullScanIntervalField.help_text}
					<p class="text-tertiary text-xs">{fullScanIntervalField.help_text}</p>
				{/if}
				<p class="text-tertiary text-xs italic">{discovery_scanModeIntervalExplainer()}</p>
			</div>
			{#if formData.discovery_type.type === 'Unified'}
				<div class="flex flex-col gap-1 pt-1">
					<label
						for="scan_force_full_scan"
						class="text-secondary flex cursor-pointer items-center gap-2 text-sm font-medium"
					>
						<input
							type="checkbox"
							id="scan_force_full_scan"
							checked={formData.force_full_scan ?? false}
							disabled={readOnly}
							onchange={(e) => {
								formData.force_full_scan = e.currentTarget.checked;
							}}
							class="checkbox-card h-4 w-4 focus:ring-1 focus:ring-blue-500 disabled:cursor-not-allowed disabled:opacity-50"
						/>
						<div>{discovery_forceFullScan()}</div>
					</label>
					<p class="text-tertiary text-xs">{discovery_forceFullScanHelp()}</p>
				</div>
			{/if}
		</div>
	{/if}
</div>
