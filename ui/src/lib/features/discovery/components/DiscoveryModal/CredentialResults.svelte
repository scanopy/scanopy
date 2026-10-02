<!--
	What each credential did in a run, with its warnings: the hosts it collected from, the SSH script
	runs, the hosts Wake-on-LAN woke. Every credential warning in the run is here and nowhere else;
	the Warnings tab points here. Each row starts collapsed, showing the credential, the addresses it
	reported on, and "Success" or its warning count; expanding it shows the warnings' own sentences
	and the per-host detail.
-->
<script lang="ts">
	import { ChevronRight } from 'lucide-svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { entityRef } from '$lib/shared/components/data/types';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { entities } from '$lib/shared/stores/metadata';
	import {
		common_credential,
		common_moreItems,
		common_success,
		common_unknownEntity,
		discovery_credentialCollectedHosts,
		discovery_credentialWarningCount,
		discovery_credentialWokeHosts,
		discovery_noCredentialResults,
		discovery_noCredentialResultsSubtitle,
		discovery_wolAnswered,
		discovery_wolDidNotAnswer
	} from '$lib/paraglide/messages';
	import type { EntityDiscriminants } from '$lib/api/entities';
	import type { DiscoveryUpdatePayload } from '../../types/api';
	import { credentialResultRows } from '../../utils/credentialResults';
	import { buildWarningReport, integration } from '../../utils/warnings';
	import SshScriptRuns from './SshScriptRuns.svelte';

	interface Props {
		payload: DiscoveryUpdatePayload;
		/** The daemon that ran the scan, named by the daemon-OS warning. */
		daemonName: string | null;
	}

	let { payload, daemonName }: Props = $props();

	/** Addresses shown on a collapsed row before the rest fold into "+N more". */
	const MAX_ADDRESSES = 5;

	let rows = $derived(
		credentialResultRows(payload.credential_results ?? [], payload.warnings ?? [])
	);

	// The whole list, as WarningReport does: credentials number in the tens and this shares its
	// cache entry. A viewer who cannot read credentials sees each row as an unknown credential.
	const credentialsQuery = useCredentialsQuery();
	let credentialsById = $derived(new Map((credentialsQuery.data ?? []).map((c) => [c.id, c])));

	let nameOfEntity = $derived((type: EntityDiscriminants, id: string) =>
		type === 'Credential' ? credentialsById.get(id)?.name : undefined
	);

	const expanded = new SvelteSet<string>();

	function toggle(key: string) {
		if (expanded.has(key)) expanded.delete(key);
		else expanded.add(key);
	}
</script>

{#if rows.length === 0}
	<EmptyState
		title={discovery_noCredentialResults()}
		subtitle={discovery_noCredentialResultsSubtitle()}
	/>
{:else}
	<ul class="space-y-3">
		{#each rows as row (row.key)}
			{@const credential = row.credentialId ? credentialsById.get(row.credentialId) : undefined}
			{@const open = expanded.has(row.key)}
			<li class="card card-static">
				<button
					type="button"
					class="flex w-full flex-wrap items-center gap-2 p-4 text-left"
					aria-expanded={open}
					onclick={() => toggle(row.key)}
				>
					<ChevronRight
						class="text-tertiary h-4 w-4 flex-shrink-0 transition-transform {open
							? 'rotate-90'
							: ''}"
					/>
					{#if credential}
						<EntityTag
							entityRef={entityRef('Credential', credential.id, credential)}
							label={credential.name}
							icon={entities.getIconComponent('Credential')}
							color={entities.getColorHelper('Credential').color}
						/>
					{:else if row.integration}
						<span class="text-secondary text-sm">{integration(row.integration)}</span>
					{:else}
						<span class="text-secondary text-sm">
							{common_unknownEntity({ entity: common_credential() })}
						</span>
					{/if}
					{#each row.addresses.slice(0, MAX_ADDRESSES) as address (address)}
						<Tag label={address} />
					{/each}
					{#if row.addresses.length > MAX_ADDRESSES}
						<Tag label={common_moreItems({ count: row.addresses.length - MAX_ADDRESSES })} />
					{/if}
					<span class="ml-auto">
						{#if row.warnings.length === 0}
							<Tag label={common_success()} color="Green" />
						{:else}
							<Tag
								label={discovery_credentialWarningCount({ count: row.warnings.length })}
								color="Amber"
							/>
						{/if}
					</span>
				</button>

				{#if open}
					<div class="space-y-4 border-t border-gray-700 p-4">
						{#each buildWarningReport(row.warnings, nameOfEntity, daemonName) as section (section.remedy)}
							{#each section.entries as entry (entry.code)}
								<div class="space-y-1">
									<p class="text-primary text-sm font-medium">{entry.title}</p>
									{#each entry.details as statement, i (i)}
										<p class="text-secondary text-sm">{statement.sentence}</p>
									{/each}
								</div>
							{/each}
						{/each}

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
									<li class="flex items-center gap-2">
										<span class="text-primary font-mono text-sm">{host.ip}</span>
										{#if host.woke}
											<Tag label={discovery_wolAnswered()} color="Green" />
										{:else}
											<Tag label={discovery_wolDidNotAnswer()} color="Red" />
										{/if}
									</li>
								{/each}
							</ul>
						{/if}
					</div>
				{/if}
			</li>
		{/each}
	</ul>
{/if}
