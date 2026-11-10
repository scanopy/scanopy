<!--
	A completed run's warnings, grouped by what they ask of the reader.

	This replaced a single amber callout holding one bulleted paragraph per warning. Fourteen of
	them, in emission order, with a broken credential that needs re-entering sitting at the same
	weight as a device quirk nobody can act on — and four near-identical paragraphs about the same
	SNMP view on four different switches. Nothing was wrong with the sentences; they had nowhere to
	sit.

	Two moves fix it, and they are separate.

	**The cards are the instruction.** `WarningRemedy` on the backend files every code under one of
	four rungs, and the card a row appears in is what the reader is being asked to do about it.
	Severity is deliberately not the cut: it measures what the scan lost, which is a different
	question, and it puts a thirty-second credential fix at the same weight as an LLDP table that
	will never parse. It stays on the row as the icon and colour, which is what it was written for.

	**A row is one code, and inside it one sentence per distinct statement.** The wall of text came
	from every occurrence carrying its own copy of the explanation. Four switches restricting the
	same view are now one row and one sentence naming four devices; a fifth that restricted a
	different capability keeps a sentence of its own. `warnings.ts` decides that generically, by
	comparing the slots that are not aggregates.

	Nesting is carried by the card and by indentation: an explanation lines up under the title of the
	row it belongs to, past the chevron that opened it.
-->
<script lang="ts">
	import { ChevronRight } from '@lucide/svelte';
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';

	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import InlineDanger from '$lib/shared/components/feedback/InlineDanger.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import CodeContainer from '$lib/shared/components/data/CodeContainer.svelte';
	import CollapsibleCard from '$lib/shared/components/data/CollapsibleCard.svelte';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import SegmentedControl from '$lib/shared/components/forms/SegmentedControl.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import { SimpleOptionDisplay } from '$lib/shared/components/forms/selection/display/SimpleOptionDisplay';
	import { entityRef } from '$lib/shared/components/data/types';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { discoveryTerminalReasons, entities } from '$lib/shared/stores/metadata';
	import { createColorHelper, createIconComponent } from '$lib/shared/utils/styling';
	import {
		common_warnings,
		discovery_daemonLogsHint,
		discovery_daemonLogsTitle,
		discovery_noWarnings,
		discovery_noWarningsBeforeStop,
		discovery_noWarningsSubtitle,
		discovery_snmpCheckFor,
		discovery_warningAffectsAddresses,
		discovery_warningAffectsHosts,
		discovery_warningAffectsSubnets
	} from '$lib/paraglide/messages';
	import type { EntityDiscriminants } from '$lib/api/entities';
	import type { DiscoveryUpdatePayload } from '../../types/api';
	import { runOutcomeReason } from '../../utils/outcome';
	import daemonVersionGates from '$lib/data/daemon-version-gates.json';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { daemonVersionAtOrBelow, runLogCommand } from '$lib/features/daemons/utils';
	import {
		buildWarningReport,
		isCredentialWarning,
		type WarningAffects,
		type WarningSubject
	} from '../../utils/warnings';
	import { actionFor, type WarningRunContext } from '../../utils/warningActions';

	interface Props {
		payload: DiscoveryUpdatePayload;
	}

	let { payload }: Props = $props();

	// Credential warnings are shown with their credential on the Credentials tab, whose title counts
	// them; this tab keeps the scan's own.
	let warnings = $derived((payload.warnings ?? []).filter((w) => !isCredentialWarning(w)));
	let credentialWarningCount = $derived(
		(payload.warnings ?? []).filter(isCredentialWarning).length
	);

	/**
	 * How the run ended, for a run that did not complete. This tab is where a reader looks for
	 * something wrong, so the reason leads it; the details tab names only the phase.
	 *
	 * `runOutcomeTag` resolves the reason's name and description, and falls back to the phase for
	 * a run recorded before reasons existed.
	 */
	let outcome = $derived.by(() => {
		if (payload.phase !== 'Failed' && payload.phase !== 'Cancelled') return null;
		const reason = runOutcomeReason(payload);
		return {
			// The reason leads here, where the status is already known from the row that was
			// opened. A run from before reasons existed has only its phase to show.
			label: reason?.name ?? payload.phase,
			body: [payload.error, reason?.description].filter(Boolean).join(' ') || null
		};
	});

	const daemonsQuery = useDaemonsQuery();

	/**
	 * The command that prints this run's log lines on its daemon's host, for a failure the daemon's
	 * logs explain. Only when the run's own daemon version tagged its lines with the session, and the
	 * daemon has said which OS it runs on, which picks the command. Read from the daemons list, so a
	 * daemon deleted since simply shows none.
	 */
	let logCommand = $derived.by(() => {
		if (payload.phase !== 'Failed' || !payload.reason || !payload.daemon_version) return null;
		if (!discoveryTerminalReasons.getMetadata(payload.reason).explained_by_daemon_logs) return null;
		if (
			daemonVersionAtOrBelow(
				daemonVersionGates.last_without_session_log_lines,
				payload.daemon_version
			)
		) {
			return null;
		}
		const os = (daemonsQuery.data ?? []).find((d) => d.id === payload.daemon_id)?.os;
		return os ? runLogCommand(os, payload.session_id) : null;
	});

	/**
	 * The devices a warning names, fetched by id.
	 *
	 * The LLDP/CDP resolution warnings and the integration-without-credential prompt carry one.
	 * Everything else carries an address, which this deliberately does not try to resolve back to a
	 * host: there is no address index on the hosts API, and buying names for those rows means
	 * downloading every host on the site with its nested children to serve one modal.
	 */
	let neededHostIds = $derived([
		...new Set(
			warnings.flatMap((w) => [
				...('host_id' in w && w.host_id ? [w.host_id] : []),
				...('remote_host_id' in w && w.remote_host_id ? [w.remote_host_id] : [])
			])
		)
	]);
	const hostsQuery = useHostsByIds(() => neededHostIds);
	let hostsData = $derived(hostsQuery.data ?? []);

	/**
	 * The credentials a warning names, for the chip and for the row's action.
	 *
	 * The whole list rather than a by-ids fetch: credentials number in the tens where hosts number
	 * in the thousands, this is the query the credentials tab already runs, and TanStack serves
	 * both from one cache entry. A viewer who cannot read them resolves nothing, which the lookup
	 * already treats as a normal outcome.
	 */
	const credentialsQuery = useCredentialsQuery();
	let credentialsData = $derived(credentialsQuery.data ?? []);
	/** The stored credential a warning names, for the options in its check commands. */
	let credentialOf = $derived((id: string) => credentialsData.find((c) => c.id === id));

	/**
	 * Whether an unresolved host id means "gone" rather than "not fetched yet".
	 *
	 * Only once the fetch has actually succeeded. While it is in flight the answer is unknown, and
	 * on an error it is still unknown — calling every device deleted because one request failed
	 * would be worse than saying nothing.
	 */
	let hostsResolved = $derived(hostsQuery.isSuccess);

	let nameOfEntity = $derived((type: EntityDiscriminants, id: string) => {
		// Credentials never report "gone": a viewer without permission to read them resolves
		// nothing, and labelling each one unknown for that reader would be a worse lie.
		if (type === 'Credential') return credentialsData.find((c) => c.id === id)?.name;
		const host = hostsData.find((h) => h.id === id);
		return host ? hostDisplayName(host) : hostsResolved ? null : undefined;
	});

	/**
	 * What a chip's hover card renders: the loaded entity, which carries the fields its display
	 * component reads (a host's `display_name`), or the label alone while it is still loading.
	 */
	function entityData(entity: { type: EntityDiscriminants; id: string }, label: string) {
		const loaded =
			entity.type === 'Credential'
				? credentialsData.find((c) => c.id === entity.id)
				: hostsData.find((h) => h.id === entity.id);
		return loaded ?? { id: entity.id, name: label };
	}

	// The run's daemon, which the Docker Desktop warning names. One deleted since names none.
	let runDaemon = $derived(
		(daemonsQuery.data ?? []).find((d) => d.id === payload.daemon_id) ?? null
	);
	let sections = $derived(buildWarningReport(warnings, nameOfEntity, runDaemon, credentialOf));

	/** Which rows are showing their explanation, by `WarningEntry.key`. */
	const openRows = new SvelteSet<string>();

	function toggleRow(key: string) {
		if (!openRows.delete(key)) openRows.add(key);
	}

	/** A row's header line: the one address, host or subnet it concerns, or how many. */
	function affectsLabel(affects: WarningAffects): string {
		if (affects.only) return affects.only;
		const count = affects.count;
		switch (affects.type) {
			case 'address':
				return discovery_warningAffectsAddresses({ count });
			case 'host':
				return discovery_warningAffectsHosts({ count });
			case 'subnet':
				return discovery_warningAffectsSubnets({ count });
		}
	}

	/** Past this many devices the selector is a dropdown; a segmented control would overflow. */
	const DEVICE_SEGMENTS = 6;

	/** Which device a row's command box shows, by `WarningEntry.key`. Unset means the default. */
	const selectedDevices = new SvelteMap<string, string>();

	/** The credential a device was read with, as a chip that opens it. */
	function credentialChipOf(credentialId: string | null): WarningSubject | null {
		if (!credentialId) return null;
		const label = nameOfEntity('Credential', credentialId);
		return label ? { label, entity: { type: 'Credential', id: credentialId } } : null;
	}

	/**
	 * The run the rows' actions read their ids from. The historical row carries no parent, but the
	 * payload does. It is absent on records written before the field existed, and it dangles for a
	 * rescan, whose discovery is deleted the moment the run finishes.
	 */
	let run = $derived<WarningRunContext>({
		daemonId: payload.daemon_id,
		siteId: payload.site_id,
		scanId: payload.discovery_type.type === 'Rescan' ? null : (payload.discovery_id ?? null),
		credentialOf
	});
</script>

<!--
	A named entity is a real entity, so its chip is drawn the way one is drawn everywhere — and its
	icon and colour come from the entity metadata rather than being fixed to Host, which is what
	lets a credential use the same snippet instead of a near-copy of it.
-->
{#snippet entityChip(subject: WarningSubject)}
	{#if subject.entity}
		{@const type = subject.entity.type}
		<EntityTag
			entityRef={entityRef(type, subject.entity.id, entityData(subject.entity, subject.label))}
			label={subject.label}
			icon={entities.getIconComponent(type)}
			color={entities.getColorHelper(type).color}
		/>
	{:else}
		<!--
			Nothing to point at, so no colour, no hover state and no click: a bare address, or a
			device deleted since the scan. The decision lives here rather than at each call site
			because it used to live at one of the three — the evidence lines rendered a chip
			unconditionally, and a subject with no entity fell through to an EntityTag carrying an
			empty id, which is a link to nowhere dressed as a device.
		-->
		<Tag label={subject.label} color={null} />
	{/if}
{/snippet}

{#if outcome}
	{#if payload.phase === 'Failed'}
		<InlineDanger title={outcome.label} body={outcome.body} />
	{:else}
		<InlineWarning title={outcome.label} body={outcome.body} />
	{/if}
{/if}

{#if logCommand}
	<div class="space-y-1">
		<p class="text-sm font-medium text-primary">{discovery_daemonLogsTitle()}</p>
		<p class="text-xs text-tertiary">{discovery_daemonLogsHint()}</p>
		<CodeContainer code={logCommand} language="bash" expandable={false} />
	</div>
{/if}

{#if sections.length === 0}
	<!-- "Nothing needs you" is only true when the Credentials tab has nothing either. -->
	<EmptyState
		title={discovery_noWarnings()}
		subtitle={outcome
			? discovery_noWarningsBeforeStop()
			: credentialWarningCount === 0
				? discovery_noWarningsSubtitle()
				: undefined}
	/>
{:else}
	<div class="space-y-4" aria-label={common_warnings()}>
		{#each sections as section (section.remedy)}
			<CollapsibleCard
				title={section.title}
				description={section.description}
				icon={createIconComponent(section.icon)}
			>
				{#snippet badge()}
					<Tag label={String(section.entries.length)} color="Amber" pill />
				{/snippet}

				<!--
					One line per row: what it is, who it concerns, and its action, so the action sits
					beside the thing it fixes. Opening a row shows the explanation, then every device
					it names.
				-->
				<ul class="divide-y divide-gray-200 dark:divide-gray-700">
					{#each section.entries as entry (entry.key)}
						{@const EntryIcon = createIconComponent(entry.icon)}
						{@const colors = createColorHelper(entry.color)}
						{@const action = actionFor(entry, run)}
						{@const isOpen = openRows.has(entry.key)}
						<li class="py-1.5">
							<div class="flex min-w-0 items-center gap-2">
								<button
									type="button"
									class="hover:bg-tertiary/40 -mx-1 flex min-w-0 cursor-pointer items-center gap-2 rounded-sm px-1 py-1 text-left"
									aria-expanded={isOpen}
									onclick={() => toggleRow(entry.key)}
								>
									<ChevronRight
										class="h-4 w-4 shrink-0 text-secondary transition-transform {isOpen
											? 'rotate-90'
											: ''}"
									/>
									<EntryIcon class="h-4 w-4 shrink-0 {colors.icon}" />
									<span class="min-w-0 truncate text-sm text-primary">{entry.title}</span>
								</button>

								<!-- Who the row concerns, as a count: the open row says the rest. -->
								<div class="flex min-w-0 flex-1 items-center overflow-hidden">
									{#if entry.affects}
										<span class="truncate text-xs text-tertiary">{affectsLabel(entry.affects)}</span
										>
									{/if}
								</div>

								{#if action}
									{@const ActionIcon = action.icon}
									<button
										type="button"
										class="btn-secondary flex max-w-56 shrink-0 items-center gap-1 py-1 text-xs"
										title={action.label}
										onclick={action.run}
									>
										<ActionIcon class="h-3.5 w-3.5 shrink-0" />
										<span class="truncate">{action.label}</span>
									</button>
								{/if}
							</div>

							{#if isOpen}
								<div class="space-y-2 pt-1 pb-1 pl-12">
									<!-- A row with commands takes one device at a time: its own result and its
									     command. Any other row lists its statements, naming every device. -->
									{#if entry.devices.length === 0}
										{#each entry.details as statement, i (i)}
											<div class="space-y-1">
												<p class="text-sm text-tertiary">{statement.sentence}</p>
												{#if statement.examples.length > 0}
													<ul class="space-y-0.5">
														{#each statement.examples as example, j (j)}
															<li class="flex flex-wrap items-center gap-1 text-xs text-tertiary">
																{#if example.near}
																	{@render entityChip(example.near)}
																{/if}
																{#if example.nearText}<span>{example.nearText}</span>{/if}
																<!-- The arrow is only drawn with something on both sides: a
															     far end nothing matched, or a host deleted since the scan,
															     would otherwise leave it pointing at nothing. -->
																{#if (example.near || example.nearText) && (example.far || example.farText)}
																	<span class="text-tertiary/60" aria-hidden="true">→</span>
																{/if}
																{#if example.far}
																	{@render entityChip(example.far)}
																{/if}
																{#if example.farText}<span>{example.farText}</span>{/if}
															</li>
														{/each}
													</ul>
												{/if}
											</div>
										{/each}
									{:else}
										{@const selected =
											entry.devices.find((c) => c.address === selectedDevices.get(entry.key)) ??
											entry.devices.find((c) => 'commands' in c.outcome) ??
											entry.devices[0]}
										{@const options = entry.devices.map((c) => ({
											value: c.address,
											label: c.address
										}))}
										<div class="space-y-2 pt-1">
											{#if entry.devices.length > DEVICE_SEGMENTS}
												<div class="max-w-xs">
													<RichSelect
														selectedValue={selected.address}
														{options}
														onSelect={(address) => selectedDevices.set(entry.key, address)}
														displayComponent={SimpleOptionDisplay}
													/>
												</div>
											{:else if entry.devices.length > 1}
												<SegmentedControl
													{options}
													selected={selected.address}
													onchange={(address) => selectedDevices.set(entry.key, address)}
												/>
											{/if}
											<p class="text-sm text-tertiary">{selected.sentence}</p>
											{#if 'commands' in selected.outcome}
												{@const credential = credentialChipOf(selected.credentialId)}
												<div class="space-y-1">
													<p class="flex flex-wrap items-center gap-1 text-xs text-secondary">
														<span>{discovery_snmpCheckFor({ address: selected.address })}</span>
														{#if credential}{@render entityChip(credential)}{/if}
													</p>
													<CodeContainer
														code={selected.outcome.commands.join('\n')}
														language="bash"
														expandable={false}
													/>
												</div>
											{:else}
												<p class="text-xs text-tertiary">{selected.outcome.reason}</p>
											{/if}
										</div>
									{/if}
								</div>
							{/if}
						</li>
					{/each}
				</ul>
			</CollapsibleCard>
		{/each}
	</div>
{/if}
