<script lang="ts">
	import CodeContainer from '$lib/shared/components/data/CodeContainer.svelte';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import { ArrowBigUpDash } from 'lucide-svelte';
	import type { Daemon } from '../types/base';
	import { VERSION } from '$lib/version';
	import { type DaemonOS, detectOS } from '../utils';
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
		daemons_upgradeStartProcess,
		daemons_upgradeStopProcess,
		discovery_upgradeConsolidationWarning
	} from '$lib/paraglide/messages';

	interface Props {
		isOpen?: boolean;
		onClose: () => void;
		daemon: Daemon;
	}

	let { isOpen = false, onClose, daemon }: Props = $props();

	// OS selection state
	let selectedOS: DaemonOS = $state(detectOS());

	type LinuxMethod = 'binary' | 'docker';
	let linuxMethod: LinuxMethod = $state('binary');

	// Commands for upgrading — all platforms use stop/download/start flow
	const binaryUpgradeCommand = `bash -c "$(curl -fsSL https://raw.githubusercontent.com/scanopy/scanopy/refs/heads/main/install.sh)"`;

	const stopCommand = 'sudo pkill scanopy-daemon';
	let startCommand = $derived(`sudo scanopy-daemon --name ${daemon.name}`);

	// Commands to list daemon config directories (each subdirectory = a daemon name)
	const linuxConfigListCommand = 'ls ~/.config/scanopy/daemon/';
	const macosConfigListCommand = 'ls ~/Library/Application\\ Support/com.scanopy.daemon/';
	const windowsConfigListCommand = 'dir %APPDATA%\\scanopy\\daemon\\';

	const windowsDownloadUrl =
		'https://github.com/scanopy/scanopy/releases/latest/download/scanopy-daemon-windows-amd64.exe';
	const windowsDownloadCommand = `Invoke-WebRequest -Uri "${windowsDownloadUrl}" -OutFile "scanopy-daemon-windows-amd64.exe"`;
	const windowsStopCommand = 'Stop-Process -Name "scanopy-daemon-windows-amd64"';
	let windowsStartCommand = $derived(`.\\scanopy-daemon-windows-amd64.exe --name ${daemon.name}`);

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

				<InlineInfo title="" body={daemons_upgradeConfigPreserved()} />

				<InlineWarning
					title=""
					body={discovery_upgradeConsolidationWarning()}
					dismissableKey="unified-discovery-migration"
				/>

				<OsSelector
					{selectedOS}
					onOsSelect={handleOsSelect}
					{linuxMethod}
					onLinuxMethodChange={(method) => (linuxMethod = method)}
				>
					{#if selectedOS === 'linux'}
						{#if linuxMethod === 'binary'}
							<!-- Linux Binary: stop, download, start -->
							<div class="space-y-3">
								<div class="text-secondary">
									<b>{common_stepNumber({ number: '1' })}</b>
									{daemons_upgradeStopProcess()}
								</div>
								<CodeContainer language="bash" expandable={false} code={stopCommand} />
								<div class="text-secondary">
									<b>{common_stepNumber({ number: '2' })}</b>
									{daemons_upgradeDownload()}
								</div>
								<CodeContainer language="bash" expandable={false} code={binaryUpgradeCommand} />
								<div class="text-secondary">
									<b>{common_stepNumber({ number: '3' })}</b>
									{daemons_upgradeStartProcess()}
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
											href="https://scanopy.net/docs/guides/multiple-daemons/#upgrading"
											linkText={daemons_docsUpgradeMultipleDaemonsLinkText()}
										/>
									</div>
								</details>
								<CodeContainer language="bash" expandable={false} code={startCommand} />
							</div>
						{:else if linuxMethod === 'docker'}
							<!-- Linux Docker Compose -->
							<div class="space-y-3">
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
						<!-- macOS: stop, download, start -->
						<div class="space-y-3">
							<div class="text-secondary">
								<b>{common_stepNumber({ number: '1' })}</b>
								{daemons_upgradeStopProcess()}
							</div>
							<CodeContainer language="bash" expandable={false} code={stopCommand} />
							<div class="text-secondary">
								<b>{common_stepNumber({ number: '2' })}</b>
								{daemons_upgradeDownload()}
							</div>
							<CodeContainer language="bash" expandable={false} code={binaryUpgradeCommand} />
							<div class="text-secondary">
								<b>{common_stepNumber({ number: '3' })}</b>
								{daemons_upgradeStartProcess()}
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
										href="https://scanopy.net/docs/guides/multiple-daemons/#upgrading"
										linkText={daemons_docsUpgradeMultipleDaemonsLinkText()}
									/>
								</div>
							</details>
							<CodeContainer language="bash" expandable={false} code={startCommand} />

							<InlineInfo title={daemons_dockerLinuxOnly()} body={daemons_dockerLinuxOnlyBody()} />
						</div>
					{:else if selectedOS === 'windows'}
						<!-- Windows: stop, download, start -->
						<div class="space-y-3">
							<div class="text-secondary">
								<b>{common_stepNumber({ number: '1' })}</b>
								{daemons_upgradeStopProcess()}
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
								{daemons_upgradeStartProcess()}
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
										href="https://scanopy.net/docs/guides/multiple-daemons/#upgrading"
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
