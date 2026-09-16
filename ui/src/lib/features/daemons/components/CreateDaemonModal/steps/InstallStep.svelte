<script lang="ts">
	import CodeContainer from '$lib/shared/components/data/CodeContainer.svelte';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import InlineDanger from '$lib/shared/components/feedback/InlineDanger.svelte';
	import InlineSuccess from '$lib/shared/components/feedback/InlineSuccess.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import TroubleshootingChecklist from './TroubleshootingChecklist.svelte';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import { downloadDaemonMsi, type DaemonOS } from '../../../utils';
	import { osInstallCommand, type InstallArtifacts } from '../../../types/base';
	import type { DaemonMode } from '../../../types/base';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { useTestReachabilityMutation, useRetryDaemonConnectionMutation } from '../../../queries';
	import AnimatedProgressBar from '$lib/features/discovery/components/cards/AnimatedProgressBar.svelte';
	import ProgressTrack from '$lib/shared/components/data/ProgressTrack.svelte';
	import OsSelector from '../../OsSelector.svelte';
	import { Download, Loader2, CheckCircle2, AlertTriangle, SlidersHorizontal } from 'lucide-svelte';
	import type { DaemonConnectionStatus } from '../../../stores/daemon-setup';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import {
		common_advanced,
		daemons_advancedTooltip,
		daemons_docsMacvlan,
		daemons_docsMacvlanLinkText,
		common_ossign,
		daemons_docsMsiSigning,
		daemons_docsMultiVlan,
		daemons_docsMultiVlanLinkText,
		daemons_fixValidationErrors,
		daemons_fixValidationErrorsBody,
		daemons_installCommandDescription,
		daemons_msiDownloadButton,
		daemons_msiOmittedConfigBody,
		daemons_msiOmittedConfigTitle,
		daemons_msiRenameHint,
		common_firstDiscoveryEmailHint,
		common_viewTopology,
		daemons_troubleshoot_waitingTitle,
		daemons_troubleshoot_waitingDesc,
		daemons_troubleshoot_troubleTitle,
		daemons_troubleshoot_troubleTitleServerPoll,
		daemons_troubleshoot_connectedTitle,
		daemons_troubleshoot_connectedDesc,
		daemons_troubleshoot_testingConnection,
		daemons_troubleshoot_reachablePolling,
		daemons_troubleshoot_connectionTestFailed,
		daemons_troubleshoot_testReachability,
		daemons_troubleshoot_healthPartial,
		daemons_troubleshoot_healthPartialDesc,
		daemons_troubleshoot_healthUnreachable,
		daemons_troubleshoot_healthUnreachableDesc
	} from '$lib/paraglide/messages';

	type LinuxMethod = 'binary' | 'docker';
	type WindowsMethod = 'exe' | 'msi';

	interface Props {
		selectedOS: DaemonOS;
		onOsSelect: (os: DaemonOS) => void;
		linuxMethod?: LinuxMethod;
		onLinuxMethodChange?: (method: LinuxMethod) => void;
		windowsMethod?: WindowsMethod;
		onWindowsMethodChange?: (method: WindowsMethod) => void;
		runCommand: string;
		/** Server-assembled install artifacts (single source of truth), key already filled. */
		artifacts?: InstallArtifacts | null;
		hasErrors: boolean;
		isFirstDaemon?: boolean;
		connectionStatus?: DaemonConnectionStatus;
		onViewDiscovery?: () => void;
		hasEmailSupport?: boolean;
		onAdvanced?: (() => void) | null;
		daemonMode?: DaemonMode;
		daemonName?: string;
		logFilePath?: string;
		daemonUrl?: string;
		provisionedDaemonId?: string;
		onStartWaitingTimeout?: () => void;
		onProgressComplete?: () => void;
		onReviewCommands?: () => void;
		onEnableSelfSigned?: () => void;
		onCopied?: () => void;
	}

	let {
		selectedOS,
		onOsSelect,
		linuxMethod = 'binary',
		onLinuxMethodChange,
		windowsMethod = 'exe',
		onWindowsMethodChange,
		runCommand,
		artifacts = null,
		hasErrors,
		isFirstDaemon = false,
		connectionStatus = 'idle',
		onViewDiscovery,
		hasEmailSupport = false,
		onAdvanced = null,
		daemonMode = 'daemon_poll',
		daemonName = 'scanopy-daemon',
		logFilePath = '',
		daemonUrl = '',
		provisionedDaemonId = '',
		onStartWaitingTimeout,
		onProgressComplete,
		onReviewCommands,
		onEnableSelfSigned,
		onCopied
	}: Props = $props();

	const configQuery = useConfigQuery();
	let hasEmail = $derived(configQuery.data?.has_email_service ?? false);
	let serverUrl = $derived(configQuery.data?.public_url ?? '');

	// MSI support is hidden for now (revisit later) — flip this back on to restore the
	// Windows exe/msi toggle. Nothing else is removed; this just stops OsSelector from
	// rendering the toggle, so windowsMethod stays 'exe' and the MSI download UI is unreachable.
	const WINDOWS_MSI_ENABLED = false;

	const windowsDownloadUrl =
		'https://github.com/scanopy/scanopy/releases/latest/download/scanopy-daemon-windows-amd64.exe';
	const windowsInstallCommand = `Invoke-WebRequest -Uri "${windowsDownloadUrl}" -OutFile "scanopy-daemon-windows-amd64.exe"`;
	const installScript = `bash -c "$(curl -fsSL https://raw.githubusercontent.com/scanopy/scanopy/refs/heads/main/install.sh)"`;

	// Combined install commands — prefer the server-assembled command for the platform
	// (single source of truth), falling back to the client-built one.
	let dockerCompose = $derived(artifacts?.docker.compose ?? '');
	let combinedLinuxMacCommand = $derived(
		(artifacts ? osInstallCommand(artifacts, selectedOS) : '') ||
			`${installScript} && ${runCommand}`
	);
	let combinedWindowsCommand = $derived(
		(artifacts ? osInstallCommand(artifacts, 'windows') : '') ||
			`${windowsInstallCommand}; ${runCommand}`
	);

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
			} else if (!result.reachable) {
				// Health check failed — transition to trouble state for full troubleshooting
				onProgressComplete?.();
			}
		} catch {
			healthResult = { reachable: false, error: 'Failed to test reachability' };
			onProgressComplete?.();
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

	// MSI download: fetched through our own server so the browser can save it under the
	// per-daemon filename (see downloadDaemonMsi). Falls back to the direct GitHub link (which
	// needs a manual rename) if our server can't reach GitHub.
	let isDownloadingMsi = $state(false);
	let msiNeedsRename = $state(false);

	async function handleDownloadMsi(filename: string) {
		isDownloadingMsi = true;
		try {
			const savedUnderFilename = await downloadDaemonMsi(filename);
			msiNeedsRename = !savedUnderFilename;
			trackEvent('daemon_install_msi_downloaded', { renamed: !savedUnderFilename });
		} finally {
			isDownloadingMsi = false;
		}
	}

	function handleCopy(context: string) {
		trackEvent('daemon_install_command_copied', { os: selectedOS, context });
		onCopied?.();
	}

	// Show waiting UI when connection status is not idle
	let showWaitingUI = $derived(connectionStatus !== 'idle');

	// Progress bar for waiting state (0-100 over 60 seconds)
	const WAIT_DURATION_MS = 45_000;
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
				if (waitingProgress >= 100) {
					clearInterval(interval);
					onProgressComplete?.();
				}
			}, 500);
			return () => clearInterval(interval);
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
						<p class="text-secondary text-sm">{daemons_troubleshoot_testingConnection()}</p>
					{:else if serverPollReachable}
						<div class="flex w-full max-w-xs items-center gap-2">
							<ProgressTrack class="flex-1">
								<AnimatedProgressBar progress={waitingProgress} />
							</ProgressTrack>
							<span class="text-secondary text-xs tabular-nums">{Math.round(waitingProgress)}%</span
							>
						</div>
						<div class="max-w-md text-left">
							<InlineSuccess title={daemons_troubleshoot_reachablePolling()} />
						</div>
					{:else if healthResult}
						<!-- Health check failed -->
						<h3 class="text-primary text-base font-semibold">
							{daemons_troubleshoot_connectionTestFailed()}
						</h3>
						<div class="max-w-md text-left">
							{#if healthResult.reachable}
								<InlineWarning
									title={daemons_troubleshoot_healthPartial()}
									body={daemons_troubleshoot_healthPartialDesc()}
								/>
							{:else}
								<InlineDanger
									title={healthResult.error ?? daemons_troubleshoot_healthUnreachable()}
									body={daemons_troubleshoot_healthUnreachableDesc()}
								/>
							{/if}
						</div>
						<button
							type="button"
							class="btn-primary text-sm"
							disabled={isCheckingHealth}
							onclick={handleHealthCheck}
						>
							{daemons_troubleshoot_testReachability()}
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
							{daemons_troubleshoot_waitingTitle()}
						</h3>
						<p class="text-secondary mt-1 text-sm">
							{daemons_troubleshoot_waitingDesc()}
						</p>
					</div>
				{/if}
			</div>
		{:else if connectionStatus === 'connected'}
			<div class="flex flex-col items-center gap-4 py-8 text-center">
				<CheckCircle2 class="h-10 w-10 text-green-400" />
				<div>
					<h3 class="text-primary text-base font-semibold">
						{daemons_troubleshoot_connectedTitle()}
					</h3>
					<p class="text-secondary mt-1 text-sm">
						{daemons_troubleshoot_connectedDesc()}
					</p>
				</div>
				<button type="button" class="btn-primary" onclick={() => onViewDiscovery?.()}>
					{common_viewTopology()}
				</button>
				{#if hasEmail && isFirstDaemon}
					<p class="text-secondary text-sm">
						{common_firstDiscoveryEmailHint()}
					</p>
				{/if}
			</div>
		{:else if connectionStatus === 'trouble'}
			<div class="flex flex-col gap-4 py-4">
				<div class="flex items-center gap-3">
					<AlertTriangle class="h-8 w-8 flex-shrink-0 text-yellow-400" />
					<h3 class="text-primary text-base font-semibold">
						{#if isServerPoll}
							{daemons_troubleshoot_troubleTitleServerPoll()}
						{:else}
							{daemons_troubleshoot_troubleTitle()}
						{/if}
					</h3>
				</div>
				<TroubleshootingChecklist
					mode={isServerPoll ? 'server_poll' : 'daemon_poll'}
					{serverUrl}
					{daemonUrl}
					{daemonName}
					{selectedOS}
					{linuxMethod}
					{hasEmailSupport}
					{logFilePath}
					onHealthCheck={handleHealthCheck}
					{isCheckingHealth}
					{healthResult}
					{onReviewCommands}
					{onEnableSelfSigned}
				/>
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
				{windowsMethod}
				onWindowsMethodChange={WINDOWS_MSI_ENABLED
					? (method) => onWindowsMethodChange?.(method)
					: undefined}
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
						{#if onAdvanced}
							<button
								type="button"
								class="btn-secondary h-10 shrink-0 text-sm"
								data-tooltip={daemons_advancedTooltip()}
								use:tooltip
								onclick={onAdvanced}
							>
								<SlidersHorizontal class="h-4 w-4" />
								<span class="hidden sm:inline">{common_advanced()}</span>
							</button>
						{/if}
					</div>
				{/snippet}
				{#if selectedOS === 'linux'}
					{#if linuxMethod === 'binary'}
						<p class="text-secondary text-sm">
							{daemons_installCommandDescription()}
						</p>
						<CodeContainer
							language="bash"
							expandable={false}
							maxHeight=""
							code={combinedLinuxMacCommand}
							onCopy={() => handleCopy('combined-install')}
							preventSelect={true}
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
							preventSelect={true}
						/>
					{/if}
				{:else if selectedOS === 'macos'}
					<p class="text-secondary text-sm">
						{daemons_installCommandDescription()}
					</p>
					<CodeContainer
						language="bash"
						expandable={false}
						maxHeight=""
						code={combinedLinuxMacCommand}
						onCopy={() => handleCopy('combined-install')}
						preventSelect={true}
					/>
				{:else if selectedOS === 'windows'}
					<DocsHint
						text={daemons_docsMsiSigning()}
						href="https://ossign.org"
						linkText={common_ossign()}
					/>
					{#if windowsMethod === 'exe'}
						<p class="text-secondary text-sm">
							{daemons_installCommandDescription()}
						</p>
						<CodeContainer
							language="powershell"
							expandable={false}
							maxHeight=""
							code={combinedWindowsCommand}
							onCopy={() => handleCopy('combined-install')}
							preventSelect={true}
						/>
					{:else if artifacts?.msi.filename}
						<div class="flex flex-wrap items-center gap-2">
							<button
								type="button"
								class="btn-secondary inline-flex items-center gap-1"
								disabled={isDownloadingMsi}
								onclick={() => handleDownloadMsi(artifacts.msi.filename)}
							>
								{#if isDownloadingMsi}
									<Loader2 class="h-4 w-4 animate-spin" />
								{:else}
									<Download class="h-4 w-4" />
								{/if}
								{daemons_msiDownloadButton()}
							</button>
							<span class="text-secondary font-mono text-xs">{artifacts.msi.filename}</span>
						</div>
						{#if msiNeedsRename}
							<p class="text-tertiary text-xs">
								{daemons_msiRenameHint({ filename: artifacts.msi.filename })}
							</p>
						{/if}
						{#if artifacts.msi.omitted_config_keys.length > 0}
							<InlineWarning
								title={daemons_msiOmittedConfigTitle()}
								body={daemons_msiOmittedConfigBody({
									keys: artifacts.msi.omitted_config_keys.join(', ')
								})}
							/>
						{/if}
					{/if}
				{:else if selectedOS === 'freebsd'}
					<p class="text-secondary text-sm">
						{daemons_installCommandDescription()}
					</p>
					<CodeContainer
						language="bash"
						expandable={false}
						maxHeight=""
						code={combinedLinuxMacCommand}
						onCopy={() => handleCopy('combined-install')}
						preventSelect={true}
					/>
				{/if}
			</OsSelector>
		{/if}
	{/if}
</div>
