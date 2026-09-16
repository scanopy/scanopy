<script lang="ts">
	import type { HostFormData } from '$lib/features/hosts/types/base';
	import type { AttributeSource } from '$lib/shared/utils/attribute-source';
	import AttributeSourceTag from '$lib/shared/components/data/AttributeSourceTag.svelte';
	import {
		common_contact,
		common_firmwareRevision,
		common_hardware,
		common_hostname,
		common_location,
		common_manufacturer,
		common_model,
		common_serialNumber,
		common_softwareRevision,
		hosts_deviceFacts_firmwareGroup,
		hosts_deviceFacts_identityGroup,
		hosts_deviceFacts_locationGroup,
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
		mono?: boolean;
		link?: boolean;
	}

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
								{#if fact.link}
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
							<span class="shrink-0"><AttributeSourceTag source={fact.source} /></span>
						</div>
					{/each}
				</div>
			{/each}
		</div>
	</div>
{/if}
