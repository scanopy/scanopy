<!--
	What each credential did in a run, with its warnings: the hosts it collected from, the SSH script
	runs, the hosts Wake-on-LAN woke. Every credential warning in the run is shown here rather than on
	the Scan tab. Rows use the Scan tab's density: collapsed, a row shows the credential, the
	addresses it reported on, and "Success" or its warning count; expanded, the warnings' own
	sentences and only the detail the collapsed row does not already show.
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
		discovery_noCredentialResultsSubtitle
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
	<ul class="card card-static px-4 py-2">
		{#each rows as row (row.key)}
			{@const credential = row.credentialId ? credentialsById.get(row.credentialId) : undefined}
			{@const open = expanded.has(row.key)}
			<li>
				<button
					type="button"
					class="hover:bg-tertiary/40 -mx-1 flex w-full min-w-0 cursor-pointer items-start gap-2 rounded px-1 py-1 text-left"
					aria-expanded={open}
					onclick={() => toggle(row.key)}
				>
					<ChevronRight
						class="text-secondary mt-0.5 h-4 w-4 shrink-0 transition-transform {open
							? 'rotate-90'
							: ''}"
					/>
					<span class="flex min-w-0 flex-1 flex-wrap items-center gap-1">
						{#if credential}
							<EntityTag
								entityRef={entityRef('Credential', credential.id, credential)}
								label={credential.name}
								icon={entities.getIconComponent('Credential')}
								color={entities.getColorHelper('Credential').color}
							/>
						{:else if row.integration}
							<span class="text-primary text-sm">{integration(row.integration)}</span>
						{:else}
							<span class="text-primary text-sm">
								{common_unknownEntity({ entity: common_credential() })}
							</span>
						{/if}
						{#each row.addresses.slice(0, MAX_ADDRESSES) as address (address)}
							<Tag label={address} />
						{/each}
						{#if row.addresses.length > MAX_ADDRESSES}
							<Tag label={common_moreItems({ count: row.addresses.length - MAX_ADDRESSES })} />
						{/if}
					</span>
					{#if row.warnings.length === 0}
						<Tag label={common_success()} color="Green" />
					{:else}
						<Tag
							label={discovery_credentialWarningCount({ count: row.warnings.length })}
							color="Amber"
						/>
					{/if}
				</button>

				{#if open}
					<div class="space-y-2 pb-2 pl-6">
						{#each buildWarningReport(row.warnings, nameOfEntity, daemonName) as section (section.remedy)}
							{#each section.entries as entry (entry.code)}
								<div class="space-y-1">
									<p class="text-primary text-sm">{entry.title}</p>
									{#each entry.details as statement, i (i)}
										<p class="text-tertiary text-sm">{statement.sentence}</p>
									{/each}
								</div>
							{/each}
						{/each}

						{#if row.outcome?.type === 'Collected'}
							<p class="text-tertiary text-sm">
								{discovery_credentialCollectedHosts({ count: row.outcome.hosts })}
							</p>
						{:else if row.outcome?.type === 'SshScript'}
							<SshScriptRuns runs={row.outcome.runs} />
						{:else if row.outcome?.type === 'WakeOnLan'}
							<p class="text-tertiary text-sm">
								{discovery_credentialWokeHosts({
									woke: row.outcome.hosts.filter((h) => h.woke).length,
									total: row.outcome.hosts.length
								})}
							</p>
						{/if}
					</div>
				{/if}
			</li>
		{/each}
	</ul>
{/if}
