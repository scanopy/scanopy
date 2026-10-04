<script lang="ts">
	import ChecklistItem from '$lib/shared/components/data/ChecklistItem.svelte';
	import CopyableValue from '$lib/shared/components/data/CopyableValue.svelte';
	import SupportOptions from '$lib/features/support/SupportOptions.svelte';
	import InlineSuccess from '$lib/shared/components/feedback/InlineSuccess.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import InlineDanger from '$lib/shared/components/feedback/InlineDanger.svelte';
	import { Loader2 } from 'lucide-svelte';
	import type { DaemonOS } from '../../../utils';
	import type { DaemonMode } from '../../../types/base';
	import {
		daemons_troubleshoot_isDaemonRunning,
		daemons_troubleshoot_isDaemonRunningDesc,
		daemons_troubleshoot_isListening,
		daemons_troubleshoot_isListeningDesc,
		daemons_troubleshoot_processNotFound,
		daemons_troubleshoot_canReachServer,
		daemons_troubleshoot_canReachServerStep1,
		daemons_troubleshoot_canReachServerStep2,
		daemons_troubleshoot_canReachServerStep3,
		daemons_troubleshoot_firewallNote,
		daemons_troubleshoot_followSteps,
		daemons_troubleshoot_canServerReach,
		daemons_troubleshoot_canServerReachDesc,
		daemons_troubleshoot_firewallNat,
		daemons_troubleshoot_firewallNatDesc,
		daemons_troubleshoot_checkLogs,
		daemons_troubleshoot_checkLogsDesc,
		daemons_troubleshoot_logFileAt,
		daemons_troubleshoot_logFileCustom,
		daemons_troubleshoot_logFileOrJournal,
		daemons_troubleshoot_logFileDocker,
		daemons_troubleshoot_logFileMounted,
		daemons_troubleshoot_commonErrors,
		daemons_troubleshoot_errConnRefused,
		daemons_troubleshoot_errConnRefusedCause,
		daemons_troubleshoot_errConnRefusedFix,
		daemons_troubleshoot_errTimeout,
		daemons_troubleshoot_errTimeoutCause,
		daemons_troubleshoot_errTimeoutFix,
		daemons_troubleshoot_errTls,
		daemons_troubleshoot_errTlsCause,
		daemons_troubleshoot_errTlsFix,
		daemons_troubleshoot_errApiKey,
		daemons_troubleshoot_errApiKeyCause,
		daemons_troubleshoot_errApiKeyFix,
		daemons_troubleshoot_errDns,
		daemons_troubleshoot_errDnsCause,
		daemons_troubleshoot_errDnsFix,
		daemons_troubleshoot_errNotScanopy,
		daemons_troubleshoot_errNotScanopyCause,
		daemons_troubleshoot_errNotScanopyFix,
		daemons_troubleshoot_stillStuck,
		daemons_troubleshoot_stillStuckDesc,
		daemons_troubleshoot_healthReachable,
		daemons_troubleshoot_healthReachableDesc,
		daemons_troubleshoot_healthPartial,
		daemons_troubleshoot_healthPartialDesc,
		daemons_troubleshoot_healthUnreachable,
		daemons_troubleshoot_healthUnreachableDesc,
		daemons_troubleshoot_testReachability,
		daemons_troubleshoot_reviewInstallCommand,
		daemons_troubleshoot_enableSelfSigned,
		common_error,
		common_cause,
		common_fix
	} from '$lib/paraglide/messages';

	type LinuxMethod = 'binary' | 'docker';

	interface Props {
		mode: DaemonMode;
		serverUrl: string;
		daemonUrl?: string;
		daemonName?: string;
		selectedOS: DaemonOS;
		linuxMethod?: LinuxMethod;
		logFilePath?: string;
		onHealthCheck?: () => void;
		isCheckingHealth?: boolean;
		healthResult?: { reachable: boolean; health?: boolean; error?: string } | null;
		onReviewCommands?: () => void;
		onEnableSelfSigned?: () => void;
	}

	let {
		mode,
		serverUrl,
		daemonUrl = '',
		daemonName = 'scanopy-daemon',
		selectedOS,
		linuxMethod = 'binary',
		logFilePath = '',
		onHealthCheck,
		isCheckingHealth = false,
		healthResult = null,
		onReviewCommands,
		onEnableSelfSigned
	}: Props = $props();

	let isServerPoll = $derived(mode === 'server_poll');
	let isDocker = $derived(selectedOS === 'linux' && linuxMethod === 'docker');

	// Checklist state (local, not persisted)
	let checked = $state<Record<string, boolean>>({});
	function toggle(id: string) {
		checked[id] = !checked[id];
	}

	// OS-specific process check commands
	let processCheckCommand = $derived.by(() => {
		if (isDocker) return 'docker ps | grep scanopy';
		switch (selectedOS) {
			case 'linux':
				return 'systemctl status scanopy-daemon';
			case 'macos':
				return 'ps aux | grep scanopy-daemon';
			case 'windows':
				return 'Get-Process scanopy-daemon*';
			default:
				return 'systemctl status scanopy-daemon';
		}
	});

	// Port check command for ServerPoll
	let portCheckCommand = $derived.by(() => {
		if (isDocker) return 'docker ps | grep scanopy';
		switch (selectedOS) {
			case 'linux':
				return 'ss -tlnp | grep 60073';
			case 'macos':
				return 'lsof -i :60073';
			case 'windows':
				return 'netstat -an | findstr 60073';
			default:
				return 'ss -tlnp | grep 60073';
		}
	});

	// Effective log file path — custom if set, otherwise OS default
	let hasCustomLogPath = $derived(!!logFilePath && logFilePath !== 'none');
	let effectiveLogPath = $derived.by(() => {
		if (hasCustomLogPath) {
			// If custom path looks like a directory, append daemon filename
			if (logFilePath.endsWith('/') || logFilePath.endsWith('\\') || !logFilePath.includes('.')) {
				const name = daemonName || 'scanopy-daemon';
				const sep = logFilePath.endsWith('/') || logFilePath.endsWith('\\') ? '' : '/';
				return `${logFilePath}${sep}${name}.log`;
			}
			return logFilePath;
		}
		const name = daemonName || 'scanopy-daemon';
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

	// Docker-specific: mounted volume path on host
	let dockerHostLogPath = $derived.by(() => {
		const name = daemonName || 'scanopy-daemon';
		return `/var/log/scanopy/${name}.log`;
	});

	// OS-specific log commands
	let logCommand = $derived.by(() => {
		if (isDocker) return `docker logs ${daemonName || 'scanopy-daemon'} --tail 50`;
		switch (selectedOS) {
			case 'linux':
			case 'macos':
				return `tail -50 ${effectiveLogPath}`;
			case 'windows':
				return `Get-Content "${effectiveLogPath}" -Tail 50`;
			default:
				return `tail -50 ${effectiveLogPath}`;
		}
	});

	let journalCommand = $derived(
		selectedOS === 'linux' && !isDocker ? 'journalctl -u scanopy-daemon -n 50 --no-pager' : ''
	);

	// Health check command for DaemonPoll
	let healthCheckCommand = $derived(`curl -s ${serverUrl}/api/health`);

	// Extract hostname and port info from server URL
	let serverHostname = $derived.by(() => {
		try {
			return new URL(serverUrl).hostname;
		} catch {
			return serverUrl;
		}
	});

	let serverPortDesc = $derived.by(() => {
		try {
			const url = new URL(serverUrl);
			if (url.port) return `port ${url.port}`;
			return url.protocol === 'https:' ? 'port 443 (HTTPS)' : 'port 80 (HTTP)';
		} catch {
			return 'port 443 (HTTPS)';
		}
	});
</script>

<div class="card card-static divide-y divide-gray-200 dark:divide-gray-700">
	<p class="text-secondary pb-3 text-sm">{daemons_troubleshoot_followSteps()}</p>

	{#if isServerPoll}
		<ChecklistItem
			label={daemons_troubleshoot_isListening()}
			checked={!!checked['listening']}
			onToggle={() => toggle('listening')}
		>
			{#snippet detail()}
				<p class="text-tertiary text-sm">{daemons_troubleshoot_isListeningDesc()}</p>
				<CopyableValue value={processCheckCommand} />
				{#if !isDocker}
					<CopyableValue value={portCheckCommand} />
				{/if}
				<p class="text-tertiary text-sm">{daemons_troubleshoot_processNotFound()}</p>
			{/snippet}
		</ChecklistItem>

		<ChecklistItem
			label={daemons_troubleshoot_firewallNat()}
			checked={!!checked['firewall']}
			onToggle={() => toggle('firewall')}
		>
			{#snippet detail()}
				<p class="text-tertiary text-sm">
					{daemons_troubleshoot_firewallNatDesc({ daemonUrl: daemonUrl || '' })}
				</p>
				<p class="text-tertiary text-sm italic">{daemons_troubleshoot_firewallNote()}</p>
			{/snippet}
		</ChecklistItem>

		<ChecklistItem
			label={daemons_troubleshoot_canServerReach()}
			checked={!!checked['server-reach']}
			onToggle={() => toggle('server-reach')}
		>
			{#snippet detail()}
				<p class="text-tertiary text-sm">{daemons_troubleshoot_canServerReachDesc()}</p>
				{#if daemonUrl && onHealthCheck}
					{#if healthResult}
						{#if healthResult.reachable && healthResult.health}
							<InlineSuccess
								title={daemons_troubleshoot_healthReachable()}
								body={daemons_troubleshoot_healthReachableDesc()}
							/>
						{:else if healthResult.reachable}
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
					{/if}
					<button
						type="button"
						class="btn-primary text-sm"
						disabled={isCheckingHealth}
						onclick={onHealthCheck}
					>
						{#if isCheckingHealth}
							<Loader2 class="h-4 w-4 animate-spin" />
						{/if}
						{daemons_troubleshoot_testReachability()}
					</button>
				{/if}
			{/snippet}
		</ChecklistItem>
	{:else}
		<ChecklistItem
			label={daemons_troubleshoot_isDaemonRunning()}
			checked={!!checked['running']}
			onToggle={() => toggle('running')}
		>
			{#snippet detail()}
				<p class="text-tertiary text-sm">{daemons_troubleshoot_isDaemonRunningDesc()}</p>
				<CopyableValue value={processCheckCommand} />
				<p class="text-tertiary text-sm">{daemons_troubleshoot_processNotFound()}</p>
				{#if onReviewCommands}
					<button type="button" class="btn-secondary text-xs" onclick={onReviewCommands}>
						{daemons_troubleshoot_reviewInstallCommand()}
					</button>
				{/if}
			{/snippet}
		</ChecklistItem>

		<ChecklistItem
			label={daemons_troubleshoot_canReachServer()}
			checked={!!checked['reach-server']}
			onToggle={() => toggle('reach-server')}
		>
			{#snippet detail()}
				<p class="text-tertiary text-sm">{daemons_troubleshoot_canReachServerStep1()}</p>
				<CopyableValue value={healthCheckCommand} />
				<p class="text-tertiary mt-2 text-sm">{daemons_troubleshoot_canReachServerStep2()}</p>
				<CopyableValue value={`nslookup ${serverHostname}`} />
				<p class="text-tertiary mt-2 text-sm">
					{daemons_troubleshoot_canReachServerStep3({ portDesc: serverPortDesc })}
				</p>
				<p class="text-tertiary mt-1 text-sm italic">{daemons_troubleshoot_firewallNote()}</p>
			{/snippet}
		</ChecklistItem>
	{/if}

	<!-- Check logs (both modes) -->
	<ChecklistItem
		label={daemons_troubleshoot_checkLogs()}
		checked={!!checked['logs']}
		onToggle={() => toggle('logs')}
	>
		{#snippet detail()}
			<p class="text-tertiary text-sm">{daemons_troubleshoot_checkLogsDesc()}</p>
			{#if isDocker}
				<p class="text-secondary mt-2 text-sm font-medium">
					{daemons_troubleshoot_logFileDocker()}
				</p>
				<CopyableValue value={logCommand} />
				<p class="text-secondary mt-2 text-sm font-medium">
					{daemons_troubleshoot_logFileMounted()}
				</p>
				<CopyableValue value={`tail -50 ${dockerHostLogPath}`} />
			{:else}
				{#if hasCustomLogPath}
					<p class="text-secondary mt-2 text-sm font-medium">
						{daemons_troubleshoot_logFileCustom({ path: effectiveLogPath })}
					</p>
				{:else}
					<p class="text-secondary mt-2 text-sm font-medium">
						{daemons_troubleshoot_logFileAt({ path: effectiveLogPath })}
					</p>
				{/if}
				<CopyableValue value={logCommand} />
				{#if journalCommand}
					<p class="text-secondary mt-2 text-sm font-medium">
						{daemons_troubleshoot_logFileOrJournal()}
					</p>
					<CopyableValue value={journalCommand} />
				{/if}
			{/if}

			<div class="mt-3">
				<p class="text-secondary text-sm font-medium">{daemons_troubleshoot_commonErrors()}</p>
				<table class="text-tertiary mt-1 w-full text-xs">
					<thead>
						<tr class="text-secondary border-b border-gray-200 text-left dark:border-gray-700">
							<th class="pb-1 pr-3 font-medium">{common_error()}</th>
							<th class="pb-1 pr-3 font-medium">{common_cause()}</th>
							<th class="pb-1 font-medium">{common_fix()}</th>
						</tr>
					</thead>
					<tbody
						class="[&>tr:nth-child(even)]:bg-gray-50 dark:[&>tr:nth-child(even)]:bg-gray-800/30"
					>
						<tr>
							<td class="py-1.5 pr-3 font-mono">{daemons_troubleshoot_errConnRefused()}</td>
							<td class="py-1.5 pr-3">{daemons_troubleshoot_errConnRefusedCause()}</td>
							<td class="py-1.5">{daemons_troubleshoot_errConnRefusedFix()}</td>
						</tr>
						<tr>
							<td class="py-1.5 pr-3 font-mono">{daemons_troubleshoot_errTimeout()}</td>
							<td class="py-1.5 pr-3">{daemons_troubleshoot_errTimeoutCause()}</td>
							<td class="py-1.5">
								{daemons_troubleshoot_errTimeoutFix({ portDesc: serverPortDesc })}
							</td>
						</tr>
						<tr>
							<td class="py-1.5 pr-3 font-mono">{daemons_troubleshoot_errTls()}</td>
							<td class="py-1.5 pr-3">{daemons_troubleshoot_errTlsCause()}</td>
							<td class="py-1.5">
								{daemons_troubleshoot_errTlsFix()}
								{#if onEnableSelfSigned}
									<button
										type="button"
										class="ml-1 text-blue-400 underline hover:text-blue-300"
										onclick={onEnableSelfSigned}
									>
										{daemons_troubleshoot_enableSelfSigned()}
									</button>
								{/if}
							</td>
						</tr>
						<tr>
							<td class="py-1.5 pr-3 font-mono">{daemons_troubleshoot_errApiKey()}</td>
							<td class="py-1.5 pr-3">{daemons_troubleshoot_errApiKeyCause()}</td>
							<td class="py-1.5">
								{daemons_troubleshoot_errApiKeyFix()}
								{#if onReviewCommands}
									<button
										type="button"
										class="ml-1 text-blue-400 underline hover:text-blue-300"
										onclick={onReviewCommands}
									>
										{daemons_troubleshoot_reviewInstallCommand()}
									</button>
								{/if}
							</td>
						</tr>
						<tr>
							<td class="py-1.5 pr-3 font-mono">{daemons_troubleshoot_errDns()}</td>
							<td class="py-1.5 pr-3">{daemons_troubleshoot_errDnsCause()}</td>
							<td class="py-1.5">{daemons_troubleshoot_errDnsFix()}</td>
						</tr>
						<tr>
							<td class="py-1.5 pr-3 font-mono">{daemons_troubleshoot_errNotScanopy()}</td>
							<td class="py-1.5 pr-3">{daemons_troubleshoot_errNotScanopyCause()}</td>
							<td class="py-1.5">{daemons_troubleshoot_errNotScanopyFix()}</td>
						</tr>
					</tbody>
				</table>
			</div>
		{/snippet}
	</ChecklistItem>

	<!-- Still stuck? — always expanded -->
	<div class="pt-3">
		<h3 class="text-primary text-sm font-semibold">{daemons_troubleshoot_stillStuck()}</h3>
		<p class="text-tertiary mt-0.5 text-xs">{daemons_troubleshoot_stillStuckDesc()}</p>
		<div class="mt-3">
			<SupportOptions isTroubleshooting={true} />
		</div>
	</div>
</div>
