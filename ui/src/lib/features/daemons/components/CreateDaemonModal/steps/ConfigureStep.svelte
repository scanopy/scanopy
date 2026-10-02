<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import type { FormValue } from '$lib/shared/components/forms/validators';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import InlineDanger from '$lib/shared/components/feedback/InlineDanger.svelte';

	import InlineSuccess from '$lib/shared/components/feedback/InlineSuccess.svelte';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import SelectNetwork from '$lib/features/networks/components/SelectNetwork.svelte';
	import RadioGroup from '$lib/shared/components/forms/input/RadioGroup.svelte';
	import OsSelector from '../../OsSelector.svelte';
	import type { DaemonOS } from '../../../utils';
	import { fieldDefs } from '../../../config';
	import {
		common_name,
		common_port,
		daemons_config_daemonUrl,
		daemons_config_daemonUrlHelpNoPort,
		daemons_config_mode,
		daemons_config_modeDaemonPollHelp,
		daemons_config_modeServerPollHelp,
		daemons_config_namePlaceholder,
		daemons_config_portHelpServerPoll,
		daemons_networkCannotChange,
		daemons_docsPollingMode,
		daemons_docsPollingModeLinkText,
		daemons_httpDaemonUrlWarning,
		daemons_configureReachabilityFailed,
		daemons_portReachable,
		daemons_osHelp,
		daemons_osLocked
	} from '$lib/paraglide/messages';

	interface Props {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any };
		formValues: Record<string, string | number | boolean>;
		selectedNetworkId: string;
		onNetworkChange: (id: string) => void;
		onNameInput?: () => void;
		/** Daemon identity is frozen once Configure is left — the record is provisioned from it. */
		identityLocked: boolean;
		isFirstDaemon?: boolean;
		onReachabilityChange?: (reachable: boolean | null) => void;
		reachabilityResult?: { reachable: boolean; error?: string } | null;
		/** The OS the daemon will run on. Chosen here because the integrations set up next read
		 *  files on that machine, and their paths are written for its OS. */
		selectedOS: DaemonOS;
		onOsSelect: (os: DaemonOS) => void;
		/** Fixed once integrations were created for the chosen OS. */
		osLocked?: boolean;
	}

	let {
		form,
		formValues,
		selectedNetworkId,
		onNetworkChange,
		onNameInput,
		identityLocked,
		isFirstDaemon = false,
		onReachabilityChange,
		reachabilityResult = $bindable(null),
		selectedOS,
		onOsSelect,
		osLocked = false
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
	let daemonPort = $derived(Number(formValues.daemonPort) || 60073);
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

	// Reset reachability when URL or port changes
	let prevUrlPort = $state('');
	$effect(() => {
		const key = `${daemonUrl}:${daemonPort}`;
		if (key !== prevUrlPort) {
			prevUrlPort = key;
			reachabilityResult = null;
			onReachabilityChange?.(null);
		}
	});
</script>

<div class="space-y-4">
	{#if !isFirstDaemon}
		<SelectNetwork
			{selectedNetworkId}
			onNetworkChange={(id) => onNetworkChange(id)}
			disabled={identityLocked}
			disabledReason={daemons_networkCannotChange()}
		/>
	{/if}

	<div class="space-y-2">
		<OsSelector {selectedOS} {onOsSelect} showMethods={false} osDisabled={osLocked} />
		<p class="text-muted text-xs">{osLocked ? daemons_osLocked() : daemons_osHelp()}</p>
	</div>

	<!-- Name -->
	<div oninput={() => onNameInput?.()}>
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
	</div>

	<!-- Mode -->
	<form.Field name={modeDef.id}>
		{#snippet children(field: AnyFieldApi)}
			<RadioGroup
				label={daemons_config_mode()}
				id="daemon-mode"
				{field}
				options={[
					{
						value: 'daemon_poll',
						label: (modeDef.options ?? [])[0]?.label() ?? 'Daemon Poll',
						helpText: daemons_config_modeDaemonPollHelp()
					},
					{
						value: 'server_poll',
						label: (modeDef.options ?? [])[1]?.label() ?? 'Server Poll',
						helpText: daemons_config_modeServerPollHelp()
					}
				]}
				disabled={identityLocked}
			/>
		{/snippet}
	</form.Field>

	<DocsHint
		text={daemons_docsPollingMode()}
		href="https://scanopy.net/docs/setting-up-daemons/planning-daemon-deployment/#choosing-a-polling-mode"
		linkText={daemons_docsPollingModeLinkText()}
	/>

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

		<!-- Reachability result (driven by parent) -->
		{#if reachabilityResult}
			{#if reachabilityResult.reachable}
				<InlineSuccess title={daemons_portReachable()} />
			{:else}
				<InlineDanger
					title={reachabilityResult.error ?? 'Port is not reachable'}
					body={daemons_configureReachabilityFailed()}
				/>
			{/if}
		{/if}
	{/if}
</div>
