<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import type { FormValue } from '$lib/shared/components/forms/validators';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import SelectNetwork from '$lib/features/networks/components/SelectNetwork.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import {
		SimpleOptionDisplay,
		type SimpleOption
	} from '$lib/shared/components/forms/selection/display/SimpleOptionDisplay';
	import { ArrowUpCircle } from 'lucide-svelte';
	import { openModal } from '$lib/shared/stores/modal-registry';
	import { fieldDefs } from '../../../config';
	import {
		common_name,
		common_port,
		daemons_activateAfterCreation,
		daemons_activateAfterCreationBody,
		daemons_config_daemonUrl,
		daemons_config_daemonUrlHelpNoPort,
		daemons_config_mode,
		daemons_config_namePlaceholder,
		daemons_config_portHelpServerPoll,
		daemons_networkCannotChange,
		daemons_portForwardingHint,
		daemons_httpDaemonUrlWarning
	} from '$lib/paraglide/messages';

	interface Props {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any };
		formValues: Record<string, string | number | boolean>;
		selectedNetworkId: string;
		onNetworkChange: (id: string) => void;
		hasDaemonPoll: boolean;
		keySet: boolean;
		onboardingMode: boolean;
	}

	let {
		form,
		formValues,
		selectedNetworkId,
		onNetworkChange,
		hasDaemonPoll,
		keySet,
		onboardingMode
	}: Props = $props();

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

	let nameDef = fieldDefs.find((d) => d.id === 'name')!;
	let modeDef = fieldDefs.find((d) => d.id === 'mode')!;
	let daemonUrlDef = fieldDefs.find((d) => d.id === 'daemonUrl')!;
	let daemonPortDef = fieldDefs.find((d) => d.id === 'daemonPort')!;

	let isServerPoll = $derived(formValues.mode === 'server_poll');
	let daemonUrl = $derived(String(formValues.daemonUrl ?? ''));
	let showHttpWarning = $derived.by(() => {
		try {
			const parsed = new URL(daemonUrl);
			if (parsed.protocol !== 'http:') return false;
			const host = parsed.hostname;
			if (host === 'localhost' || host === '127.0.0.1' || host === '::1') return false;
			// Suppress while user is still typing a localhost address
			if ('localhost'.startsWith(host) || '127.0.0.1'.startsWith(host)) return false;
			return true;
		} catch {
			// URL is incomplete/invalid — don't show warning yet
			return false;
		}
	});
</script>

<div class="space-y-4">
	{#if onboardingMode}
		<InlineInfo
			title={daemons_activateAfterCreation()}
			body={daemons_activateAfterCreationBody()}
		/>
	{:else}
		<SelectNetwork
			{selectedNetworkId}
			onNetworkChange={(id) => onNetworkChange(id)}
			disabled={keySet}
			disabledReason={daemons_networkCannotChange()}
		/>
	{/if}

	<!-- Name -->
	<form.Field name={nameDef.id} validators={getValidators(nameDef.id)}>
		{#snippet children(field: AnyFieldApi)}
			<TextInput
				label={common_name()}
				{field}
				id={nameDef.id}
				placeholder={daemons_config_namePlaceholder()}
				required={true}
			/>
		{/snippet}
	</form.Field>

	<!-- Mode -->
	<form.Field name={modeDef.id}>
		{#snippet children(field: AnyFieldApi)}
			<RichSelect
				label={daemons_config_mode()}
				selectedValue={String(field.state.value ?? '')}
				disabled={keySet}
				options={(modeDef.options ?? []).map((opt): SimpleOption => {
					const needsUpgrade = opt.value === 'daemon_poll' && !hasDaemonPoll;
					return {
						value: opt.value,
						label: opt.label(),
						description:
							opt.value === 'daemon_poll'
								? 'Daemon connects to server; works behind NAT/firewall without opening ports'
								: 'Server connects to daemon; requires providing Daemon URL',
						disabled: needsUpgrade,
						tags: needsUpgrade ? [{ label: 'Upgrade', color: 'Yellow', icon: ArrowUpCircle }] : []
					};
				})}
				onSelect={(value) => field.handleChange(value)}
				onDisabledClick={() => openModal('billing-plan')}
				displayComponent={SimpleOptionDisplay}
			/>
		{/snippet}
	</form.Field>

	<!-- Server Poll: URL + Port side-by-side with port forwarding hint -->
	{#if isServerPoll}
		<div class="grid grid-cols-[1fr_auto] gap-4">
			<form.Field name={daemonUrlDef.id} validators={getValidators(daemonUrlDef.id)}>
				{#snippet children(field: AnyFieldApi)}
					<TextInput
						label={daemons_config_daemonUrl()}
						{field}
						id={daemonUrlDef.id}
						placeholder={String(
							typeof daemonUrlDef.placeholder === 'function'
								? daemonUrlDef.placeholder()
								: (daemonUrlDef.placeholder ?? '')
						)}
						required={true}
						helpText={daemons_config_daemonUrlHelpNoPort()}
					/>
				{/snippet}
			</form.Field>

			<div class="w-48">
				<form.Field name={daemonPortDef.id} validators={getValidators(daemonPortDef.id)}>
					{#snippet children(field: AnyFieldApi)}
						<TextInput
							label={common_port()}
							{field}
							id={daemonPortDef.id}
							type="number"
							placeholder={String(daemonPortDef.placeholder ?? '')}
							helpText={daemons_config_portHelpServerPoll()}
						/>
					{/snippet}
				</form.Field>
			</div>
		</div>

		{#if showHttpWarning}
			<InlineWarning title="" body={daemons_httpDaemonUrlWarning()} />
		{/if}

		<InlineInfo title="" body={daemons_portForwardingHint()} />
	{/if}
</div>
