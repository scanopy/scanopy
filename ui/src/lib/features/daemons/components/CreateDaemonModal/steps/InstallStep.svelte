<script lang="ts">
	import CodeContainer from '$lib/shared/components/data/CodeContainer.svelte';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import InlineDanger from '$lib/shared/components/feedback/InlineDanger.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import InlineSuccess from '$lib/shared/components/feedback/InlineSuccess.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import SupportOptions from '$lib/features/support/SupportOptions.svelte';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import type { DaemonOS } from '../../../utils';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { useTestReachabilityMutation, useRetryDaemonConnectionMutation } from '../../../queries';
	import AnimatedProgressBar from '$lib/features/discovery/components/cards/AnimatedProgressBar.svelte';
	import ProgressTrack from '$lib/shared/components/data/ProgressTrack.svelte';
	import OsSelector from '../../OsSelector.svelte';
	import {
		Loader2,
		CheckCircle2,
		AlertTriangle,
		SlidersHorizontal,
		KeyRound,
		ExternalLink
	} from 'lucide-svelte';
	import type { DaemonConnectionStatus } from '../../../stores/daemon-setup';
	import {
		common_advanced,
		daemons_credentialWizardButton,
		daemons_dockerLinuxOnly,
		daemons_dockerLinuxOnlyBody,
		daemons_docsMacvlan,
		daemons_docsMacvlanLinkText,
		daemons_docsMultiVlan,
		daemons_docsMultiVlanLinkText,
		daemons_fixValidationErrors,
		daemons_fixValidationErrorsBody,
		daemons_wslWarning,
		daemons_wslWarningBody,
		common_firstDiscoveryEmailHint
	} from '$lib/paraglide/messages';

	type LinuxMethod = 'binary' | 'docker';

	interface Props {
		selectedOS: DaemonOS;
		onOsSelect: (os: DaemonOS) => void;
		linuxMethod?: LinuxMethod;
		onLinuxMethodChange?: (method: LinuxMethod) => void;
		runCommand: string;
		dockerCompose: string;
		hasErrors: boolean;
		isFirstDaemon?: boolean;
		connectionStatus?: DaemonConnectionStatus;
		onViewDiscovery?: () => void;
		hasEmailSupport?: boolean;
		showTroubleshootingPanel?: boolean;
		onAdvanced?: (() => void) | null;
		onCredentialWizard?: (() => void) | null;
		daemonMode?: string;
		daemonUrl?: string;
		provisionedDaemonId?: string;
		onTroubleshoot?: () => void;
		onStartWaitingTimeout?: () => void;
	}

	let {
		selectedOS,
		onOsSelect,
		linuxMethod = 'binary',
		onLinuxMethodChange,
		runCommand,
		dockerCompose,
		hasErrors,
		isFirstDaemon = false,
		connectionStatus = 'idle',
		onViewDiscovery,
		hasEmailSupport = false,
		showTroubleshootingPanel = false,
		onAdvanced = null,
		onCredentialWizard = null,
		daemonMode = 'daemon_poll',
		daemonUrl = '',
		provisionedDaemonId = '',
		onTroubleshoot,
		onStartWaitingTimeout
	}: Props = $props();

	const configQuery = useConfigQuery();
	let hasEmail = $derived(configQuery.data?.has_email_service ?? false);

	const windowsDownloadUrl =
		'https://github.com/scanopy/scanopy/releases/latest/download/scanopy-daemon-windows-amd64.exe';
	const windowsInstallCommand = `Invoke-WebRequest -Uri "${windowsDownloadUrl}" -OutFile "scanopy-daemon-windows-amd64.exe"`;
	const installScript = `bash -c "$(curl -fsSL https://raw.githubusercontent.com/scanopy/scanopy/refs/heads/main/install.sh)"`;

	// Combined install commands
	let combinedLinuxMacCommand = $derived(`${installScript} && ${runCommand}`);
	let combinedWindowsCommand = $derived(`${windowsInstallCommand}; ${runCommand}`);

	// ServerPoll health check
	const healthCheckMutation = useTestReachabilityMutation();
	const retryConnectionMutation = useRetryDaemonConnectionMutation();
	let healthResult = $state<{ reachable: boolean; health?: boolean; error?: string } | null>(null);
	let isCheckingHealth = $state(false);
	let isServerPoll = $derived(daemonMode === 'server_poll');

	async function handleHealthCheck() {
		if (!daemonUrl) return;
		isCheckingHealth = true;
		try {
			const result = await healthCheckMutation.mutateAsync({
				url: daemonUrl,
				check_health: true
			});
			healthResult = {
				reachable: result.reachable,
				health: result.health ?? undefined,
				error: result.error ?? undefined
			};
			// If reachable and healthy, reset unreachable flag so server resumes polling
			if (result.reachable && result.health && provisionedDaemonId) {
				retryConnectionMutation.mutate(provisionedDaemonId);
				// Start the 60s timeout now that we know the daemon is reachable
				onStartWaitingTimeout?.();
			}
		} catch {
			healthResult = { reachable: false, error: 'Failed to test reachability' };
		} finally {
			isCheckingHealth = false;
		}
	}

	// Clear health result when transitioning to connected
	$effect(() => {
		if (connectionStatus === 'connected') {
			healthResult = null;
		}
	});

	// ServerPoll: auto-run health check when entering waiting state
	let prevConnectionStatus = $state<DaemonConnectionStatus>('idle');
	$effect(() => {
		if (
			prevConnectionStatus !== 'waiting' &&
			connectionStatus === 'waiting' &&
			isServerPoll &&
			daemonUrl
		) {
			handleHealthCheck();
		}
		// Also auto-run on trouble entry (user hit 60s timeout)
		if (
			prevConnectionStatus !== 'trouble' &&
			connectionStatus === 'trouble' &&
			isServerPoll &&
			daemonUrl
		) {
			handleHealthCheck();
		}
		prevConnectionStatus = connectionStatus;
	});

	// ServerPoll waiting state: health check passed = show progress bar
	let serverPollReachable = $derived(
		isServerPoll && healthResult?.reachable === true && healthResult?.health === true
	);

	function handleOsSelect(os: DaemonOS) {
		onOsSelect(os);
		trackEvent('daemon_install_os_selected', { os });
	}

	function handleCopy(context: string) {
		trackEvent('daemon_install_command_copied', { os: selectedOS, context });
	}

	// Show waiting UI when connection status is not idle
	let showWaitingUI = $derived(connectionStatus !== 'idle');

	// Progress bar for waiting state (0-100 over 60 seconds)
	const WAIT_DURATION_MS = 60_000;
	let waitingProgress = $state(0);
	let waitingStartTime = $state<number | null>(null);
	$effect(() => {
		// DaemonPoll: start progress on waiting. ServerPoll: start after health check passes.
		const shouldProgress = connectionStatus === 'waiting' && (!isServerPoll || serverPollReachable);
		if (shouldProgress) {
			waitingStartTime = Date.now();
			waitingProgress = 0;
			const interval = setInterval(() => {
				const elapsed = Date.now() - (waitingStartTime ?? Date.now());
				waitingProgress = Math.min(100, (elapsed / WAIT_DURATION_MS) * 100);
				if (waitingProgress >= 100) clearInterval(interval);
			}, 500);
			return () => clearInterval(interval);
		}
	});

	// Auto-scroll to troubleshooting panel when first shown
	let troubleshootingRef: HTMLDivElement | undefined = $state(undefined);
	let hasScrolledToTroubleshooting = $state(false);
	$effect(() => {
		if (showTroubleshootingPanel && troubleshootingRef && !hasScrolledToTroubleshooting) {
			troubleshootingRef.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
			hasScrolledToTroubleshooting = true;
		}
	});
</script>

<div class="space-y-4">
	{#if showWaitingUI}
		<!-- Waiting / Connected / Trouble states -->
		{#if connectionStatus === 'waiting'}
			<div class="flex flex-col items-center gap-4 py-8 text-center">
				{#if isServerPoll}
					<!-- ServerPoll: show health check result, then progress bar if reachable -->
					{#if isCheckingHealth}
						<Loader2 class="text-primary h-10 w-10 animate-spin" />
						<p class="text-secondary text-sm">Testing connection to your daemon...</p>
					{:else if serverPollReachable}
						<div class="flex w-full max-w-xs items-center gap-2">
							<ProgressTrack class="flex-1">
								<AnimatedProgressBar progress={waitingProgress} />
							</ProgressTrack>
							<span class="text-secondary text-xs tabular-nums">{Math.round(waitingProgress)}%</span
							>
						</div>
						<div class="max-w-md text-left">
							<InlineSuccess
								title="Daemon is reachable and healthy — the server will register it on its next polling cycle"
							/>
						</div>
					{:else if healthResult}
						<!-- Health check failed -->
						<h3 class="text-primary text-base font-semibold">Connection test failed</h3>
						<div class="max-w-md text-left">
							{#if healthResult.reachable}
								<InlineWarning
									title="Port open but health check failed"
									body="The port is open but the daemon isn't responding to health checks. It may still be starting — try again in a moment. If this persists, <a href='https://scanopy.net/docs/setting-up-daemons/troubleshooting-setup/' target='_blank' class='underline'>check the daemon logs</a>."
								/>
							{:else}
								<InlineDanger
									title={healthResult.error ?? 'Not reachable'}
									body="The daemon may not be running, or the port may no longer be reachable. Verify the install command completed and the daemon process started. If the host has a firewall, check that the port is open and forwarded."
								/>
							{/if}
						</div>
						<button
							type="button"
							class="btn-primary text-sm"
							disabled={isCheckingHealth}
							onclick={handleHealthCheck}
						>
							Test Daemon Reachability
						</button>
						<button
							type="button"
							class="inline-flex items-center gap-1 text-sm text-blue-400 hover:text-blue-300"
							onclick={() => onTroubleshoot?.()}
						>
							Troubleshoot
							<ExternalLink class="h-3.5 w-3.5" />
						</button>
					{/if}
				{:else}
					<!-- DaemonPoll: progress bar immediately -->
					<div class="flex w-full max-w-xs items-center gap-2">
						<ProgressTrack class="flex-1">
							<AnimatedProgressBar progress={waitingProgress} />
						</ProgressTrack>
						<span class="text-secondary text-xs tabular-nums">{Math.round(waitingProgress)}%</span>
					</div>
					<div>
						<h3 class="text-primary text-base font-semibold">
							Waiting for your daemon to connect to the server...
						</h3>
						<p class="text-secondary mt-1 text-sm">
							This usually takes less than a minute. Make sure the daemon is running.
						</p>
					</div>
				{/if}
			</div>
			{#if showTroubleshootingPanel}
				<div bind:this={troubleshootingRef} class="pt-4">
					<p class="text-secondary mb-3 text-sm font-medium">Need help?</p>
					<SupportOptions isTroubleshooting={true} {hasEmailSupport} />
				</div>
			{/if}
		{:else if connectionStatus === 'connected'}
			<div class="flex flex-col items-center gap-4 py-8 text-center">
				<CheckCircle2 class="h-10 w-10 text-green-400" />
				<div>
					<h3 class="text-primary text-base font-semibold">Your daemon is connected!</h3>
					<p class="text-secondary mt-1 text-sm">
						Discovery has automatically started. You can view the progress in real time.
					</p>
				</div>
				<button type="button" class="btn-primary" onclick={() => onViewDiscovery?.()}>
					View Discovery Progress
				</button>
				{#if hasEmail && isFirstDaemon}
					<p class="text-secondary text-sm">
						{common_firstDiscoveryEmailHint()}
					</p>
				{/if}
			</div>
		{:else if connectionStatus === 'trouble'}
			<div class="flex flex-col items-center gap-4 py-8 text-center">
				<AlertTriangle class="h-10 w-10 text-yellow-400" />
				<div>
					{#if isServerPoll}
						<h3 class="text-primary text-base font-semibold">
							The server hasn't been able to connect to your daemon
						</h3>
					{:else}
						<h3 class="text-primary text-base font-semibold">
							Your daemon hasn't connected to this server
						</h3>
					{/if}
				</div>
				{#if isServerPoll}
					<ol class="text-secondary list-decimal space-y-1 pl-5 text-left text-sm">
						<li>Check that the daemon process is running</li>
						<li>Verify the daemon is listening on the configured port</li>
						<li>Check that no firewall is blocking inbound connections to the daemon</li>
					</ol>
				{:else}
					<ol class="text-secondary list-decimal space-y-1 pl-5 text-left text-sm">
						<li>Check that the daemon process is running</li>
						<li>Verify the server URL in the daemon config matches this server</li>
						<li>Check that no firewall is blocking outbound connections from the daemon</li>
					</ol>
				{/if}
				<button
					type="button"
					class="inline-flex items-center gap-1 text-sm text-blue-400 hover:text-blue-300"
					onclick={() => onTroubleshoot?.()}
				>
					Troubleshoot
					<ExternalLink class="h-3.5 w-3.5" />
				</button>
				{#if isServerPoll && daemonUrl}
					<div class="flex max-w-md flex-col items-center gap-2 text-left">
						{#if healthResult}
							{#if healthResult.reachable && healthResult.health}
								<InlineSuccess
									title="Daemon is reachable and healthy"
									body="The server should register it on its next polling cycle. If this persists, try restarting the daemon."
								/>
							{:else if healthResult.reachable}
								<InlineWarning
									title="Port open but health check failed"
									body="The port is open but the daemon isn't responding to health checks. It may still be starting — try again in a moment. If this persists, <a href='https://scanopy.net/docs/setting-up-daemons/troubleshooting-setup/' target='_blank' class='underline'>check the daemon logs</a>."
								/>
							{:else}
								<InlineDanger
									title={healthResult.error ?? 'Not reachable'}
									body="The daemon may not be running, or the port may no longer be reachable. Verify the install command completed and the daemon process started. If the host has a firewall, check that the port is open and forwarded."
								/>
							{/if}
						{/if}
						<button
							type="button"
							class="btn-primary text-sm"
							disabled={isCheckingHealth}
							onclick={handleHealthCheck}
						>
							{#if isCheckingHealth}
								<Loader2 class="h-4 w-4 animate-spin" />
							{/if}
							Test Daemon Reachability
						</button>
					</div>
				{/if}
			</div>
		{/if}
	{:else}
		<!-- Normal install commands view -->
		{#if hasErrors}
			<InlineWarning
				title={daemons_fixValidationErrors()}
				body={daemons_fixValidationErrorsBody()}
			/>
		{:else}
			<OsSelector
				{selectedOS}
				onOsSelect={handleOsSelect}
				{linuxMethod}
				onLinuxMethodChange={(method) => onLinuxMethodChange?.(method)}
			>
				{#snippet afterLabel()}
					<DocsHint
						text={daemons_docsMultiVlan()}
						href="https://scanopy.net/docs/setting-up-daemons/planning-daemon-deployment/"
						linkText={daemons_docsMultiVlanLinkText()}
					/>
				{/snippet}
				{#snippet afterButtons()}
					<div class="flex items-center gap-2">
						{#if onCredentialWizard}
							<button
								type="button"
								class="btn-secondary shrink-0 text-sm"
								onclick={onCredentialWizard}
							>
								<KeyRound class="h-4 w-4" />
								{daemons_credentialWizardButton()}
							</button>
						{/if}
						{#if onAdvanced}
							<button type="button" class="btn-secondary shrink-0 text-sm" onclick={onAdvanced}>
								<SlidersHorizontal class="h-4 w-4" />
								{common_advanced()}
							</button>
						{/if}
					</div>
				{/snippet}
				{#if selectedOS === 'linux'}
					{#if linuxMethod === 'binary'}
						<p class="text-secondary text-sm">
							This command will download and install the daemon, then start it with your
							configuration.
						</p>
						<CodeContainer
							language="bash"
							expandable={false}
							code={combinedLinuxMacCommand}
							onCopy={() => handleCopy('combined-install')}
						/>
					{:else if linuxMethod === 'docker' && dockerCompose}
						<DocsHint
							text={daemons_docsMacvlan()}
							href="https://scanopy.net/docs/guides/macvlan-setup/"
							linkText={daemons_docsMacvlanLinkText()}
						/>
						<CodeContainer
							language="yaml"
							expandable={false}
							maxHeight=""
							code={dockerCompose}
							onCopy={() => handleCopy('docker-compose')}
						/>
					{/if}
				{:else if selectedOS === 'macos'}
					<p class="text-secondary text-sm">
						This command will download and install the daemon, then start it with your
						configuration.
					</p>
					<CodeContainer
						language="bash"
						expandable={false}
						code={combinedLinuxMacCommand}
						onCopy={() => handleCopy('combined-install')}
					/>

					<InlineInfo title={daemons_dockerLinuxOnly()} body={daemons_dockerLinuxOnlyBody()} />
				{:else if selectedOS === 'windows'}
					<p class="text-secondary text-sm">
						This command will download and install the daemon, then start it with your
						configuration.
					</p>
					<CodeContainer
						language="powershell"
						expandable={false}
						code={combinedWindowsCommand}
						onCopy={() => handleCopy('combined-install')}
					/>

					<InlineWarning title={daemons_wslWarning()} body={daemons_wslWarningBody()} />
					<InlineInfo title={daemons_dockerLinuxOnly()} body={daemons_dockerLinuxOnlyBody()} />
				{/if}
			</OsSelector>

			{#if showTroubleshootingPanel}
				<div bind:this={troubleshootingRef} class="pt-4">
					<p class="text-secondary mb-3 text-sm font-medium">Need help?</p>
					<SupportOptions isTroubleshooting={true} {hasEmailSupport} />
				</div>
			{/if}
		{/if}
	{/if}
</div>
