<script lang="ts">
	import CodeContainer from '$lib/shared/components/data/CodeContainer.svelte';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import InlineDanger from '$lib/shared/components/feedback/InlineDanger.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import { ArrowBigUpDash } from 'lucide-svelte';
	import type { Daemon } from '../types/base';
	import { VERSION } from '$lib/version';
	import {
		type DaemonOS,
		detectOS,
		daemonServiceId,
		daemonLaunchdLabel,
		isPreUnifiedDaemon
	} from '../utils';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import OsSelector from './OsSelector.svelte';
	import {
		common_close,
		common_stepNumber,
		daemons_currentVersion,
		daemons_dockerApplyChanges,
		daemons_dockerLatestTag,
		daemons_dockerLinuxOnly,
		daemons_dockerLinuxOnlyBody,
		daemons_dockerPinnedVersion,
		daemons_docsUpgradeMultipleDaemons,
		daemons_docsUpgradeMultipleDaemonsLinkText,
		daemons_latestVersion,
		daemons_updateAvailable,
		daemons_upgradeConfigPreserved,
		daemons_upgradeDownload,
		daemons_upgradeDaemon,
		daemons_upgradeMultipleDaemons,
		daemons_upgradeMultipleDaemonsBody,
		daemons_upgradeRestartService,
		daemons_upgradeStartService,
		daemons_upgradeStopService,
		daemons_upgradeVolumeMountCheckLabel,
		daemons_upgradeVolumeMountFixStep,
		daemons_upgradeVolumeMountWarningBody,
		daemons_upgradeVolumeMountWarningTitle,
		daemons_sunsetDeprecatedTitle,
		daemons_sunsetDeprecatedBody,
		daemons_sunsetUnsupportedTitle,
		daemons_sunsetUnsupportedBody,
		discovery_upgradeConsolidationWarning
	} from '$lib/paraglide/messages';

	interface Props {
		isOpen?: boolean;
		onClose: () => void;
		daemon: Daemon;
	}

	let { isOpen = false, onClose, daemon }: Props = $props();

	// Server-published sunset for this daemon's version (set for Deprecated /
	// Unsupported). The UI renders the same date the sunset email uses; it never
	// computes its own notion of "deprecated".
	let sunsetDate = $derived(daemon.version_status.sunset_date ?? null);
	let isUnsupported = $derived(daemon.version_status.status === 'Unsupported');
	let sunsetDaysRemaining = $derived.by(() => {
		if (!sunsetDate) return 0;
		const ms = new Date(`${sunsetDate}T00:00:00Z`).getTime() - Date.now();
		return Math.max(0, Math.ceil(ms / 86_400_000));
	});

	// OS selection state
	let selectedOS: DaemonOS = $state(detectOS());

	type LinuxMethod = 'binary' | 'docker';
	let linuxMethod: LinuxMethod = $state('binary');

	// Upgrade = download the new binary, then restart the installed service (systemd / launchd /
	// Windows SCM / FreeBSD rc.d). The daemon runs as a managed service, so a plain foreground
	// restart would fight the auto-respawning service and could leave a duplicate process.
	const binaryUpgradeCommand = `bash -c "$(curl -fsSL https://raw.githubusercontent.com/scanopy/scanopy/refs/heads/main/install.sh)"`;

	let serviceId = $derived(daemonServiceId(daemon.name));
	let launchdLabel = $derived(daemonLaunchdLabel(daemon.name));
	let linuxRestartCommand = $derived(`sudo systemctl restart ${serviceId}`);
	let macosRestartCommand = $derived(`sudo launchctl kickstart -k system/${launchdLabel}`);
	let freebsdRestartCommand = $derived(`sudo service ${serviceId} restart`);

	// Commands to list daemon config directories (each subdirectory = a daemon name)
	const linuxConfigListCommand = 'ls ~/.config/scanopy/daemon/';
	const macosConfigListCommand = 'ls ~/Library/Application\\ Support/com.scanopy.daemon/';
	const windowsConfigListCommand = 'dir %APPDATA%\\scanopy\\daemon\\';

	// Windows: the running .exe is locked, so stop the service before overwriting it in place.
	// `sc` is a PowerShell alias for Set-Content, so use `sc.exe`. Requires an elevated shell.
	const windowsDownloadUrl =
		'https://github.com/scanopy/scanopy/releases/latest/download/scanopy-daemon-windows-amd64.exe';
	const windowsDownloadCommand = `Invoke-WebRequest -Uri "${windowsDownloadUrl}" -OutFile "$env:ProgramFiles\\Scanopy\\scanopy-daemon.exe"`;
	let windowsStopCommand = $derived(`sc.exe stop ${serviceId}`);
	let windowsStartCommand = $derived(`sc.exe start ${serviceId}`);

	const dockerComposeLatestPull = `docker compose pull
docker compose up -d`;
	let dockerComposeImageLine = $derived(`image: ghcr.io/scanopy/scanopy/daemon:v${VERSION}`);

	function handleOsSelect(os: DaemonOS) {
		selectedOS = os;
		trackEvent('daemon_upgrade_os_selected', { os });
	}

	let colorHelper = entities.getColorHelper('Daemon');
</script>

<GenericModal {isOpen} title={daemons_upgradeDaemon()} size="xl" {onClose}>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={ArrowBigUpDash} color={colorHelper.color} />
	{/snippet}

	<div class="flex min-h-0 flex-1 flex-col">
		<div class="flex-1 overflow-auto p-6">
			<div class="space-y-6">
				<p class="text-secondary">
					{daemons_updateAvailable()} <span class="text-primary font-medium">{daemon.name}</span>.
					{#if daemon.version_status.version}
						{daemons_currentVersion()}
						<span class="font-mono">{daemon.version_status.version}.</span>
					{/if}
					{daemons_latestVersion()} <span class="font-mono">{VERSION}.</span>
				</p>

				{#if sunsetDate && isUnsupported}
					<InlineDanger
						title={daemons_sunsetUnsupportedTitle()}
						body={daemons_sunsetUnsupportedBody({ date: sunsetDate })}
					/>
				{:else if sunsetDate}
					<InlineWarning
						title={daemons_sunsetDeprecatedTitle()}
						body={daemons_sunsetDeprecatedBody({ date: sunsetDate, days: sunsetDaysRemaining })}
					/>
				{/if}

				<InlineInfo title="" body={daemons_upgradeConfigPreserved()} />

				{#if isPreUnifiedDaemon(daemon)}
					<InlineWarning
						title=""
						body={discovery_upgradeConsolidationWarning()}
						dismissableKey="unified-discovery-migration"
					/>
				{/if}

				<OsSelector
					{selectedOS}
					onOsSelect={handleOsSelect}
					{linuxMethod}
					onLinuxMethodChange={(method) => (linuxMethod = method)}
				>
					{#if selectedOS === 'linux'}
						{#if linuxMethod === 'binary'}
							<!-- Linux Binary: download new binary, restart the service -->
							<div class="space-y-3">
								<div class="text-secondary">
									<b>{common_stepNumber({ number: '1' })}</b>
									{daemons_upgradeDownload()}
								</div>
								<CodeContainer language="bash" expandable={false} code={binaryUpgradeCommand} />
								<div class="text-secondary">
									<b>{common_stepNumber({ number: '2' })}</b>
									{daemons_upgradeRestartService()}
								</div>
								<details class="text-tertiary text-sm">
									<summary class="cursor-pointer hover:text-blue-400"
										>{daemons_upgradeMultipleDaemons()}</summary
									>
									<div class="mt-2 space-y-2 text-xs">
										<p>{daemons_upgradeMultipleDaemonsBody()}</p>
										<CodeContainer
											language="bash"
											expandable={false}
											code={linuxConfigListCommand}
										/>
										<DocsHint
											text={daemons_docsUpgradeMultipleDaemons()}
											href="https://scanopy.net/docs/guides/multiple-daemons/#upgrading-and-restarting"
											linkText={daemons_docsUpgradeMultipleDaemonsLinkText()}
										/>
									</div>
								</details>
								<CodeContainer language="bash" expandable={false} code={linuxRestartCommand} />
							</div>
						{:else if linuxMethod === 'docker'}
							<!-- Linux Docker Compose -->
							<div class="space-y-3">
								{#if daemon.version_status?.has_correct_docker_volume_mount === false}
									<InlineWarning
										title={daemons_upgradeVolumeMountWarningTitle()}
										body={daemons_upgradeVolumeMountWarningBody()}
									/>
									<div class="text-secondary">
										<b>{common_stepNumber({ number: '1' })}</b>
										{daemons_upgradeVolumeMountCheckLabel()}
									</div>
									<CodeContainer
										language="bash"
										expandable={false}
										code="docker compose config | grep daemon-config"
									/>
									<div class="text-secondary">
										<b>{common_stepNumber({ number: '2' })}</b>
										{daemons_upgradeVolumeMountFixStep()}
									</div>
									<div class="text-secondary">
										<b>{common_stepNumber({ number: '3' })}</b>
										{daemons_dockerLatestTag()}
									</div>
									<CodeContainer
										language="bash"
										expandable={false}
										code={dockerComposeLatestPull}
									/>
								{:else}
									<div class="space-y-2">
										<p class="text-secondary text-sm">
											{daemons_dockerLatestTag()}
										</p>
										<CodeContainer
											language="bash"
											expandable={false}
											code={dockerComposeLatestPull}
										/>
									</div>
								{/if}

								<div class="space-y-2">
									<p class="text-secondary text-sm">
										{daemons_dockerPinnedVersion()}
										<span class="font-mono">docker-compose.yml</span>:
									</p>
									<CodeContainer language="yaml" expandable={false} code={dockerComposeImageLine} />
									<p class="text-secondary text-sm">
										{daemons_dockerApplyChanges()}
									</p>
								</div>
							</div>
						{/if}
					{:else if selectedOS === 'macos'}
						<!-- macOS: download new binary, restart the launchd service -->
						<div class="space-y-3">
							<div class="text-secondary">
								<b>{common_stepNumber({ number: '1' })}</b>
								{daemons_upgradeDownload()}
							</div>
							<CodeContainer language="bash" expandable={false} code={binaryUpgradeCommand} />
							<div class="text-secondary">
								<b>{common_stepNumber({ number: '2' })}</b>
								{daemons_upgradeRestartService()}
							</div>
							<details class="text-tertiary text-sm">
								<summary class="cursor-pointer hover:text-blue-400"
									>{daemons_upgradeMultipleDaemons()}</summary
								>
								<div class="mt-2 space-y-2 text-xs">
									<p>{daemons_upgradeMultipleDaemonsBody()}</p>
									<CodeContainer language="bash" expandable={false} code={macosConfigListCommand} />
									<DocsHint
										text={daemons_docsUpgradeMultipleDaemons()}
										href="https://scanopy.net/docs/guides/multiple-daemons/#upgrading-and-restarting"
										linkText={daemons_docsUpgradeMultipleDaemonsLinkText()}
									/>
								</div>
							</details>
							<CodeContainer language="bash" expandable={false} code={macosRestartCommand} />

							<InlineInfo title={daemons_dockerLinuxOnly()} body={daemons_dockerLinuxOnlyBody()} />
						</div>
					{:else if selectedOS === 'freebsd'}
						<!-- FreeBSD: download new binary, restart the rc.d service -->
						<div class="space-y-3">
							<div class="text-secondary">
								<b>{common_stepNumber({ number: '1' })}</b>
								{daemons_upgradeDownload()}
							</div>
							<CodeContainer language="bash" expandable={false} code={binaryUpgradeCommand} />
							<div class="text-secondary">
								<b>{common_stepNumber({ number: '2' })}</b>
								{daemons_upgradeRestartService()}
							</div>
							<details class="text-tertiary text-sm">
								<summary class="cursor-pointer hover:text-blue-400"
									>{daemons_upgradeMultipleDaemons()}</summary
								>
								<div class="mt-2 space-y-2 text-xs">
									<p>{daemons_upgradeMultipleDaemonsBody()}</p>
									<CodeContainer language="bash" expandable={false} code={linuxConfigListCommand} />
									<DocsHint
										text={daemons_docsUpgradeMultipleDaemons()}
										href="https://scanopy.net/docs/guides/multiple-daemons/#upgrading-and-restarting"
										linkText={daemons_docsUpgradeMultipleDaemonsLinkText()}
									/>
								</div>
							</details>
							<CodeContainer language="bash" expandable={false} code={freebsdRestartCommand} />

							<InlineInfo title={daemons_dockerLinuxOnly()} body={daemons_dockerLinuxOnlyBody()} />
						</div>
					{:else if selectedOS === 'windows'}
						<!-- Windows: stop the service, replace the exe, start the service (elevated PowerShell) -->
						<div class="space-y-3">
							<div class="text-secondary">
								<b>{common_stepNumber({ number: '1' })}</b>
								{daemons_upgradeStopService()}
							</div>
							<CodeContainer language="powershell" expandable={false} code={windowsStopCommand} />
							<div class="text-secondary">
								<b>{common_stepNumber({ number: '2' })}</b>
								{daemons_upgradeDownload()}
							</div>
							<CodeContainer
								language="powershell"
								expandable={false}
								code={windowsDownloadCommand}
							/>
							<div class="text-secondary">
								<b>{common_stepNumber({ number: '3' })}</b>
								{daemons_upgradeStartService()}
							</div>
							<details class="text-tertiary text-sm">
								<summary class="cursor-pointer hover:text-blue-400"
									>{daemons_upgradeMultipleDaemons()}</summary
								>
								<div class="mt-2 space-y-2 text-xs">
									<p>{daemons_upgradeMultipleDaemonsBody()}</p>
									<CodeContainer
										language="powershell"
										expandable={false}
										code={windowsConfigListCommand}
									/>
									<DocsHint
										text={daemons_docsUpgradeMultipleDaemons()}
										href="https://scanopy.net/docs/guides/multiple-daemons/#upgrading-and-restarting"
										linkText={daemons_docsUpgradeMultipleDaemonsLinkText()}
									/>
								</div>
							</details>
							<CodeContainer language="powershell" expandable={false} code={windowsStartCommand} />

							<InlineInfo title={daemons_dockerLinuxOnly()} body={daemons_dockerLinuxOnlyBody()} />
						</div>
					{/if}
				</OsSelector>
			</div>
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex items-center justify-end">
				<button type="button" class="btn-secondary" onclick={onClose}>{common_close()}</button>
			</div>
		</div>
	</div>
</GenericModal>
