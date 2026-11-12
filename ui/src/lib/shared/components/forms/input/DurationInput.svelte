<script lang="ts">
	import FormField from './FormField.svelte';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import { combineToHours, splitHours } from '#lib/shared/utils/duration.ts';
	import { common_days, common_hours } from '#lib/paraglide/messages.js';

	interface Props {
		label: string;
		/** TanStack field holding the duration as a total number of hours. Written to, never read back. */
		field: AnyFieldApi;
		id: string;
		required?: boolean;
		helpText?: string;
		disabled?: boolean;
		/**
		 * Duration to populate the boxes with, in hours. Reactive: when it
		 * changes (the form is reset for a different entity) the boxes re-seed.
		 * Supplied as a prop rather than read from `field.state.value` because
		 * that is not tracked by Svelte 5 reactivity, and because it removes any
		 * dependence on whether mount or `form.reset` happens first.
		 */
		initialHours?: number | null;
		/** Placeholder duration, in hours, shown when both boxes are empty. */
		placeholderHours?: number | null;
	}

	let {
		label,
		field,
		id,
		required = false,
		helpText = '',
		disabled = false,
		initialHours = null,
		placeholderHours = null
	}: Props = $props();

	let hasErrors = $derived(field.state.meta.isTouched && field.state.meta.errors.length > 0);

	// `field.state.value` is NOT tracked by Svelte 5's `$derived` (documented
	// TanStack Form pitfall), so the two boxes cannot be rendered from it.
	// These `$state` vars are the display source of truth; the field is written
	// through on every edit and never read back for rendering.
	let days = $state<number | null>(null);
	let hours = $state<number | null>(null);

	// Re-seed whenever the incoming duration changes — i.e. the form was reset
	// for a different entity. `initialHours` is a primitive, so an unrelated
	// refetch that yields the same value does not re-run this and cannot
	// clobber an in-progress edit.
	$effect(() => {
		const seed = splitHours(initialHours);
		days = seed.days;
		hours = seed.hours;
	});

	function sync() {
		field.handleChange(combineToHours(days, hours));
	}

	function onDaysInput(e: Event & { currentTarget: HTMLInputElement }) {
		const raw = e.currentTarget.value;
		days = raw === '' ? null : Math.max(0, Number(raw));
		sync();
	}

	function onHoursInput(e: Event & { currentTarget: HTMLInputElement }) {
		const raw = e.currentTarget.value;
		hours = raw === '' ? null : Math.max(0, Number(raw));
		sync();
	}

	// The default is only a meaningful suggestion while the whole field is
	// empty. Once either box has a value the pair reads as one duration, so the
	// other box must show 0 — otherwise entering "6" in hours renders as
	// "[28] days [6] hours" and implies 28d 6h when the value is 6 hours.
	let hasAnyValue = $derived(days !== null || hours !== null);
	let defaultSplit = $derived(splitHours(placeholderHours));
	let placeholder = $derived(
		hasAnyValue ? { days: 0, hours: 0 } : { days: defaultSplit.days, hours: defaultSplit.hours }
	);
</script>

<FormField {label} {field} {required} {helpText} {id}>
	<div class="flex items-center gap-2">
		<div class="flex items-center gap-1.5">
			<input
				{id}
				type="number"
				min="0"
				value={days ?? ''}
				onblur={() => field.handleBlur()}
				oninput={onDaysInput}
				placeholder={placeholder.days === null ? '' : String(placeholder.days)}
				{disabled}
				class="input-field w-20"
				class:input-field-error={hasErrors}
				aria-label={common_days()}
			/>
			<span class="text-sm text-secondary">{common_days()}</span>
		</div>
		<div class="flex items-center gap-1.5">
			<input
				id={`${id}-hours`}
				type="number"
				min="0"
				max="23"
				value={hours ?? ''}
				onblur={() => field.handleBlur()}
				oninput={onHoursInput}
				placeholder={placeholder.hours === null ? '' : String(placeholder.hours)}
				{disabled}
				class="input-field w-20"
				class:input-field-error={hasErrors}
				aria-label={common_hours()}
			/>
			<span class="text-sm text-secondary">{common_hours()}</span>
		</div>
	</div>
</FormField>
