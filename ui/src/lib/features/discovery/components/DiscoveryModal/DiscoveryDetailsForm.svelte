<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import { required, max } from '$lib/shared/components/forms/validators';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import { DaemonDisplay } from '$lib/shared/components/forms/selection/display/DaemonDisplay.svelte';
	import {
		SimpleOptionDisplay,
		type SimpleOption
	} from '$lib/shared/components/forms/selection/display/SimpleOptionDisplay';
	import type { Discovery } from '../../types/base';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import type { Host } from '$lib/features/hosts/types/base';
	import type { Subnet } from '$lib/features/subnets/types/base';
	import { openModal } from '$lib/shared/stores/modal-registry';
	import { ArrowUpCircle } from 'lucide-svelte';
	import CollapsibleCard from '$lib/shared/components/data/CollapsibleCard.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import {
		common_daemon,
		common_ipAddress,
		discovery_adHoc,
		discovery_adHocDescription,
		discovery_bestService,
		discovery_daemonHelp,
		discovery_daemonSelect,
		discovery_hostNameFallback,
		discovery_hostNameFallbackHelp,
		discovery_name,
		discovery_namePlaceholder,
		discovery_runType,
		discovery_lastRun,
		discovery_neverRun,
		discovery_scanCount,
		discovery_scanInfo,
		discovery_scanModeBaselinePending,
		discovery_scanModeFirstLight,
		discovery_scanModeInfo,
		discovery_scanModeFull,
		discovery_scanModeLight,
		discovery_scheduled,
		discovery_scheduledDescription,
		discovery_upgradeRequiredTitle,
		discovery_upgradeRequiredBody
	} from '$lib/paraglide/messages';

	interface Props {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any; state: { values: { name: string } } };
		formData: Discovery;
		daemons?: Daemon[];
		hosts?: Host[];
		subnets?: Subnet[];
		readOnly?: boolean;
		hasScheduledDiscovery?: boolean;
		daemon?: Daemon | null;
	}

	let {
		form,
		formData = $bindable(),
		daemons = [],
		hosts = [],
		subnets = [],
		readOnly = false,
		hasScheduledDiscovery = true,
		daemon = null
	}: Props = $props();

	let runTypeOptions: SimpleOption[] = $derived([
		{ value: 'AdHoc', label: discovery_adHoc() },
		{
			value: 'Scheduled',
			label: discovery_scheduled(),
			disabled: !hasScheduledDiscovery,
			tags: !hasScheduledDiscovery
				? [
						{
							label: 'Upgrade',
							color: 'Yellow',
							icon: ArrowUpCircle
						}
					]
				: []
		}
	]);

	function handleRunTypeChange(value: string) {
		if (value === 'AdHoc' && formData.run_type.type !== 'AdHoc') {
			formData.run_type = {
				type: 'AdHoc',
				last_run: null
			};
		} else if (value === 'Scheduled' && formData.run_type.type !== 'Scheduled') {
			formData.run_type = {
				type: 'Scheduled',
				cron_schedule: '0 0 0 * * 0',
				last_run: null,
				enabled: true,
				timezone: Intl.DateTimeFormat().resolvedOptions().timeZone
			};
		}
	}

	let hostNameFallbackOptions = $derived([
		{ value: 'Ip', label: common_ipAddress() },
		{ value: 'BestService', label: discovery_bestService() }
	]);

	function handleHostNameFallbackChange(value: string) {
		if (
			formData.discovery_type.type == 'Docker' ||
			formData.discovery_type.type == 'Network' ||
			formData.discovery_type.type == 'Unified'
		) {
			if (formData.discovery_type.host_naming_fallback !== value) {
				formData.discovery_type = {
					...formData.discovery_type,
					host_naming_fallback: value as 'BestService' | 'Ip'
				};
			}
		}
	}
</script>

<div class="space-y-4">
	<form.Field
		name="name"
		validators={{
			onBlur: ({ value }: { value: string }) => required(value) || max(100)(value)
		}}
	>
		{#snippet children(field: AnyFieldApi)}
			<TextInput
				label={discovery_name()}
				id="name"
				placeholder={discovery_namePlaceholder()}
				required={true}
				{field}
				disabled={readOnly}
			/>
		{/snippet}
	</form.Field>

	<!-- Daemon Selection -->
	<div class="space-y-2">
		<RichSelect
			label={common_daemon()}
			required={true}
			placeholder={discovery_daemonSelect()}
			disabled={readOnly}
			selectedValue={formData.daemon_id}
			options={daemons}
			displayComponent={DaemonDisplay}
			getOptionContext={() => ({ hosts, subnets })}
			onSelect={(value) => {
				const selectedDaemon = daemons.find((d) => d.id === value);
				if (selectedDaemon) {
					formData = { ...formData, daemon_id: value, network_id: selectedDaemon.network_id };
				}
			}}
		/>
		<p class="text-tertiary text-xs">{discovery_daemonHelp()}</p>
	</div>

	{#if daemon && daemon.version_status?.supports_unified_discovery === false}
		<InlineWarning
			title={discovery_upgradeRequiredTitle()}
			body={discovery_upgradeRequiredBody()}
		/>
	{/if}

	<!-- Run Type Selection -->
	<form.Field
		name="run_type_type"
		listeners={{
			onChange: ({ value }: { value: string }) => handleRunTypeChange(value)
		}}
	>
		{#snippet children(field: AnyFieldApi)}
			<RichSelect
				label={discovery_runType()}
				selectedValue={field.state.value}
				options={runTypeOptions}
				onSelect={(value) => field.handleChange(value)}
				onDisabledClick={() => openModal('billing-plan')}
				displayComponent={SimpleOptionDisplay}
				disabled={readOnly}
			/>
			<p class="text-tertiary mt-1 text-xs">
				{field.state.value === 'AdHoc'
					? discovery_adHocDescription()
					: discovery_scheduledDescription()}
			</p>
		{/snippet}
	</form.Field>

	{#if formData.discovery_type.type == 'Docker' || formData.discovery_type.type == 'Network' || formData.discovery_type.type == 'Unified'}
		<form.Field
			name="host_naming_fallback"
			listeners={{
				onChange: ({ value }: { value: string }) => handleHostNameFallbackChange(value)
			}}
		>
			{#snippet children(field: AnyFieldApi)}
				<SelectInput
					label={discovery_hostNameFallback()}
					id="host_name_fallback"
					options={hostNameFallbackOptions}
					{field}
					disabled={readOnly}
					helpText={discovery_hostNameFallbackHelp()}
				/>
			{/snippet}
		</form.Field>
	{/if}

	{#if formData.discovery_type.type === 'Unified' && formData.scan_count !== undefined}
		{@const scanCount = formData.scan_count ?? 0}
		{@const interval = formData.discovery_type.scan_settings?.full_scan_interval ?? 3}
		{@const nextScanNumber = scanCount + 1}
		{@const nextIsFullScan =
			formData.force_full_scan ||
			(interval !== 0 &&
				(interval === 1 ||
					scanCount === 1 ||
					(scanCount > 1 && interval > 0 && scanCount % interval === 0)))}
		{@const lastRun =
			formData.run_type.type === 'Scheduled' || formData.run_type.type === 'AdHoc'
				? formData.run_type.last_run
				: null}
		<CollapsibleCard title={discovery_scanInfo()} expanded={true}>
			<div class="space-y-1">
				{#if scanCount === 0}
					<p class="text-secondary text-sm">{discovery_neverRun()}</p>
					<p class="text-tertiary text-xs">{discovery_scanModeFirstLight()}</p>
				{:else}
					<p class="text-secondary text-sm">
						{discovery_lastRun({ time: lastRun ? new Date(lastRun).toLocaleString() : '—' })}
					</p>
					<p class="text-secondary text-sm">{discovery_scanCount({ count: String(scanCount) })}</p>
					<p class="text-tertiary text-sm">
						{discovery_scanModeInfo({
							next: String(nextScanNumber),
							mode: nextIsFullScan ? discovery_scanModeFull() : discovery_scanModeLight()
						})}
					</p>
					{#if scanCount === 1}
						<p class="text-tertiary text-xs">{discovery_scanModeBaselinePending()}</p>
					{/if}
				{/if}
			</div>
		</CollapsibleCard>
	{/if}
</div>
