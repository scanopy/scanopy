<!--
	One SSH credential's script runs in a scan, one entry per host it ran on. Rendered inside that
	credential's row on the run's Credentials tab, below the row's warnings, so it names the address
	and not the credential. A failed run is explained by its warning; this adds what the run itself
	recorded.
-->
<script lang="ts">
	import InfoRow from '$lib/shared/components/data/InfoRow.svelte';
	import type { components } from '$lib/api/schema';
	import { formatMillisAsSeconds } from '$lib/shared/utils/formatting';
	import {
		common_details,
		common_duration,
		discovery_sshScriptAppliedKeys,
		discovery_sshScriptExitCode,
		discovery_sshScriptRejectedKeys
	} from '$lib/paraglide/messages';

	type SshScriptRun = components['schemas']['SshScriptRun'];

	let { runs }: { runs: SshScriptRun[] } = $props();
</script>

<div class="space-y-4">
	{#each runs as run, i (`${run.ip}-${i}`)}
		<div class="space-y-1">
			<!-- The collapsed row already shows the address and the outcome; the address is repeated
			     here only to tell several runs apart. -->
			{#if runs.length > 1}
				<p class="text-primary font-mono text-sm">{run.ip}</p>
			{/if}
			{#if run.exit_code != null}
				<InfoRow label={discovery_sshScriptExitCode()} mono>{run.exit_code}</InfoRow>
			{/if}
			{#if run.duration_ms > 0}
				<InfoRow label={common_duration()}>{formatMillisAsSeconds(run.duration_ms)}</InfoRow>
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
