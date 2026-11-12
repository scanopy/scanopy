<script lang="ts">
	import { Monitor, Sun, Moon, ChevronRight } from '@lucide/svelte';
	import { createForm } from '@tanstack/svelte-form';
	import { themeStore } from '#lib/shared/stores/theme.svelte.ts';
	import { VERSION } from '#lib/version.ts';
	import {
		browserTimeZone,
		DEFAULT_DISPLAY_SETTINGS,
		displaySettings,
		timeZoneOptions
	} from '#lib/shared/stores/display-settings.svelte.ts';
	import { useCurrentUserQuery } from '#lib/features/auth/queries.ts';
	import { useUpdateSelfMutation } from '#lib/features/users/queries.ts';
	import { formatDate, formatRelativeTime, formatTimestamp } from '#lib/shared/utils/formatting.ts';
	import { pushSuccess } from '#lib/shared/stores/feedback.ts';
	import SelectInput from '#lib/shared/components/forms/input/SelectInput.svelte';
	import DocsHint from '#lib/shared/components/feedback/DocsHint.svelte';
	import CollapsibleCard from '#lib/shared/components/data/CollapsibleCard.svelte';
	import logoCredits from '#lib/data/logo-credits.json';
	import type { Snippet } from 'svelte';
	import type { components } from '#lib/api/schema';
	import {
		common_browserDefault,
		common_system,
		common_theme,
		common_timezone,
		common_light,
		common_dark,
		common_monday,
		common_sunday,
		common_clock,
		common_12Hour,
		common_24Hour,
		common_comfortable,
		common_compact,
		common_tables,
		settings_system_copyright,
		settings_system_dateAndTime,
		settings_system_dateOrder,
		settings_system_dateOrderDayFirst,
		settings_system_dateOrderIso,
		settings_system_dateOrderMonthFirst,
		settings_system_displayUpdated,
		settings_system_iconCredits,
		settings_system_iconCreditsDescription,
		settings_system_iconCreditsTrademark,
		settings_system_license,
		settings_system_licenseLinkText,
		settings_system_openSourceLicenses,
		settings_system_openSourceLicensesLink,
		settings_system_preview,
		settings_system_tablesDesc,
		settings_system_tablesUpdated,
		settings_system_themeDesc,
		settings_system_timeZoneBrowser,
		settings_system_timestamps,
		settings_system_timestampsAbsolute,
		settings_system_timestampsRelative,
		settings_system_version,
		settings_system_weekStart
	} from '#lib/paraglide/messages.js';

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
	let creditsOpen = $state(false);

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
	<div class="card-static card flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
		<div class="min-w-0">
			{#if labelFor}
				<label for={labelFor} class="text-sm font-semibold text-primary">{title}</label>
			{:else}
				<h3 class="text-sm font-semibold text-primary">{title}</h3>
			{/if}
			<p class="mt-0.5 text-xs text-tertiary">{description}</p>
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

	<!--
		The preview stands in for a description: it shows the current formats while
		the card is collapsed and updates live as the selects change.
	-->
	<CollapsibleCard
		title={settings_system_dateAndTime()}
		description={settings_system_preview({
			date: formatDate(previewNow),
			timestamp: formatTimestamp(previewNow),
			relative: formatRelativeTime(previewRecent)
		})}
		expanded={false}
	>
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
		<p class="text-xs text-tertiary">{settings_system_version({ version: VERSION })}</p>
		<p class="text-xs text-tertiary">{settings_system_copyright({ year: copyrightYear })}</p>
		<DocsHint
			text={settings_system_license()}
			href="https://scanopy.net/commercial"
			linkText={settings_system_licenseLinkText()}
		/>
		<!-- Generated by the UI build (vite.config.ts) from every crate, npm package and data file. -->
		<DocsHint
			text={settings_system_openSourceLicenses()}
			href="/third-party-licenses.txt"
			linkText={settings_system_openSourceLicensesLink()}
		/>
		<!-- Every logo source and its terms, from the backend's `logos.rs`. -->
		<p class="text-xs text-tertiary">
			{settings_system_iconCreditsDescription()}
			<button
				type="button"
				class="inline-flex items-center text-blue-500 hover:underline"
				aria-expanded={creditsOpen}
				onclick={() => (creditsOpen = !creditsOpen)}
			>
				{settings_system_iconCredits()}<ChevronRight
					class="ml-0.5 h-3 w-3 transition-transform {creditsOpen ? 'rotate-90' : ''}"
				/>
			</button>
		</p>
		{#if creditsOpen}
			<ul class="flex flex-col divide-y divide-gray-200 pl-3 dark:divide-gray-700">
				{#each logoCredits as credit (credit.name)}
					<li class="flex flex-col gap-0.5 py-2 text-xs">
						<span>
							<a
								href={credit.homepage}
								target="_blank"
								rel="external noopener noreferrer"
								class="font-medium text-blue-500 hover:underline">{credit.name}</a
							>
							<span class="text-tertiary">
								· {credit.copyright} ·
								{#if credit.license && credit.license_url}
									<a
										href={credit.license_url}
										target="_blank"
										rel="external noopener noreferrer"
										class="text-blue-500 hover:underline">{credit.license}</a
									>
								{:else}
									{credit.license ?? settings_system_iconCreditsTrademark()}
								{/if}
							</span>
						</span>
						<span class="text-tertiary">
							{credit.used_for.join(', ')}
						</span>
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</div>
