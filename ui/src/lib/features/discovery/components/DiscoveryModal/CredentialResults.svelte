<!--
	What each stored credential did in a run: hosts it collected from, the SSH script runs, the
	hosts Wake-on-LAN woke. A credential the run only warned about (skipped before it ran) gets a
	row too, so every credential the run touched is listed once. Problems stay on the Issues tab;
	each row links there with its warning count.
-->
<script lang="ts">
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { entityRef } from '$lib/shared/components/data/types';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { credentialTypes, entities } from '$lib/shared/stores/metadata';
	import { formatMillisAsSeconds } from '$lib/shared/utils/formatting';
	import {
		common_credential,
		common_unknownEntity,
		discovery_credentialCollectedHosts,
		discovery_credentialWarningCount,
		discovery_credentialWokeHosts,
		discovery_noCredentialResults,
		discovery_noCredentialResultsSubtitle,
		discovery_noWarnings,
		discovery_wolDidNotWake,
		discovery_wolWokeAfter
	} from '$lib/paraglide/messages';
	import type { DiscoveryUpdatePayload } from '../../types/api';
	import { credentialResultRows } from '../../utils/credentialResults';
	import SshScriptRuns from './SshScriptRuns.svelte';

	interface Props {
		payload: DiscoveryUpdatePayload;
		/** Switch the modal to the Issues tab. */
		onShowIssues: () => void;
	}

	let { payload, onShowIssues }: Props = $props();

	let rows = $derived(
		credentialResultRows(payload.credential_results ?? [], payload.warnings ?? [])
	);

	// The whole list, as WarningReport does: credentials number in the tens and this shares its
	// cache entry. A viewer who cannot read credentials sees each row as an unknown credential.
	const credentialsQuery = useCredentialsQuery();
	let credentialsById = $derived(new Map((credentialsQuery.data ?? []).map((c) => [c.id, c])));
</script>

{#if rows.length === 0}
	<EmptyState
		title={discovery_noCredentialResults()}
		subtitle={discovery_noCredentialResultsSubtitle()}
	/>
{:else}
	<div class="space-y-4">
		{#each rows as row (row.credentialId)}
			{@const credential = credentialsById.get(row.credentialId)}
			<div class="card card-static space-y-3 p-4">
				<div class="flex flex-wrap items-center justify-between gap-2">
					<div class="flex min-w-0 flex-wrap items-center gap-2">
						{#if credential}
							<EntityTag
								entityRef={entityRef('Credential', credential.id, credential)}
								label={credential.name}
								icon={entities.getIconComponent('Credential')}
								color={entities.getColorHelper('Credential').color}
							/>
							<span class="text-tertiary text-sm">
								{credentialTypes.getName(credential.credential_type.type)}
							</span>
						{:else}
							<span class="text-secondary text-sm">
								{common_unknownEntity({ entity: common_credential() })}
							</span>
						{/if}
					</div>
					{#if row.warningCount === 0}
						<span class="text-tertiary text-xs">{discovery_noWarnings()}</span>
					{:else}
						<button type="button" class="text-link text-xs" onclick={onShowIssues}>
							{discovery_credentialWarningCount({ count: row.warningCount })}
						</button>
					{/if}
				</div>

				{#if row.outcome?.type === 'Collected'}
					<p class="text-secondary text-sm">
						{discovery_credentialCollectedHosts({ count: row.outcome.hosts })}
					</p>
				{:else if row.outcome?.type === 'SshScript'}
					<SshScriptRuns runs={row.outcome.runs} />
				{:else if row.outcome?.type === 'WakeOnLan'}
					{@const hosts = row.outcome.hosts}
					<p class="text-secondary text-sm">
						{discovery_credentialWokeHosts({
							woke: hosts.filter((h) => h.woke).length,
							total: hosts.length
						})}
					</p>
					<ul class="space-y-1">
						{#each hosts as host, i (`${host.ip}-${i}`)}
							{@const duration = formatMillisAsSeconds(host.waited_ms)}
							<li class="flex items-center gap-2">
								<span class="text-primary font-mono text-sm">{host.ip}</span>
								{#if host.woke}
									<Tag label={discovery_wolWokeAfter({ duration })} color="Green" />
								{:else}
									<Tag label={discovery_wolDidNotWake({ duration })} color="Red" />
								{/if}
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		{/each}
	</div>
{/if}
