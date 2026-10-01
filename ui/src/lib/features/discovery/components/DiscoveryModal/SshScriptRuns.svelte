<script lang="ts">
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import InfoRow from '$lib/shared/components/data/InfoRow.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import type { Color } from '$lib/shared/utils/styling';
	import type { components } from '$lib/api/schema';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import {
		common_applied,
		common_details,
		common_duration,
		common_unknown,
		discovery_sshScriptAppliedKeys,
		discovery_sshScriptExitCode,
		discovery_sshScriptOutcomeAuthenticationFailed,
		discovery_sshScriptOutcomeConnectionFailed,
		discovery_sshScriptOutcomeHostKeyMismatch,
		discovery_sshScriptOutcomeInvalidOutput,
		discovery_sshScriptOutcomeNonZeroExit,
		discovery_sshScriptOutcomeOutputTooLarge,
		discovery_sshScriptOutcomeTimedOut,
		discovery_sshScriptRejectedKeys,
		discovery_sshScripts
	} from '$lib/paraglide/messages';

	type SshScriptRun = components['schemas']['SshScriptRun'];
	type SshScriptOutcome = components['schemas']['SshScriptOutcome'];

	let { runs }: { runs: SshScriptRun[] } = $props();

	// The whole list, as WarningReport does: credentials number in the tens and this shares its
	// cache entry. A viewer who cannot read credentials sees the address alone.
	const credentialsQuery = useCredentialsQuery();
	let credentialsData = $derived(credentialsQuery.data ?? []);

	// Exhaustive over the generated union, so an outcome added on the backend fails `npm run check`
	// here rather than rendering blank.
	function outcomeLabel(outcome: SshScriptOutcome): string {
		switch (outcome) {
			case 'Applied':
				return common_applied();
			case 'NonZeroExit':
				return discovery_sshScriptOutcomeNonZeroExit();
			case 'TimedOut':
				return discovery_sshScriptOutcomeTimedOut();
			case 'InvalidOutput':
				return discovery_sshScriptOutcomeInvalidOutput();
			case 'OutputTooLarge':
				return discovery_sshScriptOutcomeOutputTooLarge();
			case 'HostKeyMismatch':
				return discovery_sshScriptOutcomeHostKeyMismatch();
			case 'AuthenticationFailed':
				return discovery_sshScriptOutcomeAuthenticationFailed();
			case 'ConnectionFailed':
				return discovery_sshScriptOutcomeConnectionFailed();
			case 'Unknown':
				return common_unknown();
		}
	}

	function outcomeColor(outcome: SshScriptOutcome): Color {
		if (outcome === 'Applied') return 'Green';
		if (outcome === 'HostKeyMismatch') return 'Yellow';
		if (outcome === 'Unknown') return 'Gray';
		return 'Red';
	}

	function title(run: SshScriptRun): string {
		const name = credentialsData.find((c) => c.id === run.credential_id)?.name;
		return name ? `${run.ip} · ${name}` : run.ip;
	}

	function seconds(ms: number): string {
		return `${(ms / 1000).toFixed(1)}s`;
	}
</script>

<InfoCard title={discovery_sshScripts()}>
	<div class="space-y-4">
		{#each runs as run, i (`${run.ip}-${run.credential_id}-${i}`)}
			<div class="space-y-1">
				<div class="flex items-center gap-2">
					<span class="text-primary font-mono text-sm">{title(run)}</span>
					<Tag label={outcomeLabel(run.outcome)} color={outcomeColor(run.outcome)} />
				</div>
				{#if run.exit_code != null}
					<InfoRow label={discovery_sshScriptExitCode()} mono>{run.exit_code}</InfoRow>
				{/if}
				{#if run.duration_ms > 0}
					<InfoRow label={common_duration()}>{seconds(run.duration_ms)}</InfoRow>
				{/if}
				{#if run.applied_keys && run.applied_keys.length > 0}
					<InfoRow label={discovery_sshScriptAppliedKeys()} mono>
						{run.applied_keys.join(', ')}
					</InfoRow>
				{/if}
				{#if run.rejected_keys && run.rejected_keys.length > 0}
					<InfoRow label={discovery_sshScriptRejectedKeys()} mono>
						{run.rejected_keys.join(', ')}
					</InfoRow>
				{/if}
				{#if run.detail}
					<InfoRow label={common_details()}>
						<pre
							class="text-secondary max-h-40 overflow-auto whitespace-pre-wrap font-mono text-xs">{run.detail}</pre>
					</InfoRow>
				{/if}
			</div>
		{/each}
	</div>
</InfoCard>
