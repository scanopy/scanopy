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
	import { untrack } from 'svelte';
	import { ChevronDown, ChevronRight } from 'lucide-svelte';
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import { daemonTooOldForCredential } from '$lib/features/credentials/utils/versionGate';
	import {
		groupIntegrationsByCategory,
		groupTypesByIntegration,
		initiallyExpandedIntegrationIds,
		selectedTypeCount
	} from '$lib/features/credentials/utils/integrationPicker';
	import {
		daemons_integrationsSubtitle,
		credentials_integrationSelectedCount,
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

	// Rows holding a selection on open start expanded, so a preselected type is visible.
	// After that, expansion is the user's alone.
	let expandedIds = $state<string[]>(
		untrack(() => initiallyExpandedIntegrationIds(groups, checkedTypeIds))
	);

	function toggleExpanded(id: string) {
		expandedIds = expandedIds.includes(id)
			? expandedIds.filter((x) => x !== id)
			: [...expandedIds, id];
	}

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

<!-- A selectable type: a native checkbox and the type's display, the same checkbox the entity
     cards and tables use. The wrapper carries the tooltip so a disabled (locked or
     version-incompatible) type still shows the reason on hover. -->
{#snippet typeOption(type: CredType, singleTypeGroup: Group | null)}
	{@const locked = isDisabled(type)}
	<span class="block" data-tooltip={disabledReason(type)} use:tooltip>
		<label
			class="flex w-full items-center gap-3 {locked
				? 'cursor-not-allowed opacity-60'
				: 'cursor-pointer'}"
		>
			<input
				type="checkbox"
				class="checkbox-card h-4 w-4 flex-shrink-0"
				checked={checkedTypeIds.includes(type.id)}
				disabled={locked}
				onchange={() => toggleType(type)}
			/>
			<div class="min-w-0 flex-1">
				{#if singleTypeGroup}
					<ListSelectItem
						item={singleTypeGroup}
						displayComponent={IntegrationRowDisplay}
						context={{}}
						staticTags={true}
					/>
				{:else}
					<ListSelectItem
						item={type}
						displayComponent={CredentialTransportDisplay}
						context={{}}
						staticTags={true}
					/>
				{/if}
			</div>
		</label>
	</span>
{/snippet}

<div class="flex min-h-0 flex-1 flex-col overflow-auto p-4 sm:p-6">
	<p class="text-secondary mb-4 text-sm">{daemons_integrationsSubtitle()}</p>

	<div class="flex flex-col gap-6">
		{#each sections as section (section.category)}
			<section class="flex flex-col gap-2">
				<h3 class="text-secondary text-xs font-semibold uppercase tracking-wide">
					{section.category}
				</h3>
				{#each section.groups as group (group.integration.id)}
					<div class="card card-static rounded-lg border p-3">
						{#if group.types.length === 1}
							<!-- A single-type integration has nothing to expand: the row is the type. -->
							{@render typeOption(group.types[0], group)}
						{:else}
							{@const expanded = expandedIds.includes(group.integration.id)}
							{@const count = selectedTypeCount(group, checkedTypeIds)}
							<button
								type="button"
								onclick={() => toggleExpanded(group.integration.id)}
								aria-expanded={expanded}
								class="flex w-full items-center gap-3 text-left"
							>
								<div class="min-w-0 flex-1">
									<ListSelectItem
										item={group}
										displayComponent={IntegrationRowDisplay}
										context={{}}
										staticTags={true}
									/>
								</div>
								{#if count > 0}
									<Tag label={credentials_integrationSelectedCount({ count })} color="Blue" pill />
								{/if}
								{#if expanded}
									<ChevronDown class="text-secondary h-4 w-4 flex-shrink-0" />
								{:else}
									<ChevronRight class="text-secondary h-4 w-4 flex-shrink-0" />
								{/if}
							</button>

							{#if expanded}
								<div class="card-divider-h mt-3 flex flex-col gap-3 pt-3">
									{#each group.types as type (type.id)}
										{@render typeOption(type, null)}
									{/each}
								</div>
							{/if}
						{/if}
					</div>
				{/each}
			</section>
		{/each}
	</div>
</div>
