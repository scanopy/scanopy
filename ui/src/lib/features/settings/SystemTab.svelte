<script lang="ts">
	import { Monitor, Sun, Moon } from 'lucide-svelte';
	import { createForm } from '@tanstack/svelte-form';
	import { themeStore } from '$lib/shared/stores/theme.svelte';
	import {
		browserTimeZone,
		DEFAULT_DISPLAY_SETTINGS,
		displaySettings,
		timeZoneOptions
	} from '$lib/shared/stores/display-settings.svelte';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useUpdateSelfMutation } from '$lib/features/users/queries';
	import { formatDate, formatRelativeTime, formatTimestamp } from '$lib/shared/utils/formatting';
	import { pushSuccess } from '$lib/shared/stores/feedback';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import CollapsibleCard from '$lib/shared/components/data/CollapsibleCard.svelte';
	import type { Snippet } from 'svelte';
	import type { components } from '$lib/api/schema';
	import {
		common_browserDefault,
		common_system,
		common_theme,
		common_timezone,
		common_light,
		common_license,
		common_dark,
		common_monday,
		common_source,
		common_sunday,
		common_clock,
		common_12Hour,
		common_24Hour,
		common_comfortable,
		common_compact,
		common_tables,
		settings_system_copyright,
		settings_system_dashboardIconsAttribution,
		settings_system_dateAndTime,
		settings_system_dateAndTimeDesc,
		settings_system_dateOrder,
		settings_system_dateOrderDayFirst,
		settings_system_dateOrderIso,
		settings_system_dateOrderMonthFirst,
		settings_system_displayUpdated,
		settings_system_license,
		settings_system_licenseLinkText,
		settings_system_preview,
		settings_system_recogAttribution,
		settings_system_simpleIconsAttribution,
		settings_system_tablesDesc,
		settings_system_tablesUpdated,
		settings_system_themeDesc,
		settings_system_timeZoneBrowser,
		settings_system_timestamps,
		settings_system_timestampsAbsolute,
		settings_system_timestampsRelative,
		settings_system_tuxAttribution,
		settings_system_weekStart
	} from '$lib/paraglide/messages';

	type DisplaySettings = components['schemas']['DisplaySettings'];
	type DateOrder = components['schemas']['DateOrder'];
	type ClockFormat = components['schemas']['ClockFormat'];
	type WeekStart = components['schemas']['WeekStart'];
	type TimestampStyle = components['schemas']['TimestampStyle'];
	type TableDensity = components['schemas']['TableDensity'];

	const options = [
		{ id: 'system' as const, label: common_system(), icon: Monitor },
		{ id: 'light' as const, label: common_light(), icon: Sun },
		{ id: 'dark' as const, label: common_dark(), icon: Moon }
	];

	const copyrightYear = new Date().getFullYear();

	// Option values are the generated enum types, so a renamed backend variant fails `npm run check`.
	const dateOrderOptions: { value: DateOrder; label: string }[] = [
		{ value: 'browser_default', label: common_browserDefault() },
		{ value: 'iso', label: settings_system_dateOrderIso() },
		{ value: 'day_first', label: settings_system_dateOrderDayFirst() },
		{ value: 'month_first', label: settings_system_dateOrderMonthFirst() }
	];
	const clockOptions: { value: ClockFormat; label: string }[] = [
		{ value: 'browser_default', label: common_browserDefault() },
		{ value: 'twelve_hour', label: common_12Hour() },
		{ value: 'twenty_four_hour', label: common_24Hour() }
	];
	const weekStartOptions: { value: WeekStart; label: string }[] = [
		{ value: 'monday', label: common_monday() },
		{ value: 'sunday', label: common_sunday() }
	];
	const timestampOptions: { value: TimestampStyle; label: string }[] = [
		{ value: 'relative', label: settings_system_timestampsRelative() },
		{ value: 'absolute', label: settings_system_timestampsAbsolute() }
	];
	const tableDensityOptions: { value: TableDensity; label: string }[] = [
		{ value: 'comfortable', label: common_comfortable() },
		{ value: 'compact', label: common_compact() }
	];
	// '' stands for "no zone chosen" (null on the server), since a <select> value is a string.
	const timeZoneSelectOptions = [
		{ value: '', label: settings_system_timeZoneBrowser({ zone: browserTimeZone() }) },
		...timeZoneOptions()
	];

	const currentUserQuery = useCurrentUserQuery();
	const updateSelfMutation = useUpdateSelfMutation();
	let user = $derived(currentUserQuery.data);

	type FormValues = Omit<DisplaySettings, 'time_zone'> & { time_zone: string };

	function toFormValues(settings: DisplaySettings | undefined): FormValues {
		const merged = { ...DEFAULT_DISPLAY_SETTINGS, ...settings };
		return { ...merged, time_zone: merged.time_zone ?? '' };
	}

	function fromFormValues(value: FormValues): DisplaySettings {
		return { ...value, time_zone: value.time_zone || null };
	}

	// Stable literal defaults; never read reactive state inside the createForm options getter.
	const form = createForm(() => ({
		defaultValues: toFormValues(DEFAULT_DISPLAY_SETTINGS),
		onSubmit: async ({ value }) => {
			if (!user) return;
			try {
				await updateSelfMutation.mutateAsync({ ...user, display_settings: fromFormValues(value) });
				pushSuccess(
					lastChanged === 'tables'
						? settings_system_tablesUpdated()
						: settings_system_displayUpdated()
				);
			} catch {
				// The API client reports the error. Put the form and every displayed date back on
				// the last value the server accepted.
				resetForm(user.display_settings);
				displaySettings.set(user.display_settings);
			}
		}
	}));

	// Reset without saving: used for hydration and error revert.
	let suppressSave = false;
	function resetForm(settings: DisplaySettings | undefined) {
		suppressSave = true;
		form.reset(toFormValues(settings));
		suppressSave = false;
	}

	let hydrated = $state(false);
	$effect(() => {
		if (user && !hydrated) {
			resetForm(user.display_settings);
			hydrated = true;
		}
	});

	// Which card the last saved change came from, so the toast names it.
	let lastChanged: 'dateAndTime' | 'tables' = 'dateAndTime';

	// Apply the change to every date and table on screen at once, then persist it.
	function onSettingChange() {
		saveSetting('dateAndTime');
	}

	function onTableSettingChange() {
		saveSetting('tables');
	}

	function saveSetting(card: typeof lastChanged) {
		if (suppressSave || !hydrated) return;
		lastChanged = card;
		displaySettings.set(fromFormValues(form.state.values));
		void form.handleSubmit();
	}

	// A fixed sample, so the preview shows each form a date takes in the app: a table date, a
	// full timestamp and a recent-activity time.
	const previewNow = new Date();
	const previewRecent = new Date(previewNow.getTime() - 3 * 60 * 60 * 1000);
</script>

<!--
	A setting whose control fits beside its description sits on one row, so the
	whole tab fits above the fold. `labelFor` makes the title the control's label.
-->
{#snippet inlineSetting(
	title: string,
	description: string,
	labelFor: string | null,
	control: Snippet
)}
	<div class="card card-static flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
		<div class="min-w-0">
			{#if labelFor}
				<label for={labelFor} class="text-primary text-sm font-semibold">{title}</label>
			{:else}
				<h3 class="text-primary text-sm font-semibold">{title}</h3>
			{/if}
			<p class="text-tertiary mt-0.5 text-xs">{description}</p>
		</div>
		<div class="shrink-0 sm:w-80">
			{@render control()}
		</div>
	</div>
{/snippet}

{#snippet themeControl()}
	<div class="flex gap-2" role="group" aria-label={common_theme()}>
		{#each options as option (option.id)}
			<button
				type="button"
				class="btn-secondary flex flex-1 items-center justify-center gap-1.5 {themeStore.themeMode ===
				option.id
					? 'ring-primary ring-2'
					: ''}"
				onclick={() => themeStore.setTheme(option.id)}
			>
				<option.icon size={16} />
				{option.label}
			</button>
		{/each}
	</div>
{/snippet}

{#snippet densityControl()}
	<form.Field name="table_density" listeners={{ onChange: onTableSettingChange }}>
		{#snippet children(field)}
			<!-- Labelled by the card title, so no second label above the select. -->
			<SelectInput id="display-table-density" label="" options={tableDensityOptions} {field} />
		{/snippet}
	</form.Field>
{/snippet}

<div class="flex h-full flex-col gap-4 overflow-y-auto p-6">
	{@render inlineSetting(common_theme(), settings_system_themeDesc(), null, themeControl)}

	{@render inlineSetting(
		common_tables(),
		settings_system_tablesDesc(),
		'display-table-density',
		densityControl
	)}

	<CollapsibleCard
		title={settings_system_dateAndTime()}
		description={settings_system_dateAndTimeDesc()}
		expanded={false}
	>
		<p class="text-secondary text-sm">
			{settings_system_preview({
				date: formatDate(previewNow),
				timestamp: formatTimestamp(previewNow),
				relative: formatRelativeTime(previewRecent)
			})}
		</p>
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
			<form.Field name="date_order" listeners={{ onChange: onSettingChange }}>
				{#snippet children(field)}
					<SelectInput
						id="display-date-order"
						label={settings_system_dateOrder()}
						options={dateOrderOptions}
						{field}
					/>
				{/snippet}
			</form.Field>
			<form.Field name="clock" listeners={{ onChange: onSettingChange }}>
				{#snippet children(field)}
					<SelectInput id="display-clock" label={common_clock()} options={clockOptions} {field} />
				{/snippet}
			</form.Field>
			<form.Field name="time_zone" listeners={{ onChange: onSettingChange }}>
				{#snippet children(field)}
					<SelectInput
						id="display-time-zone"
						label={common_timezone()}
						options={timeZoneSelectOptions}
						{field}
					/>
				{/snippet}
			</form.Field>
			<form.Field name="week_start" listeners={{ onChange: onSettingChange }}>
				{#snippet children(field)}
					<SelectInput
						id="display-week-start"
						label={settings_system_weekStart()}
						options={weekStartOptions}
						{field}
					/>
				{/snippet}
			</form.Field>
			<form.Field name="timestamps" listeners={{ onChange: onSettingChange }}>
				{#snippet children(field)}
					<SelectInput
						id="display-timestamps"
						label={settings_system_timestamps()}
						options={timestampOptions}
						{field}
					/>
				{/snippet}
			</form.Field>
		</div>
	</CollapsibleCard>

	<div class="mt-auto flex flex-col gap-1">
		<p class="text-tertiary text-xs">{settings_system_copyright({ year: copyrightYear })}</p>
		<DocsHint
			text={settings_system_license()}
			href="https://scanopy.net/commercial"
			linkText={settings_system_licenseLinkText()}
		/>
		<DocsHint
			text={settings_system_tuxAttribution()}
			href="https://commons.wikimedia.org/wiki/File:Tux.svg"
			linkText={common_source()}
		/>
		<DocsHint
			text={settings_system_recogAttribution()}
			href="https://github.com/scanopy/scanopy/blob/main/backend/assets/recog/LICENSE"
			linkText={common_license()}
		/>
		<DocsHint
			text={settings_system_dashboardIconsAttribution()}
			href="https://github.com/scanopy/scanopy/blob/main/backend/assets/dashboard-icons/LICENSE"
			linkText={common_license()}
		/>
		<DocsHint
			text={settings_system_simpleIconsAttribution()}
			href="https://github.com/simple-icons/simple-icons/blob/develop/LICENSE.md"
			linkText={common_license()}
		/>
	</div>
</div>
