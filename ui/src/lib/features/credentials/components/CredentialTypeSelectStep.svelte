<script lang="ts" module>
	import { credentialIntegrations, credentialTypes } from '$lib/shared/stores/metadata';
	import type {
		TypedTypeMetadata,
		CredentialTypeMetadata,
		CredentialIntegrationMetadata
	} from '$lib/shared/stores/metadata';
	import type { EntityDisplayComponent } from '$lib/shared/components/forms/selection/types';
	import type { IntegrationGroup } from '$lib/features/credentials/utils/integrationPicker';
	import { CredentialTypeDisplay } from '$lib/shared/components/forms/selection/display/CredentialTypeDisplay.svelte';
	import { metaTransportNote } from '$lib/i18n/metadata';

	type CredType = TypedTypeMetadata<CredentialTypeMetadata>;
	type Integration = TypedTypeMetadata<CredentialIntegrationMetadata>;
	type Group = IntegrationGroup<Integration, CredType>;

	// An integration row: the integration's name, logo and "what it discovers" text. A
	// single-type row also carries that type's tags (Beta, targets), since the row is the type.
	const IntegrationRowDisplay: EntityDisplayComponent<Group, object> = {
		getId: (group) => group.integration.id,
		getLabel: (group) => credentialIntegrations.getName(group.integration.id),
		getDescription: (group) => credentialIntegrations.getDescription(group.integration.id),
		getIcon: (group) => credentialIntegrations.getIconComponent(group.integration.id),
		getIconColor: (group) => credentialIntegrations.getColorHelper(group.integration.id).icon,
		getTags: (group) =>
			group.types.length === 1 ? (CredentialTypeDisplay.getTags?.(group.types[0], {}) ?? []) : []
	};

	// A type under its integration row. The row already shows the logo and what the integration
	// discovers, so the type shows only how it connects. Standalone surfaces (the type dropdown)
	// use CredentialTypeDisplay, which carries both.
	const CredentialTransportDisplay: EntityDisplayComponent<CredType, object> = {
		getId: (type) => type.id,
		getLabel: (type) => credentialTypes.getName(type.id),
		getDescription: (type) =>
			metaTransportNote('credential_types', type.id, type.metadata?.transport_note ?? ''),
		getTags: (type) => CredentialTypeDisplay.getTags?.(type, {}) ?? []
	};
</script>

<script lang="ts">
	import { SvelteSet } from 'svelte/reactivity';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import ExpandableChildList from '$lib/shared/components/forms/selection/ExpandableChildList.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { daemonTooOldForCredential } from '$lib/features/credentials/utils/versionGate';
	import {
		groupIntegrationsByCategory,
		groupTypesByIntegration,
		selectedIntegrationCount,
		selectedTypeCount
	} from '$lib/features/credentials/utils/integrationPicker';
	import {
		daemons_integrationsSubtitle,
		credentials_integrationSelectedCount,
		credentials_integrationTypeCount,
		credentials_lockedDaemonCapability,
		credentials_requiresDaemonVersion
	} from '$lib/paraglide/messages';

	interface Props {
		/** Selected credential types. Configurable types prefill the wizard; auto-local
		 *  types (e.g. Docker socket) map to a daemon install flag. */
		selectedTypeIds: string[];
		/** Type ids rendered read-only (non-toggleable), reflecting a fixed daemon
		 *  capability — e.g. an already-installed daemon's local Docker socket. */
		lockedTypeIds?: string[];
		/** Type ids always rendered checked, independent of `selectedTypeIds` (used
		 *  for locked types reflecting a fixed capability). */
		forceCheckedTypeIds?: string[];
		/** Version of the single daemon this picker targets (the discovery modal's bound
		 *  daemon). A type is disabled when this version is older than the credential
		 *  type's `minimum_daemon_version`. `null`/absent (e.g. create-daemon flow, where
		 *  no daemon is connected yet) ⇒ no version gate. Assignment surfaces that span
		 *  many daemons don't pass this — those are handled by the backend dispatch filter. */
		daemonVersion?: string | null;
		/** Name of that daemon, used in the version-requirement tooltip. */
		daemonName?: string | null;
	}

	let {
		selectedTypeIds = $bindable([]),
		lockedTypeIds = [],
		forceCheckedTypeIds = [],
		daemonVersion = null,
		daemonName = null
	}: Props = $props();

	// One row per integration (SNMP, Docker, SSH, …), grouped on the backend's
	// `metadata.integration` key, in sections by the integration's category.
	let groups = $derived(
		groupTypesByIntegration(credentialTypes.getItems(), credentialIntegrations.getItems())
	);
	let sections = $derived(groupIntegrationsByCategory(groups, credentialIntegrations.getItems()));

	let checkedTypeIds = $derived([...new Set([...selectedTypeIds, ...forceCheckedTypeIds])]);

	// Expanded multi-type integrations. The user's alone: nothing opens by default.
	const expandedIds = new SvelteSet<string>();

	function toggleExpanded(group: Group) {
		if (group.types.length < 2) return;
		const id = group.integration.id;
		if (expandedIds.has(id)) {
			expandedIds.delete(id);
		} else {
			expandedIds.add(id);
		}
	}

	// ListManager selects integration rows; only single-type rows are selectable, and a row's
	// selection is its one type's. Forced types read as selected and are never removed here.
	function selectedGroupsIn(sectionGroups: Group[]): Group[] {
		return sectionGroups.filter(
			(g) => g.types.length === 1 && checkedTypeIds.includes(g.types[0].id)
		);
	}

	function setSelectedGroupsIn(sectionGroups: Group[], selected: Group[]) {
		const wanted = new Set(selected.map((g) => g.integration.id));
		let next = selectedTypeIds;
		for (const group of sectionGroups) {
			if (group.types.length !== 1 || isDisabled(group.types[0])) continue;
			const id = group.types[0].id;
			if (wanted.has(group.integration.id)) {
				if (!next.includes(id)) next = [...next, id];
			} else {
				next = next.filter((x) => x !== id);
			}
		}
		selectedTypeIds = next;
	}

	let integrationCount = $derived(selectedIntegrationCount(groups, checkedTypeIds));

	function isLocked(id: string): boolean {
		return lockedTypeIds.includes(id);
	}

	// The target daemon is too old for this credential type when its version is below
	// the type's `minimum_daemon_version` floor.
	function isIncompatible(type: CredType): boolean {
		return daemonTooOldForCredential(type.metadata?.minimum_daemon_version, daemonVersion);
	}

	function isDisabled(type: CredType): boolean {
		return isLocked(type.id) || isIncompatible(type);
	}

	// Disabled reason for the hover tooltip: version-incompatibility takes
	// precedence over a fixed-capability lock (a too-old daemon can't run it at all).
	function disabledReason(type: CredType): string | undefined {
		if (isIncompatible(type)) {
			return credentials_requiresDaemonVersion({
				version: type.metadata?.minimum_daemon_version ?? '',
				name: daemonName ?? ''
			});
		}
		if (isLocked(type.id)) {
			return credentials_lockedDaemonCapability({
				integration: credentialIntegrations.getName(type.metadata?.integration ?? null)
			});
		}
		return undefined;
	}

	function toggleType(type: CredType) {
		if (isDisabled(type)) return;
		const id = type.id;
		selectedTypeIds = selectedTypeIds.includes(id)
			? selectedTypeIds.filter((x) => x !== id)
			: [...selectedTypeIds, id];
	}
</script>

<div class="flex min-h-0 flex-1 flex-col overflow-auto p-4 sm:p-6">
	<div class="mb-4 flex items-start gap-3">
		<p class="text-secondary flex-1 text-sm">{daemons_integrationsSubtitle()}</p>
		{#if integrationCount > 0}
			<Tag
				label={credentials_integrationSelectedCount({ count: integrationCount })}
				color="Blue"
				pill
			/>
		{/if}
	</div>

	<!-- One ListManager per category, as in the Applications wizard's Assign step: a row is an
	     integration; a single-type row selects with its right-aligned checkbox, and a multi-type
	     row expands to its types, which carry checkboxes in the same column. -->
	<div class="flex flex-col gap-4">
		{#each sections as section (section.category)}
			<ListManager
				label={section.category}
				items={section.groups}
				itemDisplayComponent={IntegrationRowDisplay}
				optionDisplayComponent={IntegrationRowDisplay}
				allowAddFromOptions={false}
				allowReorder={false}
				allowItemEdit={() => false}
				allowItemRemove={() => false}
				allowSelection={true}
				itemClickAction="select"
				selectionIndicator="checkbox"
				showSelectAll={false}
				allowItemSelection={(group) => group.types.length === 1}
				getItemSelectionDisabledReason={(group) => disabledReason(group.types[0]) ?? null}
				bind:selectedItems={
					() => selectedGroupsIn(section.groups),
					(selected) => setSelectedGroupsIn(section.groups, selected)
				}
				onClick={(group) => toggleExpanded(group)}
			>
				{#snippet itemExpandedSnippet({ item: group })}
					{#if group.types.length > 1}
						{@const count = selectedTypeCount(group, checkedTypeIds)}
						<ExpandableChildList
							items={group.types}
							displayComponent={CredentialTransportDisplay}
							toggleLabel={credentials_integrationTypeCount({ count: group.types.length })}
							expanded={expandedIds.has(group.integration.id)}
							onToggleExpanded={() => toggleExpanded(group)}
							selection={{
								isSelected: (type) => checkedTypeIds.includes(type.id),
								onToggle: toggleType,
								disabledReason
							}}
						>
							{#snippet badge()}
								{#if count > 0}
									<Tag label={credentials_integrationSelectedCount({ count })} color="Blue" pill />
								{/if}
							{/snippet}
						</ExpandableChildList>
					{/if}
				{/snippet}
			</ListManager>
		{/each}
	</div>
</div>
