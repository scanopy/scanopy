<script lang="ts">
	import daemonVersionGates from '#lib/data/daemon-version-gates.json';
	import { formatTimestamp } from '#lib/shared/utils/formatting.ts';
	import { effectiveTimeZone } from '#lib/shared/stores/display-settings.svelte.ts';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import { required, max } from '#lib/shared/components/forms/validators.ts';
	import TextInput from '#lib/shared/components/forms/input/TextInput.svelte';
	import RichSelect from '#lib/shared/components/forms/selection/RichSelect.svelte';
	import { DaemonDisplay } from '#lib/shared/components/forms/selection/display/DaemonDisplay.svelte';
	import {
		SimpleOptionDisplay,
		type SimpleOption
	} from '#lib/shared/components/forms/selection/display/SimpleOptionDisplay.ts';
	import type { Discovery } from '../../types/base';
	import type { Daemon } from '#lib/features/daemons/types/base.ts';
	import { isPreUnifiedDaemon } from '#lib/features/daemons/utils.ts';
	import type { Host } from '#lib/features/hosts/types/base.ts';
	import type { Subnet } from '#lib/features/subnets/types/base.ts';
	import { triggerUpgrade } from '#lib/features/billing/trigger-upgrade.ts';
	import { ArrowUpCircle } from '@lucide/svelte';
	import CollapsibleCard from '#lib/shared/components/data/CollapsibleCard.svelte';
	import InlineWarning from '#lib/shared/components/feedback/InlineWarning.svelte';
	import SelectInput from '#lib/shared/components/forms/input/SelectInput.svelte';
	import {
		common_daemon,
		common_ipAddress,
		discovery_adHoc,
		discovery_adHocDescription,
		discovery_bestService,
		discovery_daemonHelp,
		discovery_daemonSelect,
		common_discovery,
		common_entityName,
		discovery_hostNameFallback,
		discovery_hostNameFallbackHelp,
		discovery_namePlaceholder,
		discovery_runType,
		discovery_lastRun,
		discovery_neverRun,
		discovery_scanCount,
		discovery_scanInfo,
		discovery_nextRunFull,
		discovery_nextRunLight,
		discovery_scheduled,
		discovery_scheduledDescription,
		discovery_upgradeRequiredTitle,
		discovery_upgradeRequiredBody
	} from '#lib/paraglide/messages.js';

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
				timezone: effectiveTimeZone()
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
	{#if daemon && isPreUnifiedDaemon(daemon)}
		<InlineWarning
			title={discovery_upgradeRequiredTitle()}
			body={discovery_upgradeRequiredBody({
				version: daemonVersionGates.last_without_unified_discovery
			})}
		/>
	{/if}

	<div class="space-y-3">
		<form.Field
			name="name"
			validators={{
				onBlur: ({ value }: { value: string }) => required(value) || max(100)(value)
			}}
		>
			{#snippet children(field: AnyFieldApi)}
				<TextInput
					label={common_entityName({ entity: common_discovery() })}
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
						formData = { ...formData, daemon_id: value, site_id: selectedDaemon.site_id };
					}
				}}
			/>
			<p class="text-xs text-tertiary">{discovery_daemonHelp()}</p>
		</div>

		<!-- Run Type Selection -->
		<div class="grid grid-cols-1 items-start gap-4 sm:grid-cols-2">
			<div>
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
							onDisabledClick={() =>
								triggerUpgrade({
									feature: 'scheduled_discovery',
									source: 'discovery_form',
									surface: 'discovery_form'
								})}
							displayComponent={SimpleOptionDisplay}
							disabled={readOnly}
						/>
						<p class="mt-1 text-xs text-tertiary">
							{field.state.value === 'AdHoc'
								? discovery_adHocDescription()
								: discovery_scheduledDescription()}
						</p>
					{/snippet}
				</form.Field>
			</div>
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
		</div>
	</div>

	{#if formData.discovery_type.type === 'Unified' && formData.scan_count !== undefined}
		{@const scanCount = formData.scan_count ?? 0}
		{@const lastRun =
			formData.run_type.type === 'Scheduled' || formData.run_type.type === 'AdHoc'
				? formData.run_type.last_run
				: null}
		<CollapsibleCard title={discovery_scanInfo()} expanded={true}>
			<div class="space-y-1">
				{#if scanCount === 0}
					<p class="text-sm text-secondary">{discovery_neverRun()}</p>
				{:else}
					<p class="text-sm text-secondary">
						{discovery_lastRun({ time: lastRun ? formatTimestamp(lastRun) : '—' })}
					</p>
					<p class="text-sm text-secondary">{discovery_scanCount({ count: String(scanCount) })}</p>
				{/if}
				<!-- Which run is full is decided on the server; it sends the result for the next run. -->
				<p class="text-xs text-tertiary">
					{formData.next_run_is_full_scan ? discovery_nextRunFull() : discovery_nextRunLight()}
				</p>
			</div>
		</CollapsibleCard>
	{/if}
</div>
