<script lang="ts">
	import type { HostFormData } from '$lib/features/hosts/types/base';
	import type { AttributeSource } from '$lib/shared/utils/attribute-source';
	import AttributeSourceTag from '$lib/shared/components/data/AttributeSourceTag.svelte';
	import { hostOsLabel, type HostOs } from '$lib/features/hosts/host-os';
	import OsTag from '../../OsTag.svelte';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import { entityRef, type EntityRef } from '$lib/shared/components/data/types';
	import type { Color } from '$lib/shared/utils/styling';
	import {
		containerNetworkTypes,
		entities,
		hostVirtualizations,
		proxmoxGuestTypes
	} from '$lib/shared/stores/metadata';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';
	import { useServicesByIds } from '$lib/features/services/queries';
	import { useHostSummariesQuery } from '$lib/features/hosts/queries';
	import {
		common_contact,
		common_firmwareRevision,
		common_hardware,
		common_hostname,
		common_hypervisor,
		common_interface,
		common_location,
		common_manager,
		common_manufacturer,
		common_model,
		common_operatingSystem,
		common_presentedBy,
		common_runtime,
		common_serialNumber,
		common_softwareRevision,
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
		hosts_snmp_chassisId,
		hosts_snmp_managementUrl,
		hosts_snmp_sysDescr,
		hosts_snmp_sysName,
		hosts_snmp_sysObjectId
	} from '$lib/paraglide/messages';

	let { host }: { host: HostFormData } = $props();

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
		mono?: boolean;
		link?: boolean;
		/** Drawn as an OS tag rather than text. */
		os?: HostOs;
		/** Drawn as a clickable entity tag labelled with `value`. */
		entity?: { ref: EntityRef; color: Color };
	}

	// The owner is the host that runs the service virtualizing this one: the hypervisor of a VM,
	// the runtime host of a container, the host presenting a network identity.
	const ownerServiceQuery = useServicesByIds(() =>
		host.virtualization_service_id ? [host.virtualization_service_id] : []
	);
	let ownerHostId = $derived(ownerServiceQuery.data?.[0]?.host_id ?? null);
	const ownerHostQuery = useHostSummariesQuery(() => ({
		ids: ownerHostId ? [ownerHostId] : []
	}));
	let ownerHost = $derived(ownerHostQuery.data?.items.find((h) => h.id === ownerHostId) ?? null);

	function sourceless(label: string, value: string | null | undefined, mono = false): Fact {
		return { label, value, source: null, sourceless: true, mono };
	}

	let virtualizationFacts = $derived.by((): Fact[] => {
		const virtualization = host.virtualization_metadata;
		if (!virtualization) return [];
		const owner = (label: string): Fact => ({
			...sourceless(label, ownerHost ? hostDisplayName(ownerHost) : null),
			entity: ownerHost
				? {
						ref: entityRef('Host', ownerHost.id, ownerHost),
						color: entities.getColorHelper('Host').color
					}
				: undefined
		});
		const manager = sourceless(common_manager(), hostVirtualizations.getName(virtualization.type));
		switch (virtualization.type) {
			case 'Proxmox':
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
					)
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
					)
				];
			}
			case 'NetworkIdentity':
				return [
					owner(common_presentedBy()),
					sourceless(common_interface(), virtualization.details.interface, true)
				];
		}
	});

	// Grouped by what each value describes, not by the protocol that carried it. A model arrives
	// from ENTITY-MIB, a controller or an industrial probe, and each value's own tag says which.
	let sections = $derived(
		[
			{
				title: hosts_deviceFacts_identityGroup(),
				facts: [
					{
						label: common_hostname(),
						value: host.hostname,
						source: host.hostname_source,
						mono: true
					},
					{ label: hosts_snmp_sysName(), value: host.sys_name, source: host.sys_name_source },
					{
						label: hosts_snmp_chassisId(),
						value: host.chassis_id,
						source: host.chassis_id_source,
						mono: true
					}
				] satisfies Fact[]
			},
			{
				title: common_hardware(),
				facts: [
					{
						label: common_manufacturer(),
						value: host.manufacturer,
						source: host.manufacturer_source
					},
					{ label: common_model(), value: host.model, source: host.model_source, mono: true },
					{
						label: common_serialNumber(),
						value: host.serial_number,
						source: host.serial_number_source,
						mono: true
					},
					{
						label: hosts_snmp_sysObjectId(),
						value: host.sys_object_id,
						source: host.sys_object_id_source,
						mono: true
					}
				] satisfies Fact[]
			},
			{
				title: hosts_deviceFacts_firmwareGroup(),
				facts: [
					{
						label: common_operatingSystem(),
						value: host.os ? hostOsLabel(host.os) : undefined,
						source: host.os_source,
						os: host.os
					},
					{
						label: common_firmwareRevision(),
						value: host.firmware_revision,
						source: host.firmware_revision_source,
						mono: true
					},
					{
						label: common_softwareRevision(),
						value: host.software_revision,
						source: host.software_revision_source,
						mono: true
					},
					{ label: hosts_snmp_sysDescr(), value: host.sys_descr, source: host.sys_descr_source }
				] satisfies Fact[]
			},
			{
				title: hosts_deviceFacts_locationGroup(),
				facts: [
					{ label: common_location(), value: host.sys_location, source: host.sys_location_source },
					{ label: common_contact(), value: host.sys_contact, source: host.sys_contact_source },
					{
						label: hosts_snmp_managementUrl(),
						value: host.management_url,
						source: host.management_url_source,
						link: true
					}
				] satisfies Fact[]
			},
			{
				title: common_virtualization(),
				facts: virtualizationFacts
			}
		]
			.map((section) => ({
				...section,
				facts: (section.facts as Fact[]).filter((fact) => fact.value?.trim())
			}))
			.filter((section) => section.facts.length > 0)
	);
</script>

{#if sections.length > 0}
	<div class="card card-static">
		<div class="divide-y divide-gray-200 dark:divide-gray-700">
			{#each sections as section (section.title)}
				<div class="space-y-2 py-3 first:pt-0 last:pb-0">
					<h4 class="text-secondary text-xs font-semibold uppercase tracking-wide">
						{section.title}
					</h4>
					{#each section.facts as fact (fact.label)}
						<div class="flex flex-wrap items-center gap-x-4 gap-y-1">
							<span class="text-secondary w-40 shrink-0 text-sm">{fact.label}</span>
							<span
								class="text-primary min-w-0 flex-1 break-words text-sm"
								class:font-mono={fact.mono}
							>
								{#if fact.entity}
									<EntityTag
										entityRef={fact.entity.ref}
										label={fact.value ?? undefined}
										icon={entities.getIconComponent(fact.entity.ref.entityType)}
										color={fact.entity.color}
									/>
								{:else if fact.os}
									<!-- The same tooltip the hosts table shows: every detail and the source. -->
									<OsTag os={fact.os} source={fact.source} />
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
								{:else}
									{fact.value}
								{/if}
							</span>
							{#if !fact.sourceless}
								<span class="shrink-0"><AttributeSourceTag source={fact.source} /></span>
							{/if}
						</div>
					{/each}
				</div>
			{/each}
		</div>
	</div>
{/if}
