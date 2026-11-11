<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import type { FormValue } from '#lib/shared/components/forms/validators.ts';
	import TextInput from '#lib/shared/components/forms/input/TextInput.svelte';
	import SelectInput from '#lib/shared/components/forms/input/SelectInput.svelte';
	import Checkbox from '#lib/shared/components/forms/input/Checkbox.svelte';
	import DocsHint from '#lib/shared/components/feedback/DocsHint.svelte';
	import InlineInfo from '#lib/shared/components/feedback/InlineInfo.svelte';
	import CollapsibleCard from '#lib/shared/components/data/CollapsibleCard.svelte';
	import {
		daemons_docsConfigOptions,
		daemons_docsConfigOptionsLinkText
	} from '#lib/paraglide/messages.js';
	import { fieldDefs, sectionDefs } from '../../../config';

	interface Props {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any };
		formValues: Record<string, string | number | boolean>;
		selectedOS?: string;
		linuxMethod?: string;
	}

	let { form, formValues, selectedOS = 'linux', linuxMethod = 'binary' }: Props = $props();

	// Dynamic placeholder for logFile field based on selected OS
	let logFilePlaceholder = $derived.by(() => {
		const name = (formValues.name as string) || 'scanopy-daemon';
		if (selectedOS === 'linux' && linuxMethod === 'docker') {
			return `/var/log/scanopy/${name}.log (mounted)`;
		}
		switch (selectedOS) {
			case 'linux':
				return `/var/log/scanopy/${name}.log`;
			case 'macos':
				return `~/Library/Logs/scanopy/${name}.log`;
			case 'windows':
				return `%ProgramData%\\scanopy\\${name}.log`;
			default:
				return `/var/log/scanopy/${name}.log`;
		}
	});

	const advancedFieldDefs = fieldDefs.filter((d) => d.section);

	// Get unique section names in order of appearance (compare by return value, not function reference)
	const sectionNames = [...new Set(advancedFieldDefs.map((d) => d.section!()))];

	// Group advanced fields by section (compare by return value), booleans sorted to end
	const advancedSections = sectionNames.map((name) => ({
		name: () => name,
		fields: advancedFieldDefs
			.filter((d) => d.section!() === name)
			.sort((a, b) => (a.type === 'boolean' ? 1 : 0) - (b.type === 'boolean' ? 1 : 0))
	}));

	// Get validators for a field
	function getValidators(fieldId: string) {
		const def = fieldDefs.find((d) => d.id === fieldId);
		if (!def?.validators || def.validators.length === 0) return {};

		return {
			onBlur: ({ value }: { value: FormValue }) => {
				for (const validator of def.validators!) {
					const error = validator(value);
					if (error) return error;
				}
				return undefined;
			}
		};
	}
</script>

<div class="space-y-6">
	<InlineInfo
		title=""
		body="Changes you make here will be reflected in the install command and can be edited in the daemon's config file after installation."
		dismissableKey="daemon-advanced-hint"
	/>

	<DocsHint
		text={daemons_docsConfigOptions()}
		href="https://scanopy.net/docs/reference/daemon-configuration/"
		linkText={daemons_docsConfigOptionsLinkText()}
	/>

	{#each advancedSections as section (section.name)}
		{@const sectionName = section.name()}
		{@const sectionDef = sectionDefs[sectionName]}
		{@const description = sectionDef?.description()}
		<CollapsibleCard title={sectionName} {description} expanded={false}>
			{#if sectionDef?.docsHint}
				<DocsHint
					text={sectionDef.docsHint.text()}
					href={sectionDef.docsHint.href}
					linkText={sectionDef.docsHint.linkText()}
				/>
			{/if}
			<div class="grid grid-cols-1 gap-3 sm:grid-cols-2 sm:gap-4">
				{#each section.fields as def (def.id)}
					{#if !def.showWhen || def.showWhen(formValues)}
						{#if def.docsOnly}
							<div></div>
						{:else if def.type === 'string'}
							<form.Field name={def.id} validators={getValidators(def.id)}>
								{#snippet children(field: AnyFieldApi)}
									<TextInput
										label={def.label()}
										{field}
										id={def.id}
										placeholder={def.id === 'logFile'
											? logFilePlaceholder
											: String(
													typeof def.placeholder === 'function'
														? def.placeholder()
														: (def.placeholder ?? '')
												)}
										helpText={def.helpText()}
									/>
								{/snippet}
							</form.Field>
						{:else if def.type === 'number'}
							<form.Field name={def.id} validators={getValidators(def.id)}>
								{#snippet children(field: AnyFieldApi)}
									<TextInput
										label={def.label()}
										{field}
										id={def.id}
										type="number"
										placeholder={String(
											typeof def.placeholder === 'function'
												? def.placeholder()
												: (def.placeholder ?? '')
										)}
										helpText={def.helpText()}
									/>
								{/snippet}
							</form.Field>
						{:else if def.type === 'select'}
							<form.Field name={def.id}>
								{#snippet children(field: AnyFieldApi)}
									<SelectInput
										label={def.label()}
										{field}
										id={def.id}
										options={(def.options ?? []).map((opt) => ({
											value: opt.value,
											label: opt.label()
										}))}
										helpText={def.helpText()}
									/>
								{/snippet}
							</form.Field>
						{:else if def.type === 'boolean'}
							<form.Field name={def.id}>
								{#snippet children(field: AnyFieldApi)}
									<Checkbox label={def.label()} {field} id={def.id} helpText={def.helpText()} />
								{/snippet}
							</form.Field>
						{/if}
					{/if}
				{/each}
			</div>
		</CollapsibleCard>
	{/each}
</div>
