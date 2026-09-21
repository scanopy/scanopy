<script lang="ts">
	/**
	 * Give a legacy daemon (one with no 1:1 bound key) a dedicated key.
	 *
	 * The server cannot hand a key to a running daemon — the daemon reads its config once at
	 * boot and there is no server→daemon control channel — so associating a key also has to
	 * produce a reconfigure command for the operator to run on the machine.
	 *
	 * What that means differs by mode, so the copy does too:
	 *
	 * - DaemonPoll: the daemon dials the server and authenticates with the key it already has,
	 *   which this does not touch. It keeps working until the command is run.
	 * - ServerPoll: the server dials the daemon and will present the NEW key from the next poll
	 *   onwards, while the daemon still only accepts its old one — so it stops being reachable
	 *   until it is reconfigured.
	 */
	import { KeyRound } from 'lucide-svelte';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import {
		useProvisionDaemonMutation,
		useDaemonInstallCommandQuery
	} from '$lib/features/daemons/queries';
	import {
		fillInstallArtifactsKey,
		osInstallCommand,
		type OsInstallMethod
	} from '$lib/features/daemons/types/base';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import CodeContainer from '$lib/shared/components/data/CodeContainer.svelte';
	import OsSelector from './OsSelector.svelte';
	import type { DaemonOS } from '$lib/features/daemons/utils';
	import {
		common_associating,
		daemons_bindKey,
		daemons_bindKeyCta,
		daemons_bindKeyDaemonPollHelp,
		daemons_bindKeyTitle,
		daemons_bindKeyServerPollWarning,
		daemons_reconfigureCommandDaemonPoll,
		daemons_reconfigureCommandServerPoll,
		daemons_reconfigureTitle
	} from '$lib/paraglide/messages';

	let { daemon }: { daemon: Daemon } = $props();

	const provisionMutation = useProvisionDaemonMutation();

	let isServerPoll = $derived(daemon.mode === 'server_poll');
	let associating = $state(false);
	let selectedOS = $state<DaemonOS>('linux');
	let linuxMethod = $state<'binary' | 'docker'>('binary');

	// Docker is another install target alongside the OS methods.
	let selectedPlatform = $derived<OsInstallMethod | 'docker'>(
		selectedOS === 'linux' && linuxMethod === 'docker' ? 'docker' : selectedOS
	);
	let language = $derived(
		selectedPlatform === 'docker' ? 'yaml' : selectedOS === 'windows' ? 'powershell' : 'bash'
	);

	// The minted key, held only after the user associates. The install command comes from the
	// builder (with an <API_KEY> placeholder); we substitute this key into it for display.
	let mintedKey = $state<string | null>(null);

	const installCommandQuery = useDaemonInstallCommandQuery(
		() => daemon.id,
		() => ({ purpose: 'install' }),
		{ enabled: () => mintedKey != null }
	);

	let command = $derived.by(() => {
		if (!mintedKey || !installCommandQuery.data) return '';
		const filled = fillInstallArtifactsKey(installCommandQuery.data, mintedKey);
		// Docker is a first-install here, so it has a full compose.
		return selectedPlatform === 'docker'
			? (filled.docker.compose ?? '')
			: osInstallCommand(filled, selectedPlatform);
	});

	async function associate() {
		associating = true;
		try {
			const result = await provisionMutation.mutateAsync({ daemon_id: daemon.id });
			mintedKey = result.daemon_api_key;
		} catch {
			// The API client reports the failure.
		} finally {
			associating = false;
		}
	}
</script>

{#if mintedKey}
	<div class="space-y-4">
		<InlineWarning
			title={daemons_reconfigureTitle()}
			body={isServerPoll
				? daemons_reconfigureCommandServerPoll({ name: daemon.name })
				: daemons_reconfigureCommandDaemonPoll({ name: daemon.name })}
		/>

		<OsSelector
			{selectedOS}
			onOsSelect={(os) => (selectedOS = os)}
			{linuxMethod}
			onLinuxMethodChange={(method) => (linuxMethod = method)}
		>
			<CodeContainer
				{language}
				expandable={false}
				maxHeight=""
				code={command}
				preventSelect={true}
			/>
		</OsSelector>
	</div>
{:else}
	<!-- Only the icon, heading and button are centred; the explanatory bodies stay
	     left-aligned, since centred prose reads badly. -->
	<div class="space-y-4 py-4">
		<KeyRound class="text-tertiary mx-auto h-10 w-10" />
		<h3 class="text-primary text-center text-lg font-medium">{daemons_bindKey()}</h3>

		{#if isServerPoll}
			<InlineWarning title={daemons_bindKeyTitle()} body={daemons_bindKeyServerPollWarning()} />
		{:else}
			<InlineInfo title={daemons_bindKeyTitle()} body={daemons_bindKeyDaemonPollHelp()} />
		{/if}

		<div class="text-center">
			<button type="button" class="btn-primary" disabled={associating} onclick={associate}>
				{associating ? common_associating() : daemons_bindKeyCta()}
			</button>
		</div>
	</div>
{/if}
