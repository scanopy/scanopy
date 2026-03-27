<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import type { FormValue } from '$lib/shared/components/forms/validators';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import InlineSuccess from '$lib/shared/components/feedback/InlineSuccess.svelte';
	import CollapsibleCard from '$lib/shared/components/data/CollapsibleCard.svelte';
	import SegmentedControl from '$lib/shared/components/forms/SegmentedControl.svelte';
	import {
		common_disabled,
		common_docker,
		common_proxy,
		daemons_docsConfigOptions,
		daemons_docsConfigOptionsLinkText,
		daemons_dockerDescription,
		daemons_dockerLocalSocket,
		daemons_dockerProxyCreated,
		daemons_dockerDisabledHelp,
		daemons_dockerLocalSocketHelp,
		daemons_dockerProxyHelp,
		daemons_dockerGoToScanCredentials,
		daemons_dockerProxyWizardCta
	} from '$lib/paraglide/messages';
	import { fieldDefs, sectionDefs } from '../../../config';

	interface Props {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any };
		formValues: Record<string, string | number | boolean>;
		dockerMode?: string;
		hasDockerProxyCredential?: boolean;
		onNavigateToCredentialWizard?: () => void;
	}

	let {
		form,
		formValues,
		dockerMode = $bindable('local_socket'),
		hasDockerProxyCredential = false,
		onNavigateToCredentialWizard
	}: Props = $props();

	let dockerModeHelp = $derived.by(() => {
		switch (dockerMode) {
			case 'disabled':
				return daemons_dockerDisabledHelp();
			case 'local_socket':
				return daemons_dockerLocalSocketHelp();
			case 'proxy':
				return daemons_dockerProxyHelp();
			default:
				return '';
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
		body="Changes you make here will be reflected in the install commands."
		dismissableKey="daemon-advanced-hint"
	/>

	<DocsHint
		text={daemons_docsConfigOptions()}
		href="https://scanopy.net/docs/reference/daemon-configuration/"
		linkText={daemons_docsConfigOptionsLinkText()}
	/>

	<!-- Docker Section -->
	<CollapsibleCard
		title={common_docker()}
		description={daemons_dockerDescription()}
		expanded={false}
	>
		<div class="space-y-4">
			<SegmentedControl
				options={[
					{ value: 'disabled', label: common_disabled() },
					{ value: 'local_socket', label: daemons_dockerLocalSocket() },
					{ value: 'proxy', label: common_proxy() }
				]}
				selected={hasDockerProxyCredential ? 'proxy' : dockerMode}
				onchange={(v) => {
					dockerMode = v;
				}}
				disabled={hasDockerProxyCredential}
				size="md"
			/>

			<p class="text-muted text-xs">{dockerModeHelp}</p>

			{#if hasDockerProxyCredential}
				<InlineSuccess title={daemons_dockerProxyCreated()} />
			{:else if dockerMode === 'proxy' && onNavigateToCredentialWizard}
				<div class="space-y-2">
					<p class="text-secondary text-sm">{daemons_dockerProxyWizardCta()}</p>
					<button type="button" class="btn-primary text-sm" onclick={onNavigateToCredentialWizard}>
						{daemons_dockerGoToScanCredentials()}
					</button>
				</div>
			{/if}
		</div>
	</CollapsibleCard>

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
			<div class="grid grid-cols-2 gap-4">
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
										placeholder={String(
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
