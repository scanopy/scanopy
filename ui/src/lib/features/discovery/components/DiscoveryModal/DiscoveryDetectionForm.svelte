<script lang="ts">
	import scanSettingsFields from '$lib/data/scan-settings.json';
	import type { FieldDefinition, ServicedDefinitionMetadata } from '$lib/shared/stores/metadata';
	import type { Discovery } from '../../types/base';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';
	import { translateFieldDefinitions } from '$lib/i18n/metadata';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import {
		common_enabled,
		discovery_firstScanMustBeLight,
		discovery_forceFullScan,
		discovery_forceFullScanHelp,
		discovery_fullPortScan,
		discovery_requiredToDetect,
		discovery_requiredToDetectRouted,
		discovery_scanModeIntervalExplainer
	} from '$lib/paraglide/messages';

	interface Props {
		formData: Discovery;
		readOnly?: boolean;
		isEditing?: boolean;
	}

	let { formData = $bindable(), readOnly = false, isEditing = false }: Props = $props();

	// Labels/placeholders/help text resolved via meta_* i18n keys with fixture fallback, as
	// DiscoveryScanSettingsForm does. Without it every Detection field renders raw English.
	const fields = translateFieldDefinitions(
		'scan_settings',
		null,
		scanSettingsFields as FieldDefinition[]
	);
	const detectionFields = fields.filter((f) => f.group === 'Detection');
	// full_scan_interval is grouped with force_full_scan in its own card
	const booleanFields = detectionFields.filter(
		(f) => f.field_type === 'boolean' && f.id !== 'full_scan_interval'
	);
	const fullScanIntervalField = detectionFields.find((f) => f.id === 'full_scan_interval');
	const maxDiscoveryDurationField = detectionFields.find((f) => f.id === 'max_discovery_duration');

	function serviceNamesWhere(predicate: (metadata: ServicedDefinitionMetadata) => boolean): string {
		return (serviceDefinitions.getItems() ?? [])
			.filter((s) => s.metadata && predicate(s.metadata))
			.map((s) => s.name)
			.filter((name): name is string => !!name)
			.sort()
			.join(', ');
	}

	let rawSocketServiceNames = $derived(serviceNamesWhere((m) => m.gated_by_raw_socket_scanning));
	let connectOnlyServiceNames = $derived(serviceNamesWhere((m) => m.connect_only));

	/** Which detections a setting is required for, appended to its help text.
	 *
	 *  Both lists come from service metadata rather than being written here, so they shrink on their
	 *  own as detections stop being port-only. A service appearing in both needs both settings —
	 *  JetDirect on a routed subnet is the case, since 9100 is a raw-socket port *and* the only
	 *  evidence the definition has. */
	function getHelpText(field: FieldDefinition): string {
		if (field.id === 'probe_raw_socket_ports' && rawSocketServiceNames) {
			return `${field.help_text} ${discovery_requiredToDetect({ services: rawSocketServiceNames })}`;
		}
		if (field.id === 'trust_port_only_detections' && connectOnlyServiceNames) {
			return `${field.help_text} ${discovery_requiredToDetectRouted({ services: connectOnlyServiceNames })}`;
		}
		return field.help_text ?? '';
	}

	function getScanSettings() {
		if (formData.discovery_type.type === 'Unified') {
			return formData.discovery_type.scan_settings ?? {};
		}
		return {};
	}

	/** Read straight from the settings by field id rather than from a hand-written map. The map this
	 *  replaces silently rendered any field missing from it as unchecked, and an unchecked box the
	 *  user never touches is never written back — so a new boolean looked present and did nothing. */
	function getScanValue(id: string): string | boolean | number {
		const settings = getScanSettings() as Record<string, string | boolean | number | null>;
		const value = settings[id];
		if (value === null || value === undefined) {
			// Booleans default off; the numeric fields render their placeholder when empty.
			return fields.find((f) => f.id === id)?.field_type === 'boolean' ? false : '';
		}
		return value;
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
		<InfoCard title={field.label}>
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
					<div>{common_enabled()}</div>
				</label>
				{#if getHelpText(field)}
					<p class="text-tertiary text-xs">{getHelpText(field)}</p>
				{/if}
			</div>
		</InfoCard>
	{/each}

	{#if fullScanIntervalField}
		<InfoCard title={discovery_fullPortScan()}>
			<div
				class="space-y-2"
				use:tooltip
				data-tooltip={!isEditing ? discovery_firstScanMustBeLight() : null}
			>
				<label for="scan_full_scan_interval" class="text-secondary block text-sm font-medium">
					{fullScanIntervalField.label}
				</label>
				<input
					id="scan_full_scan_interval"
					type="number"
					value={getScanValue('full_scan_interval')}
					oninput={(e) => updateScanSetting('full_scan_interval', Number(e.currentTarget.value))}
					placeholder={fullScanIntervalField.placeholder ?? ''}
					disabled={readOnly || !isEditing}
					class="input-field"
				/>
				{#if fullScanIntervalField.help_text}
					<p class="text-tertiary text-xs">{fullScanIntervalField.help_text}</p>
				{/if}
				<p class="text-tertiary text-xs italic">{discovery_scanModeIntervalExplainer()}</p>
			</div>
			{#if formData.discovery_type.type === 'Unified'}
				<div
					class="flex flex-col gap-1 pt-1"
					use:tooltip
					data-tooltip={!isEditing ? discovery_firstScanMustBeLight() : null}
				>
					<label
						for="scan_force_full_scan"
						class="text-secondary flex items-center gap-2 text-sm font-medium"
						class:cursor-pointer={isEditing && !readOnly}
						class:cursor-not-allowed={!isEditing || readOnly}
						class:opacity-50={!isEditing}
					>
						<input
							type="checkbox"
							id="scan_force_full_scan"
							checked={formData.force_full_scan ?? false}
							disabled={readOnly || !isEditing}
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
		</InfoCard>
	{/if}

	{#if maxDiscoveryDurationField}
		<InfoCard title={maxDiscoveryDurationField.label} variant="compact">
			<input
				id="scan_max_discovery_duration"
				aria-label={maxDiscoveryDurationField.label}
				type="number"
				value={getScanValue('max_discovery_duration')}
				oninput={(e) => updateScanSetting('max_discovery_duration', Number(e.currentTarget.value))}
				placeholder={maxDiscoveryDurationField.placeholder ?? ''}
				disabled={readOnly}
				class="input-field"
			/>
			{#if maxDiscoveryDurationField.help_text}
				<p class="text-tertiary text-xs">{maxDiscoveryDurationField.help_text}</p>
			{/if}
		</InfoCard>
	{/if}
</div>
