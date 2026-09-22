<script lang="ts">
	import type { Discovery } from '../../types/base';
	import { generateDayTimeCronSchedule } from '../../queries';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import TimeInput from '$lib/shared/components/forms/input/TimeInput.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import {
		common_fri,
		common_mon,
		common_sat,
		common_sun,
		common_thu,
		common_time,
		common_timezone,
		common_tue,
		common_wed,
		discovery_manualDiscovery,
		discovery_manualDiscoveryHelp,
		discovery_scheduleCronExpression,
		discovery_scheduleCronInfo,
		discovery_scheduleDaysOfWeek,
		discovery_scheduleEditAsCron,
		discovery_scheduleFreeDiscoveryPaused,
		discovery_scheduleFreePlanWarning,
		discovery_scheduleHelp,
		discovery_scheduleLapsedWarning,
		discovery_scheduleResetToDayPicker,
		discovery_scheduleTimezoneHelp
	} from '$lib/paraglide/messages';

	interface Props {
		/* eslint-disable @typescript-eslint/no-explicit-any */
		form: any;
		/* eslint-enable @typescript-eslint/no-explicit-any */
		formData: Discovery;
		readOnly?: boolean;
		rawCronMode?: boolean;
		schedulePaused?: boolean;
		/** The pause is because the org lapsed, not because its plan lacks schedules. */
		scheduleLapsed?: boolean;
	}

	let {
		form,
		formData = $bindable(),
		readOnly = false,
		rawCronMode = $bindable(false),
		schedulePaused = false,
		scheduleLapsed = false
	}: Props = $props();

	const dayLabels = [
		() => common_sun(),
		() => common_mon(),
		() => common_tue(),
		() => common_wed(),
		() => common_thu(),
		() => common_fri(),
		() => common_sat()
	];

	let timezoneOptions = $derived(
		Intl.supportedValuesOf('timeZone').map((tz) => ({
			value: tz,
			label: tz
		}))
	);

	function updateCronFromDayTime() {
		if (formData.run_type.type !== 'Scheduled') return;
		const daysStr: string = form.state.values.schedule_days_of_week ?? '0,1,2,3,4,5,6';
		const time: string = form.state.values.schedule_time ?? '00:00';
		const days = daysStr
			.split(',')
			.filter(Boolean)
			.map((d) => parseInt(d));
		const [hour, minute] = time.split(':').map((n) => parseInt(n));
		formData.run_type = {
			...formData.run_type,
			cron_schedule: generateDayTimeCronSchedule(
				days.length > 0 ? days : [0, 1, 2, 3, 4, 5, 6],
				hour || 0,
				minute || 0
			)
		};
	}

	function handleTimezoneChange(value: string) {
		if (formData.run_type.type === 'Scheduled') {
			formData.run_type = {
				...formData.run_type,
				timezone: value
			};
		}
	}

	function handleRawCronChange(value: string) {
		if (formData.run_type.type === 'Scheduled') {
			formData.run_type = {
				...formData.run_type,
				cron_schedule: value
			};
		}
	}

	function toggleDay(field: AnyFieldApi, dayIndex: number) {
		if (readOnly) return;
		const current = ((field.state.value as string) ?? '0,1,2,3,4,5,6')
			.split(',')
			.filter(Boolean)
			.map(Number);
		let updated: number[];
		if (current.includes(dayIndex)) {
			if (current.length <= 1) return;
			updated = current.filter((d) => d !== dayIndex);
		} else {
			updated = [...current, dayIndex].sort((a, b) => a - b);
		}
		field.handleChange(updated.join(','));
		if (formData.run_type.type !== 'Scheduled') return;
		const time: string = form.state.values.schedule_time ?? '00:00';
		const [hour, minute] = time.split(':').map((n: string) => parseInt(n));
		formData.run_type = {
			...formData.run_type,
			cron_schedule: generateDayTimeCronSchedule(updated, hour || 0, minute || 0)
		};
	}

	function isDaySelected(field: AnyFieldApi, dayIndex: number): boolean {
		return ((field.state.value as string) ?? '0,1,2,3,4,5,6')
			.split(',')
			.filter(Boolean)
			.map(Number)
			.includes(dayIndex);
	}

	function switchToRawCron() {
		rawCronMode = true;
	}

	function resetToDayPicker() {
		rawCronMode = false;
		const time: string = form.state.values.schedule_time ?? '00:00';
		const [hour, minute] = time.split(':').map((n) => parseInt(n));
		const cron = generateDayTimeCronSchedule([0, 1, 2, 3, 4, 5, 6], hour || 0, minute || 0);
		form.setFieldValue('schedule_days_of_week', '0,1,2,3,4,5,6');
		if (formData.run_type.type === 'Scheduled') {
			formData.run_type = {
				...formData.run_type,
				cron_schedule: cron
			};
			form.setFieldValue('schedule_cron', cron);
		}
	}
</script>

{#if formData.run_type.type === 'Scheduled'}
	<div class="space-y-4">
		{#if schedulePaused}
			<InlineWarning
				title={discovery_scheduleFreeDiscoveryPaused()}
				body={scheduleLapsed
					? discovery_scheduleLapsedWarning()
					: discovery_scheduleFreePlanWarning()}
			/>
		{/if}
		<p class="text-tertiary text-sm">
			{discovery_scheduleHelp()}
		</p>

		{#if rawCronMode}
			<!-- Raw Cron Mode -->
			<form.Field
				name="schedule_cron"
				listeners={{
					onChange: ({ value }: { value: string }) => handleRawCronChange(value)
				}}
			>
				{#snippet children(field: AnyFieldApi)}
					<TextInput
						label={discovery_scheduleCronExpression()}
						id="schedule_cron"
						{field}
						disabled={readOnly}
						placeholder="0 0 0 * * *"
					/>
				{/snippet}
			</form.Field>

			<InlineInfo title={discovery_scheduleCronExpression()} body={discovery_scheduleCronInfo()} />

			<button
				type="button"
				class="text-sm text-blue-400 hover:text-blue-300"
				onclick={resetToDayPicker}
				disabled={readOnly}
			>
				{discovery_scheduleResetToDayPicker()}
			</button>
		{:else}
			<!-- Day Picker Mode -->
			<form.Field name="schedule_days_of_week">
				{#snippet children(field: AnyFieldApi)}
					<div>
						<span class="text-secondary mb-2 block text-sm font-medium">
							{discovery_scheduleDaysOfWeek()}
						</span>
						<div class="flex gap-1">
							{#each [1, 2, 3, 4, 5, 6, 0] as dayIndex (dayIndex)}
								<button
									type="button"
									class="{isDaySelected(field, dayIndex)
										? 'btn-info'
										: 'btn-secondary'} px-3 py-1.5 text-sm"
									disabled={readOnly}
									onclick={() => toggleDay(field, dayIndex)}
								>
									{dayLabels[dayIndex]()}
								</button>
							{/each}
						</div>
					</div>
				{/snippet}
			</form.Field>

			<form.Field
				name="schedule_time"
				listeners={{
					onChange: () => updateCronFromDayTime()
				}}
			>
				{#snippet children(field: AnyFieldApi)}
					<TimeInput label={common_time()} id="schedule_time" {field} disabled={readOnly} />
				{/snippet}
			</form.Field>

			<button
				type="button"
				class="text-sm text-blue-400 hover:text-blue-300"
				onclick={switchToRawCron}
				disabled={readOnly}
			>
				{discovery_scheduleEditAsCron()}
			</button>
		{/if}

		<!-- Timezone (shown in both modes) -->
		<form.Field
			name="schedule_timezone"
			listeners={{
				onChange: ({ value }: { value: string }) => handleTimezoneChange(value)
			}}
		>
			{#snippet children(field: AnyFieldApi)}
				<SelectInput
					label={common_timezone()}
					id="schedule_timezone"
					options={timezoneOptions}
					{field}
					disabled={readOnly}
					helpText={discovery_scheduleTimezoneHelp()}
				/>
			{/snippet}
		</form.Field>
	</div>
{:else}
	<!-- Ad-hoc info box (safety fallback if rendered in AdHoc mode) -->
	<div class="card flex items-start gap-3">
		<svg
			class="text-tertiary mt-0.5 h-5 w-5 flex-shrink-0"
			fill="none"
			stroke="currentColor"
			viewBox="0 0 24 24"
		>
			<path
				stroke-linecap="round"
				stroke-linejoin="round"
				stroke-width="2"
				d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
			/>
		</svg>
		<div>
			<h4 class="mb-1 text-sm font-medium text-gray-300">{discovery_manualDiscovery()}</h4>
			<p class="text-sm text-gray-400">
				{discovery_manualDiscoveryHelp()}
			</p>
		</div>
	</div>
{/if}
