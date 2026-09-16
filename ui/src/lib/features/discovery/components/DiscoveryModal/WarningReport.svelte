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
	import { ChevronRight } from 'lucide-svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';

	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import CollapsibleCard from '$lib/shared/components/data/CollapsibleCard.svelte';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { entityRef } from '$lib/shared/components/data/types';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { navigateToEntity, openModal } from '$lib/shared/stores/modal-registry';
	import { entities } from '$lib/shared/stores/metadata';
	import { createColorHelper, createIconComponent } from '$lib/shared/utils/styling';
	import type { IconComponent } from '$lib/shared/utils/types';
	import {
		common_credentials,
		common_warnings,
		daemons_upgradeDaemon,
		discovery_noWarnings,
		discovery_noWarningsSubtitle,
		discovery_scanSettings,
		subnets_resolveRange
	} from '$lib/paraglide/messages';
	import type { EntityDiscriminants } from '$lib/api/entities';
	import type { DiscoveryUpdatePayload } from '../../types/api';
	import {
		buildWarningReport,
		credentialIdsOf,
		type WarningEntry,
		type WarningSubject
	} from '../../utils/warnings';

	interface Props {
		payload: DiscoveryUpdatePayload;
	}

	let { payload }: Props = $props();

	let warnings = $derived(payload.warnings ?? []);

	/**
	 * The devices a warning names, fetched by id.
	 *
	 * Only the LLDP/CDP resolution warnings carry one — everything else carries an address, which
	 * this deliberately does not try to resolve back to a host: there is no address index on the
	 * hosts API, and buying names for those rows means downloading every host on the network with
	 * its nested children to serve one modal.
	 */
	let neededHostIds = $derived([
		...new Set(
			warnings.flatMap((w) => [
				...('host_id' in w ? [w.host_id] : []),
				...('remote_host_id' in w ? [w.remote_host_id] : [])
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

	let sections = $derived(buildWarningReport(warnings, nameOfEntity));

	/** Which rows are showing their explanation. Keyed by code, which is unique within a run. */
	const openRows = new SvelteSet<string>();

	function toggleRow(code: string) {
		if (!openRows.delete(code)) openRows.add(code);
	}

	/** The codes whose answer is a scan setting rather than a credential. */
	const SCAN_SETTING_CODES = new Set<WarningEntry['code']>([
		'ScanTimeLimit',
		'ScanTimeLimitWithEstimate',
		'WarningsTruncated'
	]);

	/**
	 * Whether the run still has a scan to send the reader to.
	 *
	 * The historical row carries no parent, but the payload does. It is absent on records written
	 * before the field existed, and it dangles for a rescan — that discovery is deleted the moment
	 * the run finishes.
	 */
	let scanSettingsTarget = $derived(
		payload.discovery_type.type === 'Rescan' ? null : (payload.discovery_id ?? null)
	);

	/**
	 * Where a row hands off to, when Scanopy owns the fix.
	 *
	 * Only four destinations are real, and a row without one renders no button rather than a dead
	 * one. The credential family is recognised by the `integration` field on the payload rather
	 * than by a list of code names here — those eleven variants are exactly the ones that carry it,
	 * so the set cannot drift out of step with the backend.
	 */
	function actionFor(
		entry: WarningEntry
	): { label: string; icon: IconComponent; run: () => void } | null {
		if (entry.code === 'OutdatedDaemonFormat') {
			return {
				label: daemons_upgradeDaemon(),
				icon: createIconComponent('circle-arrow-up'),
				run: () => {
					// The daemon to upgrade is the one that ran this scan, so the id comes off the
					// payload rather than the warning — one session has exactly one daemon, and
					// repeating it per warning would let the two disagree.
					window.location.hash = 'daemons';
					openModal('upgrade-daemon', { id: payload.daemon_id });
				}
			};
		}

		if (entry.code === 'ProvisionalSubnetInferred') {
			const subnetIds = entry.warnings.flatMap((w) => ('subnet_id' in w ? [w.subnet_id] : []));
			return {
				label: subnets_resolveRange(),
				icon: createIconComponent('cloud-alert'),
				run: () => {
					window.location.hash = 'subnets';
					// One range resolves in place; several is a trip to the list, because the
					// four-way choice is made per subnet and there is no "all of them" answer.
					if (subnetIds.length === 1) openModal('provisional-range', { id: subnetIds[0] });
				}
			};
		}

		const scanId = scanSettingsTarget;
		if (scanId && SCAN_SETTING_CODES.has(entry.code)) {
			return {
				label: discovery_scanSettings(),
				icon: createIconComponent('sliders-horizontal'),
				run: () => {
					window.location.hash = 'discovery-scans';
					// Max Discovery Duration lives on Detection, not Performance.
					openModal('discovery-editor', { id: scanId, tab: 'detection' });
				}
			};
		}

		if (entry.warnings.some((w) => 'integration' in w)) {
			const [credentialId] = credentialIdsOf(entry);
			return {
				label: common_credentials(),
				icon: createIconComponent('key-round'),
				run: () => {
					// Any known id opens that credential's editor. Unlike an inferred range above,
					// where the four-way choice is genuinely per subnet, every credential on this
					// row needs the same thing done to it — so landing on the first is a head start
					// on the list, and the row's chips reach the others directly. Only a row with
					// no id at all — written before ids were carried, or posted by a daemon that
					// predates them — still goes to the list.
					if (!credentialId) {
						window.location.hash = 'credentials';
						return;
					}
					// The credential itself where it has loaded, so the editor opens on the id
					// alone rather than waiting for the credentials tab's own query to settle —
					// `resolveModalDeepLink` falls back to `entityData` for exactly this.
					navigateToEntity(
						'Credential',
						credentialId,
						credentialsData.find((c) => c.id === credentialId)
					);
				}
			};
		}

		return null;
	}
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
			entityRef={entityRef(type, subject.entity.id, {
				id: subject.entity.id,
				name: subject.label
			})}
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

{#if sections.length === 0}
	<EmptyState title={discovery_noWarnings()} subtitle={discovery_noWarningsSubtitle()} />
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

				<ul>
					{#each section.entries as entry (entry.code)}
						{@const EntryIcon = createIconComponent(entry.icon)}
						{@const colors = createColorHelper(entry.color)}
						{@const action = actionFor(entry)}
						{@const isOpen = openRows.has(entry.code)}
						<li>
							<div class="min-w-0">
								<div class="flex items-start gap-2">
									<button
										type="button"
										class="hover:bg-tertiary/40 -mx-1 flex min-w-0 flex-1 cursor-pointer items-start gap-2 rounded px-1 py-1 text-left"
										aria-expanded={isOpen}
										onclick={() => toggleRow(entry.code)}
									>
										<ChevronRight
											class="text-secondary mt-0.5 h-4 w-4 shrink-0 transition-transform {isOpen
												? 'rotate-90'
												: ''}"
										/>
										<EntryIcon class="mt-0.5 h-4 w-4 shrink-0 {colors.icon}" />
										<span class="text-primary shrink-0 text-sm">{entry.title}</span>
										{#if entry.subjects.length > 0}
											<span class="flex flex-wrap items-center gap-1">
												{#each entry.subjects as subject (subject.label)}
													{@render entityChip(subject)}
												{/each}
											</span>
										{/if}
									</button>
									{#if action}
										{@const ActionIcon = action.icon}
										<button
											type="button"
											class="btn-secondary flex shrink-0 items-center gap-1 py-1 text-xs"
											onclick={action.run}
										>
											<ActionIcon class="h-3.5 w-3.5" />
											<span>{action.label}</span>
										</button>
									{/if}
								</div>

								{#if isOpen}
									<div class="space-y-2 pb-2 pl-6">
										{#each entry.details as statement, i (i)}
											<div class="space-y-1">
												<p class="text-tertiary text-sm">{statement.sentence}</p>
												{#if statement.examples.length > 0}
													<ul class="space-y-0.5">
														{#each statement.examples as example, j (j)}
															<li class="text-tertiary flex flex-wrap items-center gap-1 text-xs">
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
									</div>
								{/if}
							</div>
						</li>
					{/each}
				</ul>
			</CollapsibleCard>
		{/each}
	</div>
{/if}
