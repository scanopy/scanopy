<script lang="ts">
	import type { HostFormData } from '#lib/features/hosts/types/base.ts';
	import type { AttributeSource } from '#lib/shared/utils/attribute-source.ts';
	import AttributeSourceTag from '#lib/shared/components/data/AttributeSourceTag.svelte';
	import Tag from '#lib/shared/components/data/Tag.svelte';
	import type { TagProps } from '#lib/shared/components/data/types.ts';
	import { type HostOs } from '#lib/features/hosts/host-os.ts';
	import OsTag from '../../OsTag.svelte';
	import DeviceTypeLabel from '../../DeviceTypeLabel.svelte';
	import EditableFactRow from './EditableFactRow.svelte';
	import { EDITABLE_TEXT_FACTS, deviceLabelDefaults } from './device-labels';
	import { deviceTypeSourceTag } from '#lib/features/hosts/device-type-source.ts';
	import { sysContactEmail } from '#lib/features/hosts/sys-contact.ts';
	import EntityTag from '#lib/shared/components/data/EntityTag.svelte';
	import CopyableValue from '#lib/shared/components/data/CopyableValue.svelte';
	import GuestStateTag from '#lib/shared/components/virtualization/GuestStateTag.svelte';
	import { entityRef, type EntityRef } from '#lib/shared/components/data/types.ts';
	import type { Color } from '#lib/shared/utils/styling.ts';
	import {
		containerNetworkTypes,
		deviceTypes,
		entities,
		entitySources,
		guestStates,
		hostVirtualizations,
		proxmoxGuestTypes
	} from '#lib/shared/stores/metadata.ts';
	import { hostDisplayName } from '#lib/features/hosts/host-display-name.ts';
	import { hostIcon } from '#lib/features/hosts/host-icon.ts';
	import type { IconComponent } from '#lib/shared/utils/types.ts';
	import type { components } from '#lib/api/schema';
	import { interfaceDisplayName } from '#lib/features/hosts/interface-display-name.ts';
	import { virtualizationSummary } from '#lib/features/hosts/host-virtualization.ts';
	import { useServicesByIds } from '#lib/features/services/queries.ts';
	import { useHostSummariesQuery } from '#lib/features/hosts/queries.ts';
	import { useInterfacesQuery } from '#lib/features/interfaces/queries.ts';
	import {
		collectionLookup,
		entityFieldLabel,
		entityFields,
		fieldSource,
		fieldText
	} from '#lib/shared/entity-fields/index.ts';
	import {
		common_hardware,
		common_hypervisor,
		common_interface,
		common_manager,
		common_presentedBy,
		common_runtime,
		common_source,
		common_status,
		common_virtualization,
		hosts_deviceFacts_composeProject,
		hosts_deviceFacts_containerId,
		hosts_deviceFacts_containerName,
		hosts_deviceFacts_firmwareGroup,
		hosts_deviceFacts_guestName,
		hosts_deviceFacts_guestType,
		hosts_deviceFacts_identityGroup,
		hosts_deviceFacts_locationGroup,
		hosts_deviceFacts_networkType,
		hosts_deviceFacts_vmId,
		hosts_deviceType
	} from '#lib/paraglide/messages.js';

	let {
		form,
		host,
		column
	}: {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any };
		host: HostFormData;
		/**
		 * Which of the Details tab's two columns this instance draws. The left one sits under the
		 * name card, so it starts heavier.
		 */
		column: 'left' | 'right';
	} = $props();

	type DeviceType = components['schemas']['DeviceType'];

	interface Fact {
		label: string;
		value: string | null | undefined;
		/** `null` for a field that records no provenance, which the tag then says. */
		source: AttributeSource | null | undefined;
		/**
		 * The model keeps no per-field provenance for this value at all, so no source tag is drawn.
		 * Distinct from a `null` source, which is a field that could carry one and has none.
		 */
		sourceless?: boolean;
		/**
		 * A machine identifier a user copies or compares character by character (serial, OID,
		 * chassis id, revision, VM or container id), drawn as a copyable mono chip. Names and prose
		 * (hostname, model, vendor, location) stay plain text.
		 */
		identifier?: boolean;
		link?: boolean;
		/** Drawn as a mailto link to this address, with `value` as its text. */
		mailto?: string | null;
		/** Drawn as an OS tag rather than text. */
		os?: HostOs;
		/** Drawn as a clickable entity tag labelled with `value`, with `icon` when the entity has its own. */
		entity?: { ref: EntityRef; color: Color; icon?: IconComponent };
		/** Drawn as the device type's glyph beside `value`, its name. */
		deviceType?: DeviceType;
		/** Drawn as the guest's run-state chip. */
		guestState?: string;
		/** Drawn as the chip for how the host came to exist, the Hosts table's Source column. */
		entitySource?: components['schemas']['EntitySource']['type'];
		/** Drawn in place of the source tag, for a value whose provenance is not an attribute source. */
		sourceTag?: TagProps;
		/** A fact a person can set, edited in place through its form field. */
		editable?: { name: string; kind: 'text' | 'deviceType'; initial: string };
	}

	interface Section {
		title: string;
		/** One line under the title saying what the section's values add up to. */
		summary?: string;
		facts: Fact[];
	}

	// The owner is the host that runs the service virtualizing this one: the hypervisor of a VM,
	// the runtime host of a container, the host presenting a network identity.
	// The guest interface that presents a network identity, from the interfaces the hosts query
	// cached; absent when that host is not loaded.
	const interfacesQuery = useInterfacesQuery();
	let presentingInterface = $derived(
		host.virtualization_interface_id
			? (interfacesQuery.data?.find((i) => i.id === host.virtualization_interface_id) ?? null)
			: null
	);

	const ownerServiceQuery = useServicesByIds(() =>
		host.virtualization_service_id ? [host.virtualization_service_id] : []
	);
	let ownerService = $derived(ownerServiceQuery.data?.[0] ?? null);
	let ownerHostId = $derived(ownerService?.host_id ?? null);
	const ownerHostQuery = useHostSummariesQuery(() => ({
		ids: ownerHostId ? [ownerHostId] : []
	}));
	let ownerHost = $derived(ownerHostQuery.data?.items.find((h) => h.id === ownerHostId) ?? null);

	// The attributed host facts come from the entity field registry: label, value, provenance and
	// whether a value reads as an identifier. This section only groups and lays them out.
	const HOST_FIELDS = new Map(entityFields('Host').map((field) => [field.id, field]));
	const NO_RELATIONS = collectionLookup({});
	function registryFact(id: string, extras: Partial<Fact> = {}): Fact {
		const field = HOST_FIELDS.get(id);
		if (!field) return { label: id, value: undefined, source: undefined, ...extras };
		return {
			label: entityFieldLabel('Host', field),
			value: fieldText(field, host, NO_RELATIONS) || undefined,
			source: fieldSource(field, host),
			identifier: field.format.type === 'Identifier',
			...extras
		};
	}

	function sourceless(label: string, value: string | null | undefined, identifier = false): Fact {
		return { label, value, source: null, sourceless: true, identifier };
	}

	function stateFact(state: string | undefined): Fact {
		return {
			...sourceless(common_status(), state ? guestStates.getName(state) : null),
			guestState: state
		};
	}

	let virtualizationFacts = $derived.by((): Fact[] => {
		const virtualization = host.virtualization_metadata;
		if (!virtualization) return [];
		const owner = (label: string): Fact => ({
			...sourceless(label, ownerHost ? hostDisplayName(ownerHost) : null),
			entity: ownerHost
				? {
						ref: entityRef('Host', ownerHost.id, ownerHost),
						color: entities.getColorHelper('Host').color,
						icon: hostIcon(ownerHost)
					}
				: undefined
		});
		// The service doing the virtualizing, the same tag the hosts table shows. Until it loads,
		// the manager type's name stands in.
		const manager: Fact = ownerService
			? {
					...sourceless(common_manager(), ownerService.name),
					entity: {
						ref: entityRef('Service', ownerService.id, ownerService),
						color: entities.getColorHelper('Service').color
					}
				}
			: sourceless(common_manager(), hostVirtualizations.getName(virtualization.type));
		switch (virtualization.type) {
			case 'Proxmox':
			case 'Libvirt':
			case 'VCenter':
			case 'ESXi': {
				const guestType =
					virtualization.type === 'Proxmox' ? virtualization.details.guest_type : null;
				return [
					owner(common_hypervisor()),
					manager,
					sourceless(hosts_deviceFacts_guestName(), virtualization.details.vm_name),
					sourceless(hosts_deviceFacts_vmId(), virtualization.details.vm_id, true),
					sourceless(
						hosts_deviceFacts_guestType(),
						guestType ? proxmoxGuestTypes.getName(guestType) : null
					),
					...(virtualization.type === 'Proxmox' || virtualization.type === 'Libvirt'
						? [stateFact(virtualization.details.state)]
						: [])
				];
			}
			case 'Docker':
			case 'Podman': {
				const details = virtualization.details;
				return [
					owner(common_runtime()),
					manager,
					sourceless(hosts_deviceFacts_containerName(), details.container_name),
					sourceless(hosts_deviceFacts_containerId(), details.container_id, true),
					sourceless(hosts_deviceFacts_composeProject(), details.compose_project),
					sourceless(
						hosts_deviceFacts_networkType(),
						containerNetworkTypes.getName(details.network_type)
					),
					stateFact(details.state)
				];
			}
			case 'NetworkIdentity':
				return [
					owner(common_presentedBy()),
					{
						...sourceless(
							common_interface(),
							presentingInterface ? interfaceDisplayName(presentingInterface) : null
						),
						entity: presentingInterface
							? {
									ref: entityRef('Interface', presentingInterface.id, presentingInterface),
									color: entities.getColorHelper('Interface').color
								}
							: undefined
					}
				];
		}
	});

	// How the host came to exist (discovered, imported, inferred from a neighbour, or created by
	// hand). It describes the whole record, so it carries no per-field source of its own.
	let entitySourceFact = $derived.by((): Fact => ({
		...sourceless(common_source(), entitySources.getName(host.source.type)),
		entitySource: host.source.type
	}));

	// What the form opened with, so each editable row can tell a typed value from the stored one.
	let initial = $derived(deviceLabelDefaults(host));

	/** A registry fact a person can set: always shown, with a pencil that opens its field. */
	function editableFact(id: (typeof EDITABLE_TEXT_FACTS)[number], extras: Partial<Fact> = {}) {
		return registryFact(id, {
			editable: { name: id, kind: 'text', initial: initial[id] },
			...extras
		});
	}

	// Shown even when nothing identifies the host: "Unidentified" is the answer, not a missing value.
	// Its tag names the stored match's source, or what the type was derived from.
	let deviceTypeFact = $derived.by((): Fact => {
		const deviceType = host.device_type ?? 'Unidentified';
		return {
			...sourceless(hosts_deviceType(), deviceTypes.getName(deviceType)),
			deviceType,
			sourceTag: host.device_type_source ? deviceTypeSourceTag(host.device_type_source) : undefined,
			editable: { name: 'device_type', kind: 'deviceType', initial: initial.device_type }
		};
	});

	// Grouped by what each value describes, not by the protocol that carried it. A model arrives
	// from ENTITY-MIB, a controller or an industrial probe, and each value's own tag says which.
	// Facts a person can set are always listed, so there is always a pencil to set them with; the
	// rest are listed only when something reported them.
	let sections = $derived(
		(
			[
				{
					title: hosts_deviceFacts_identityGroup(),
					facts: [
						entitySourceFact,
						deviceTypeFact,
						registryFact('hostname'),
						registryFact('sys_name'),
						registryFact('chassis_id')
					]
				},
				{
					title: common_hardware(),
					facts: [
						editableFact('manufacturer'),
						editableFact('model'),
						editableFact('serial_number'),
						editableFact('asset_tag'),
						registryFact('sys_object_id')
					]
				},
				{
					title: hosts_deviceFacts_firmwareGroup(),
					facts: [
						registryFact('os_family', { os: host.os ?? undefined }),
						registryFact('firmware_revision'),
						registryFact('software_revision'),
						registryFact('sys_descr')
					]
				},
				{
					title: hosts_deviceFacts_locationGroup(),
					facts: [
						editableFact('sys_location'),
						editableFact('sys_contact', { mailto: sysContactEmail(host.sys_contact) }),
						editableFact('management_url', { link: true })
					]
				},
				{
					title: common_virtualization(),
					summary: host.virtualization_metadata
						? virtualizationSummary(
								host.virtualization_metadata,
								ownerHost ? hostDisplayName(ownerHost) : null,
								presentingInterface ? interfaceDisplayName(presentingInterface) : null
							)
						: undefined,
					facts: virtualizationFacts
				}
			] satisfies Section[]
		)
			.map((section): Section => ({
				...section,
				facts: section.facts.filter((fact) => fact.editable || fact.value?.trim())
			}))
			.filter((section) => section.facts.length > 0)
	);

	// Which column each section sits in. Names and identifiers opens the right column, beside the
	// name card; every other section goes to whichever column is shorter so far. Measured in rows,
	// never in pixels, and from the facts alone, so opening a pencil never moves a card.
	const NAME_CARD_ROWS = 8;
	const SECTION_CHROME_ROWS = 2;
	let placed = $derived.by(() => {
		const heights = { left: NAME_CARD_ROWS, right: 0 };
		return sections.map((section, index) => {
			const side: 'left' | 'right' = index === 0 || heights.right < heights.left ? 'right' : 'left';
			heights[side] += section.facts.length + SECTION_CHROME_ROWS;
			return { section, side };
		});
	});
</script>

{#snippet factValue(fact: Fact)}
	{#if !fact.value?.trim()}
		<span class="text-secondary">—</span>
	{:else if fact.entitySource}
		<Tag
			{...entitySources.getTag(fact.entitySource)}
			icon={entitySources.getIconComponent(fact.entitySource)}
			title={entitySources.getDescription(fact.entitySource)}
		/>
	{:else if fact.guestState}
		<GuestStateTag state={fact.guestState} />
	{:else if fact.entity}
		<EntityTag
			entityRef={fact.entity.ref}
			label={fact.value ?? undefined}
			icon={fact.entity.icon ?? entities.getIconComponent(fact.entity.ref.entityType)}
			color={fact.entity.color}
		/>
	{:else if fact.deviceType}
		<DeviceTypeLabel deviceType={fact.deviceType} />
	{:else if fact.os}
		<!-- The same tooltip the hosts table shows: every detail and the source. -->
		<OsTag os={fact.os} source={fact.source} />
	{:else if fact.mailto}
		<a href="mailto:{fact.mailto}" class="break-all text-blue-400 hover:text-blue-300">
			{fact.value}
		</a>
	{:else if fact.link}
		<!-- eslint-disable svelte/no-navigation-without-resolve -->
		<a
			href={fact.value}
			target="_blank"
			rel="external noopener noreferrer"
			class="break-all text-blue-400 hover:text-blue-300"
		>
			{fact.value}
		</a>
		<!-- eslint-enable svelte/no-navigation-without-resolve -->
	{:else if fact.identifier && fact.value}
		<CopyableValue value={fact.value} variant="inline" />
	{:else}
		{fact.value}
	{/if}
{/snippet}

{#snippet factSource(fact: Fact)}
	{#if fact.sourceTag}
		<Tag {...fact.sourceTag} />
	{:else if !fact.sourceless && fact.value?.trim()}
		<AttributeSourceTag source={fact.source} />
	{/if}
{/snippet}

<!-- This column's section cards, one per section. -->
{#each placed.filter((p) => p.side === column) as { section } (section.title)}
	<div class="card-static card space-y-2">
		<h4 class="text-xs font-semibold tracking-wide text-secondary uppercase">
			{section.title}
		</h4>
		{#if section.summary}
			<p class="text-sm text-secondary">{section.summary}</p>
		{/if}
		{#each section.facts as fact (fact.label)}
			{#if fact.editable}
				<EditableFactRow
					{form}
					name={fact.editable.name}
					label={fact.label}
					kind={fact.editable.kind}
					initial={fact.editable.initial}
					hostId={host.id}
				>
					{#snippet display()}{@render factValue(fact)}{/snippet}
					{#snippet source()}{@render factSource(fact)}{/snippet}
				</EditableFactRow>
			{:else}
				<div class="flex flex-wrap items-center gap-x-4 gap-y-1">
					<span class="w-40 shrink-0 text-sm text-secondary">{fact.label}</span>
					<span class="min-w-0 flex-1 text-sm wrap-break-word text-primary">
						{@render factValue(fact)}
					</span>
					<span class="shrink-0">{@render factSource(fact)}</span>
				</div>
			{/if}
		{/each}
	</div>
{/each}
